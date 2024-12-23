use std::{
    collections::{HashSet, VecDeque},
    ffi::{CStr, OsStr, OsString},
    fs, io,
    os::{
        fd::{AsFd, AsRawFd},
        unix::fs::MetadataExt,
    },
    path::{Path, PathBuf},
};

use async_stream::stream;
use nix::{
    fcntl::AT_FDCWD,
    sys::{
        epoll::Epoll,
        fanotify::{Fanotify, FanotifyInfoRecord},
    },
};
use tokio::sync::broadcast::error::TryRecvError;
use tokio_util::sync::CancellationToken;

use crate::{FileSystemEvent, FileSystemEventType, FileSystemTracer, FileSystemTracerError};

use super::TracerOptions;

pub struct FanotifyTracer {
    // mark_set: HashSet<i32>,
    fanotify: Fanotify,
    epoll: Epoll,
    sender: tokio::sync::broadcast::Sender<FileSystemEvent>,
    // reciever: tokio::sync::broadcast::Receiver<FileSystemEvent>,
    cancellation_token: CancellationToken,
}

#[repr(C)]
pub struct FileHandle {
    pub handle_bytes: u32,
    pub handle_type: i32,
    pub f_handle: [u8; 0],
}

impl FileSystemTracer<TracerOptions> for FanotifyTracer {
    fn new(
        _opts: TracerOptions,
    ) -> Result<impl FileSystemTracer<TracerOptions>, FileSystemTracerError> {
        use nix::sys::epoll::{EpollCreateFlags, EpollEvent, EpollFlags};
        use nix::sys::fanotify::{EventFFlags, InitFlags};

        #[allow(non_snake_case)]
        let INIT_FLAGS: InitFlags = InitFlags::FAN_CLASS_NOTIF
            | InitFlags::FAN_REPORT_DFID_NAME
            | InitFlags::FAN_UNLIMITED_QUEUE
            | InitFlags::FAN_UNLIMITED_MARKS;
        #[allow(non_snake_case)]
        let EVENT_FLAGS: EventFFlags =
            EventFFlags::O_RDONLY | EventFFlags::O_NONBLOCK | EventFFlags::O_CLOEXEC;

        let fanotify_fd = Fanotify::init(INIT_FLAGS, EVENT_FLAGS);

        if let Ok(fanotify) = fanotify_fd {
            // Setup epoll
            let epoll_event =
                EpollEvent::new(EpollFlags::EPOLLIN, fanotify.as_fd().as_raw_fd() as u64);

            let epoll_fd = Epoll::new(EpollCreateFlags::EPOLL_CLOEXEC);

            if let Ok(epoll) = epoll_fd {
                if let Err(e) = epoll.add(fanotify.as_fd(), epoll_event) {
                    Err(FileSystemTracerError::FileSystemError(e.to_string()))
                } else {
                    let (tx, _rx) = tokio::sync::broadcast::channel(32);
                    Ok(FanotifyTracer {
                        // mark_set: HashSet::new(),
                        fanotify,
                        epoll,
                        sender: tx,
                        // reciever: rx,
                        cancellation_token: CancellationToken::new(),
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

    fn watch(&self, dir: &str) -> Result<(), FileSystemTracerError> {
        if self.cancellation_token.is_cancelled() {
            return Err(FileSystemTracerError::StreamClosedError);
        }

        let mark_top_dir = mark(&self.fanotify, Path::new(dir));

        if let Ok(_) = mark_top_dir {
            let mut traversal_queue = VecDeque::from([PathBuf::from(dir)]);
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
                                        if let Err(e) =
                                            mark(&self.fanotify, &dir_item_unwrapped.path())
                                        {
                                            return Err(e);
                                        }
                                        if dir_item_unwrapped.path().is_dir() {
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

            println!("{:?}", visited.len());

            Ok(())
        } else {
            mark_top_dir
        }
    }

    fn get_events_stream(&self) -> impl futures::Stream<Item = FileSystemEvent> + Send {
        let mut listener = self.sender.subscribe();

        stream! {
            loop {
                if !self.cancellation_token.is_cancelled() {
                  match listener.try_recv() {
                    Ok(x) => yield x,
                    Err(e) => match e {
                      TryRecvError::Closed => break,
                      _ => ()
                    }
                  }
                } else {
                  break
                }
            }
        }
    }

    fn start(&self) -> Result<(), FileSystemTracerError> {
        while !self.cancellation_token.is_cancelled() {
            use nix::sys::epoll::EpollEvent;
            use nix::sys::fanotify::MaskFlags;

            let mut events = [EpollEvent::empty()];
            let res = self.epoll.wait(&mut events, 16u8)?;
            if res > 0 {
                let all_records = self.fanotify.read_events_with_info_records()?;
                for (event, records) in all_records {
                    let mut tracer_event = FileSystemEvent {
                        event_type: match event.mask() {
                            x if x.contains(MaskFlags::FAN_CREATE) => FileSystemEventType::Create,
                            x if x.contains(MaskFlags::FAN_DELETE_SELF) => FileSystemEventType::Delete,
                            x if x.contains(MaskFlags::FAN_DELETE) => FileSystemEventType::Delete,
                            x if x.contains(MaskFlags::FAN_MODIFY) => FileSystemEventType::Modify,
                            x if x.contains(MaskFlags::FAN_MOVE) => FileSystemEventType::Move,
                            x if x.contains(MaskFlags::FAN_MOVE_SELF) => FileSystemEventType::Move,
                            _ => FileSystemEventType::Unknown,
                        },
                        target: None,
                    };

                    let mut path = OsString::new();

                    for record in records {
                        if let FanotifyInfoRecord::Fid(record) = record {
                            let fh = record.handle() as *mut FileHandle;
                            let fd = unsafe {
                                libc::syscall(
                                    libc::SYS_open_by_handle_at,
                                    AT_FDCWD,
                                    fh,
                                    libc::O_RDONLY
                                        | libc::O_CLOEXEC
                                        | libc::O_PATH
                                        | libc::O_NONBLOCK,
                                )
                            };

                            if fd > 0 {
                                let fd_path = format!("/proc/self/fd/{fd}");
                                path.push(nix::fcntl::readlink::<OsStr>(fd_path.as_ref())?);
                                unsafe { libc::close(fd as i32) };
                            }

                            let file_name: *const libc::c_char = unsafe {
                                (fh.add(1) as *const libc::c_char)
                                    .add(size_of_val(&(*fh).f_handle))
                                    .add(size_of_val(&(*fh).handle_bytes))
                                    .add(size_of_val(&(*fh).handle_type))
                                    .add(4) // no idea why i need to add 4 here tbh
                            };

                            if !file_name.is_null()
                                && unsafe {
                                    libc::strcmp(
                                        file_name,
                                        b".\0".as_ptr() as *const libc::c_char,
                                    ) != 0
                                }
                            {
                                let file_name_as_cstr =
                                    unsafe { CStr::from_ptr(file_name).to_str() };
                                if let Ok(name) = file_name_as_cstr {
                                    path.push("/");
                                    path.push(name);
                                }
                            }


                            // break;
                        }
                    }
                    if path.len() > 0 {
                        tracer_event.target = Some(path);
                    }
                    self.sender.send(tracer_event);
                }
            }
        }

        Ok(())
    }

    fn close(&self) -> bool {
        use nix::sys::fanotify::{MarkFlags, MaskFlags};

        if self.cancellation_token.is_cancelled() {
            return true;
        }

        self.cancellation_token.cancel();

        #[allow(non_snake_case)]
        let MARK_FLAGS = MarkFlags::FAN_MARK_FLUSH;

        let mut has_error = false;

        if self.epoll.delete(self.fanotify.as_fd()).is_err() {
            println!("epoll.delete returned error");
            has_error = true;
        }
        if self
            .fanotify
            .mark(MARK_FLAGS, MaskFlags::empty(), AT_FDCWD, Some("/"))
            .is_err()
        {
            println!("fanotify.mark returned error");
            has_error = true;
        }
        has_error
    }
}

impl Drop for FanotifyTracer {
    fn drop(&mut self) {
        println!("dropped!");
    }
}

fn mark(fanotify: &Fanotify, path: &Path) -> Result<(), FileSystemTracerError> {
    use nix::sys::fanotify::{MarkFlags, MaskFlags};
    #[allow(non_snake_case)]
    let MARK_FLAGS = MarkFlags::FAN_MARK_ADD;
    #[allow(non_snake_case)]
    let MASK_FLAGS = MaskFlags::FAN_ONDIR
        | MaskFlags::FAN_CREATE
        | MaskFlags::FAN_MODIFY
        | MaskFlags::FAN_DELETE
        | MaskFlags::FAN_MOVE
        | MaskFlags::FAN_DELETE_SELF
        | MaskFlags::FAN_MOVE_SELF;

    if let Err(e) = fanotify.mark(MARK_FLAGS, MASK_FLAGS, AT_FDCWD, Some(path)) {
        println!("{:?}", path);
        Err(FileSystemTracerError::FileSystemError(e.to_string()))
    } else {
        Ok(())
    }
}
