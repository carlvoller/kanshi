mod structs;

use std::{
    collections::{HashSet, VecDeque},
    ffi::OsStr,
    io::Error,
    os::unix::ffi::OsStrExt,
    path::PathBuf,
    sync::{Arc, Mutex},
    task::Poll,
    thread::{self, JoinHandle, Thread},
};

use futures::Stream;

use crate::{errors::FileSystemTracerError, events::Event, opts::FileSystemWatcher};

use super::fd::Fd;

use structs::{FanotifyEventMetaData, FAN_EVENT_METADATA_LEN};

struct FanotifyOptions {}

pub struct FanotifyWatcher {
    fd: Arc<Mutex<Fd>>,
    watched_directories: HashSet<PathBuf>,
    event_queue: Arc<Mutex<VecDeque<FanotifyEventMetaData>>>,
    fd_reader_thread: JoinHandle<()>,
}

impl FileSystemWatcher for FanotifyWatcher {
    type Options = FanotifyOptions;

    fn new(_: Self::Options) -> Result<Self, FileSystemTracerError> {
        let flags = libc::FAN_CLOEXEC | libc::FAN_REPORT_FID;
        let event_flags = libc::O_RDONLY | libc::O_LARGEFILE;

        let event_queue = Arc::new(Mutex::new(VecDeque::new()));

        match unsafe { libc::fanotify_init(flags, event_flags as u32) } {
            -1 => Err(FileSystemTracerError::FileSystemError(
                Error::last_os_error(),
            )),
            fd => {
                let casted_fd: Fd = fd.into();
                let arc_fd = Arc::new(Mutex::new(casted_fd));

                let arc_fd_ref = arc_fd.clone();
                let event_queue_ref = event_queue.clone();
                let t = thread::spawn(move || {

                    loop {
                        let locked_fd = arc_fd_ref.lock().unwrap();
                        let fd = locked_fd.as_ref().to_owned();
                        drop(locked_fd); // Drop value to release lock back to main thread
    
                        let mut event_buffer = [0u8; FAN_EVENT_METADATA_LEN];
    
                        let size = unsafe {
                            libc::read(fd, event_buffer.as_mut_ptr() as _, FAN_EVENT_METADATA_LEN)
                        };

                        if size == -1 && Error::last_os_error().raw_os_error().unwrap() == libc::EAGAIN {
                            break
                        }
    
                        if size > 0 {
                            let event_ptr = event_buffer.as_ptr().cast::<FanotifyEventMetaData>();
    
                            let event = unsafe {
                                let event_ref = event_ptr.as_ref().unwrap().to_owned();
                                event_ref
                            };
    
                            let mut queue = event_queue_ref.lock().unwrap();
                            queue.push_back(event);
                        }
                    }
                });

                Ok(FanotifyWatcher {
                    fd: arc_fd,
                    watched_directories: HashSet::new(),
                    event_queue: event_queue.clone(),
                    fd_reader_thread: t,
                })
            }
        }
    }

    fn watch(&mut self, dir: PathBuf) -> Result<(), FileSystemTracerError> {
        let watch_masks: u64 = libc::FAN_MODIFY
            | libc::FAN_CREATE
            | libc::FAN_DELETE
            | libc::FAN_MOVE_SELF
            | libc::FAN_MOVE;

        let locked_fd = self.fd.lock().unwrap();
        let fd = locked_fd.as_ref().to_owned();
        drop(locked_fd);

        match unsafe {
            libc::fanotify_mark(
                fd,
                libc::FAN_MARK_ADD,
                watch_masks,
                libc::AT_FDCWD,
                dir.as_os_str().as_bytes().as_ptr().cast(),
            )
        } {
            0 => {
                self.watched_directories.insert(dir);
                Ok(())
            }
            _ => Err(FileSystemTracerError::FileSystemError(
                Error::last_os_error(),
            )),
        }
    }

    fn unwatch(&mut self, dir: PathBuf) -> Result<(), FileSystemTracerError> {
        let watch_masks: u64 = libc::FAN_MODIFY
            | libc::FAN_CREATE
            | libc::FAN_DELETE
            | libc::FAN_MOVE_SELF
            | libc::FAN_MOVE;

        let locked_fd = self.fd.lock().unwrap();
        let fd = locked_fd.as_ref().to_owned();
        drop(locked_fd);

        match unsafe {
            libc::fanotify_mark(
                fd,
                libc::FAN_MARK_REMOVE,
                watch_masks,
                libc::AT_FDCWD,
                dir.as_os_str().as_bytes().as_ptr().cast(),
            )
        } {
            0 => {
                self.watched_directories.remove(&dir);
                Ok(())
            }
            _ => Err(FileSystemTracerError::FileSystemError(
                Error::last_os_error(),
            )),
        }
    }

    fn close(self) -> Result<(), FileSystemTracerError> {
        let locked_fd = self.fd.lock().unwrap();
        let fd = locked_fd.as_ref().to_owned();
        drop(locked_fd);

        match unsafe {
            libc::fanotify_mark(
                fd,
                libc::FAN_MARK_FLUSH,
                0,
                libc::AT_FDCWD,
                OsStr::new("/").as_bytes().as_ptr().cast(),
            )
        } {
            0 => Ok(()),
            _ => Err(FileSystemTracerError::FileSystemError(
                Error::last_os_error(),
            )),
        }
    }
}

impl Stream for FanotifyWatcher {
    type Item = Event;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {

        let mut queue = self.event_queue.lock().unwrap();
        let last_event = queue.pop_front();

        match queue.pop_front() {
            None => Poll::Pending,
            Some(event) => {

                // let e = Event {
                //     target_item: event.
                // }

                let proc_path = format!("/proc/self/fd/{}", event.fd);

                Poll::Ready(None)

            }
        }
    }
}
