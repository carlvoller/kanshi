use std::{
    collections::{HashMap, HashSet, VecDeque},
    ffi::OsString,
    fs,
    os::{
        fd::{AsFd, AsRawFd},
        unix::fs::MetadataExt,
    },
    path::{self, Path, PathBuf},
    sync::Arc,
};

use async_stream::stream;
use futures::io;
use nix::sys::{
    epoll::Epoll,
    inotify::{Inotify, InotifyEvent, WatchDescriptor},
};
use tokio::sync::{broadcast::error::RecvError, Mutex};
use tokio_util::sync::CancellationToken;

use crate::{
    FileSystemEvent, FileSystemEventType, FileSystemTarget, FileSystemTargetKind, FileSystemTracer,
    FileSystemTracerError,
};

use super::TracerOptions;

pub struct INotifyTracer {
    inotify: Inotify,
    epoll: Epoll,
    sender: tokio::sync::broadcast::Sender<FileSystemEvent>,
    cancellation_token: CancellationToken,
    watch_descriptors: Arc<Mutex<HashMap<WatchDescriptor, PathBuf>>>,
}

impl FileSystemTracer<TracerOptions> for INotifyTracer {
    fn new(
        _opts: TracerOptions,
    ) -> Result<impl FileSystemTracer<TracerOptions>, crate::FileSystemTracerError> {
        use nix::sys::epoll::{EpollCreateFlags, EpollEvent, EpollFlags};
        use nix::sys::inotify::InitFlags;

        #[allow(non_snake_case)]
        let INIT_FLAGS = InitFlags::IN_CLOEXEC;

        let inotify_fd = Inotify::init(INIT_FLAGS);

        if let Ok(inotify) = inotify_fd {
            // Setup epoll
            let epoll_event =
                EpollEvent::new(EpollFlags::EPOLLIN, inotify.as_fd().as_raw_fd() as u64);

            let epoll_fd = Epoll::new(EpollCreateFlags::EPOLL_CLOEXEC);

            if let Ok(epoll) = epoll_fd {
                if let Err(e) = epoll.add(inotify.as_fd(), epoll_event) {
                    Err(FileSystemTracerError::FileSystemError(e.to_string()))
                } else {
                    let (tx, _rx) = tokio::sync::broadcast::channel(32);
                    Ok(INotifyTracer {
                        inotify,
                        epoll,
                        sender: tx,
                        cancellation_token: CancellationToken::new(),
                        watch_descriptors: Arc::new(Mutex::new(HashMap::new())),
                    })
                }
            } else {
                let e = epoll_fd.err().unwrap();
                Err(FileSystemTracerError::FileSystemError(e.to_string()))
            }
        } else {
            Err(FileSystemTracerError::FileSystemError(
                io::Error::last_os_error().to_string(),
            ))
        }
    }

    async fn watch(&self, dir: &str) -> Result<(), crate::FileSystemTracerError> {
        if self.cancellation_token.is_cancelled() {
            return Err(FileSystemTracerError::StreamClosedError);
        }

        let absolute_path = path::absolute(Path::new(dir))?;
        let mut watchers = self.watch_descriptors.lock().await;
        let mark_top_dir = mark(&self.inotify, &mut watchers, absolute_path.as_path());

        if let Ok(_) = mark_top_dir {
            let mut traversal_queue = VecDeque::from([absolute_path]);
            let mut visited = HashSet::<u64>::new();

            'outer: loop {
                if let Some(next_dir) = traversal_queue.pop_front() {
                    if let Ok(dir_items) = fs::read_dir(next_dir) {
                        for dir_item in dir_items {
                            if let Ok(dir_item_unwrapped) = dir_item {
                                if let Ok(metadata) = dir_item_unwrapped.metadata() {
                                    let inode_number = metadata.ino();
                                    if !visited.contains(&inode_number) && !metadata.is_symlink() {
                                        visited.insert(inode_number);
                                        if dir_item_unwrapped.path().is_dir() {
                                            if let Err(e) = mark(
                                                &self.inotify,
                                                &mut watchers,
                                                &dir_item_unwrapped.path(),
                                            ) {
                                                return Err(e);
                                            }
                                            traversal_queue.push_back(dir_item_unwrapped.path());
                                        }
                                    }
                                }
                            } else {
                                break 'outer;
                            }
                        }
                    } else {
                        break 'outer;
                    }
                } else {
                    break 'outer;
                }
            }

            Ok(())
        } else {
            mark_top_dir
        }
    }

    fn get_events_stream(&self) -> impl futures::Stream<Item = crate::FileSystemEvent> + Send {
        let mut listener = self.sender.subscribe();
        let cancel_token = self.cancellation_token.clone();

        stream! {
            loop {
                tokio::select! {
                    _ = cancel_token.cancelled() => {
                        break;
                    }
                    val = listener.recv() => {
                        match val {
                            Ok(x) => yield x,
                            Err(e) => match e {
                                RecvError::Closed => break,
                                _ => ()
                            }
                        }
                    }
                }
            }
        }
    }

    async fn start(&self) -> Result<(), crate::FileSystemTracerError> {
        use nix::sys::epoll::EpollEvent;

        let cancel_token = self.cancellation_token.clone();
        let sender = self.sender.clone();

        let mut events = [EpollEvent::empty(); 1];
        let mut cookie_map: HashMap<u32, InotifyEvent> = HashMap::new();
        // let mut cookie_map_old: HashMap<u32, InotifyEvent>;

        while !cancel_token.is_cancelled() {
            use nix::sys::inotify::AddWatchFlags;

            events.fill(EpollEvent::empty());
            let res = tokio::task::block_in_place(move || self.epoll.wait(&mut events, 16u8));

            if let Err(e) = res {
                println!("epoll failed {e}");
                res?;
            }

            if res.ok().unwrap() > 0 {
                // cookie_map_old = cookie_map;
                // cookie_map = HashMap::new();

                let all_records = self.inotify.read_events()?;
                for record in all_records {
                    let kind = if record.mask.contains(AddWatchFlags::IN_ISDIR) {
                        FileSystemTargetKind::Directory
                    } else {
                        FileSystemTargetKind::File
                    };

                    // Is a normal Inotify event
                    if record.cookie == 0 {
                        let mut wd = self.watch_descriptors.lock().await;
                        let path = wd.get(&record.wd);

                        if record.mask.contains(AddWatchFlags::IN_IGNORED) {
                            continue;
                        }

                        let event_type = match record.mask {
                            x if x.contains(AddWatchFlags::IN_CREATE) => {
                                FileSystemEventType::Create
                            }
                            x if x.contains(AddWatchFlags::IN_DELETE) => {
                                FileSystemEventType::Delete
                            }
                            x if x.contains(AddWatchFlags::IN_DELETE_SELF) => {
                                FileSystemEventType::Delete
                            }
                            x if x.contains(AddWatchFlags::IN_MODIFY) => {
                                FileSystemEventType::Modify
                            }
                            x if x.contains(AddWatchFlags::IN_ATTRIB) => {
                                FileSystemEventType::Modify
                            }
                            x => {
                                eprintln!("Unknown Mask Received - {:?}", x);
                                FileSystemEventType::Unknown
                            }
                        };

                        let mut full_path = OsString::new();
                        if let Some(path) = path {
                            full_path.push(path.as_os_str());
                        }

                        full_path.push("/");

                        if let Some(name) = record.name {
                            full_path.push(name);
                        }

                        if record.mask.contains(AddWatchFlags::IN_CREATE)
                            && kind == FileSystemTargetKind::Directory
                        {
                            let absolute_path = path::absolute(Path::new(&full_path))?;
                            mark(&self.inotify, &mut wd, absolute_path.as_path())?;
                        }

                        let tracer_event = FileSystemEvent {
                            event_type,
                            target: Some(FileSystemTarget {
                                kind,
                                path: full_path,
                            }),
                        };

                        if let Err(_) = sender.send(tracer_event) {
                            return Err(FileSystemTracerError::StreamClosedError);
                        }

                    // Is a MOVED_FROM or MOVED_TO event.
                    } else if cookie_map.get(&record.cookie).is_none() {
                        cookie_map.insert(record.cookie, record);
                    } else {
                        let other_record = cookie_map.remove(&record.cookie).unwrap();
                        let mut wd = self.watch_descriptors.lock().await;
                        let moved_from;
                        let moved_to;

                        let path = wd.get(&other_record.wd);
                        let mut other_full_path = OsString::new();
                        if let Some(path) = path {
                            other_full_path.push(path.as_os_str());
                        }

                        other_full_path.push("/");

                        if let Some(name) = &other_record.name {
                            other_full_path.push(name);
                        }

                        let path = wd.get(&record.wd);
                        let mut full_path = OsString::new();
                        if let Some(path) = path {
                            full_path.push(path.as_os_str());
                        }

                        full_path.push("/");

                        if let Some(name) = &record.name {
                            full_path.push(name);
                        }

                        if other_record.mask.contains(AddWatchFlags::IN_MOVED_FROM) {
                            moved_from = Some(other_full_path);
                            moved_to = Some(full_path);
                        } else {
                            moved_from = Some(full_path);
                            moved_to = Some(other_full_path);
                        }

                        if kind == FileSystemTargetKind::Directory {
                            let moved_from_as_path_buf =
                                PathBuf::from(moved_from.as_ref().unwrap());
                            let moved_to_as_path_buf = PathBuf::from(moved_to.as_ref().unwrap());
                            for dir_path in wd.values_mut() {
                                if dir_path.starts_with(&moved_from_as_path_buf) {
                                    let relative_path =
                                        dir_path.strip_prefix(&moved_from_as_path_buf);
                                    if let Ok(relative_path) = relative_path {
                                        let final_path =
                                            moved_to_as_path_buf.join(relative_path).canonicalize();
                                        if let Ok(final_path) = final_path {
                                            *dir_path = final_path;
                                        }
                                    }
                                }
                            }
                        }

                        let tracer_event1 = FileSystemEvent {
                            event_type: FileSystemEventType::MovedTo(moved_to.clone().unwrap()),
                            target: Some(FileSystemTarget {
                                path: moved_from.clone().unwrap(),
                                kind: kind.clone(),
                            }),
                        };

                        let tracer_event2 = FileSystemEvent {
                            event_type: FileSystemEventType::MovedFrom(moved_from.unwrap()),
                            target: Some(FileSystemTarget {
                                path: moved_to.clone().unwrap(),
                                kind,
                            }),
                        };

                        if let Err(_) = sender.send(tracer_event1) {
                            return Err(FileSystemTracerError::StreamClosedError);
                        }

                        if let Err(_) = sender.send(tracer_event2) {
                            return Err(FileSystemTracerError::StreamClosedError);
                        }
                    }
                }
            } else if !cookie_map.is_empty() {
                // Assume all unfulfilled cookies as moves outside of watched directory.
                for (_, record) in cookie_map.iter() {
                    let mut wd = self.watch_descriptors.lock().await;
                    let kind = if record.mask.contains(AddWatchFlags::IN_ISDIR) {
                        FileSystemTargetKind::Directory
                    } else {
                        FileSystemTargetKind::File
                    };
                    let path = wd.get(&record.wd);
                    let mut full_path = OsString::new();
                    if let Some(path) = path {
                        full_path.push(path.as_os_str());
                    }

                    full_path.push("/");

                    if let Some(name) = &record.name {
                        full_path.push(name);
                    }

                    let path_as_path_buf = PathBuf::from(full_path.clone());

                    if let Some(_) = wd
                        .values()
                        .find(|x| x.as_path() == path_as_path_buf.as_path())
                    {
                        wd.retain(|curr_wd, path| {
                            if path.starts_with(&path_as_path_buf) {
                                let _ = unmark(&self.inotify, curr_wd);
                            }
                            !path.starts_with(&path_as_path_buf)
                        });
                        drop(wd);
                    } else {
                        drop(wd);
                        self.watch(path_as_path_buf.clone().to_str().unwrap())
                            .await?;
                    }

                    // if wd.contains_key(&record.wd) {
                    //     wd.remove(&record.wd);
                    // } else {
                    //     wd.insert(record.wd, PathBuf::from(full_path.clone()));
                    // }

                    let tracer_event = FileSystemEvent {
                        event_type: FileSystemEventType::Move,
                        target: Some(FileSystemTarget {
                            path: full_path,
                            kind,
                        }),
                    };

                    if let Err(_) = sender.send(tracer_event) {
                        return Err(FileSystemTracerError::StreamClosedError);
                    }
                }
                cookie_map.clear();
            }
        }

        Ok(())
    }

    fn close(&self) -> bool {
        if self.cancellation_token.is_cancelled() {
            return true;
        }

        self.cancellation_token.cancel();

        let mut has_error = false;

        if self.epoll.delete(self.inotify.as_fd()).is_err() {
            eprintln!("epoll.delete returned error");
            has_error = true;
        }

        // Inotify is automatically closed on drop.

        !has_error
    }
}

fn mark(
    inotify: &Inotify,
    watchers: &mut HashMap<WatchDescriptor, PathBuf>,
    path: &Path,
) -> Result<(), FileSystemTracerError> {
    use nix::sys::inotify::AddWatchFlags;
    #[allow(non_snake_case)]
    let MASK_FLAGS = AddWatchFlags::IN_CREATE
        | AddWatchFlags::IN_MODIFY
        | AddWatchFlags::IN_MOVE
        | AddWatchFlags::IN_DELETE;

    let wd = inotify.add_watch(path, MASK_FLAGS);
    if let Err(e) = wd {
        Err(FileSystemTracerError::FileSystemError(e.to_string()))
    } else {
        let wd = wd.ok().unwrap();
        watchers.insert(wd, path.to_path_buf());
        Ok(())
    }
}

fn unmark(inotify: &Inotify, wd: &WatchDescriptor) -> Result<(), FileSystemTracerError> {
    inotify.rm_watch(*wd)?;
    Ok(())
}
