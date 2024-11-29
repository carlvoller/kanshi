use async_stream::stream;
use futures::Stream;
// use nix::sys::fanotify;
use std::{
    ffi::{OsStr, OsString},
    os::fd::AsRawFd,
    sync::Arc,
};

use crate::{FileSystemEvent, FileSystemEventType, FileSystemTracer, FileSystemTracerError};
use tokio;
use super::libc::fanotify;

pub struct FanotifyTracer {
    fd: Arc<fanotify::Fanotify>,
    channel: tokio::sync::broadcast::Sender<Result<FileSystemEvent, FileSystemTracerError>>,
}

pub struct FanotifyTracerOptions {}

impl FileSystemTracer<FanotifyTracerOptions> for FanotifyTracer {
    fn new(
        _opts: FanotifyTracerOptions,
    ) -> Result<impl FileSystemTracer<FanotifyTracerOptions>, FileSystemTracerError> {
        let flags = fanotify::InitFlags::from_bits_retain(libc::FAN_REPORT_FID);
        let event_flags = fanotify::EventFFlags::O_RDWR | fanotify::EventFFlags::O_LARGEFILE;
        let fanotify_fd = match fanotify::Fanotify::init(flags, event_flags) {
            Ok(fd) => Ok(fd),
            Err(e) => Err(FileSystemTracerError::FileSystemError(e.to_string())),
        };

        if let Err(e) = fanotify_fd {
            return Err(e);
        }

        let fanotify_fd = Arc::new(fanotify_fd.ok().unwrap());
        let (tx, mut _rx) = tokio::sync::broadcast::channel(32);

        let task_fd = fanotify_fd.clone();
        let task_tx = tx.clone();
        tokio::task::spawn_blocking(move || 'taskLoop: loop {
            match task_fd.read_events_with_info_records() {
                Ok(events) => {
                    for event in events {
                        if let Err(_) = task_tx.send(Ok(fanotify_to_event(event))) {
                            break 'taskLoop;
                        }
                    }
                }
                Err(e) => {
                    if let Err(_) =
                        task_tx.send(Err(FileSystemTracerError::FileSystemError(e.to_string())))
                    {
                        break 'taskLoop;
                    }
                }
            }
        });

        Ok(FanotifyTracer {
            fd: fanotify_fd,
            channel: tx,
        })
    }

    fn watch(&self, dir: &str) -> Result<(), FileSystemTracerError> {
        let watch_masks = fanotify::MaskFlags::FAN_MODIFY
            | fanotify::MaskFlags::FAN_CREATE
            | fanotify::MaskFlags::FAN_DELETE
            | fanotify::MaskFlags::FAN_MOVE_SELF
            | fanotify::MaskFlags::FAN_MOVE;

        match self.fd.mark(
            fanotify::MarkFlags::FAN_MARK_ADD | fanotify::MarkFlags::FAN_MARK_FILESYSTEM,
            watch_masks,
            None,
            Some(dir),
        ) {
            Ok(_) => Ok(()),
            Err(e) => Err(FileSystemTracerError::FileSystemError(e.to_string())),
        }
    }

    fn stream_events(&self) -> impl Stream<Item = Result<FileSystemEvent, FileSystemTracerError>> {
        let mut listener = self.channel.subscribe();

        let s = stream! {
            loop {
                match listener.recv().await {
                    Ok(x) => yield x,
                    Err(_) => yield Err(FileSystemTracerError::StreamClosedError)
                }
            }
        };

        s
    }

    fn close(self) {}
}

fn fd_to_fullpath(fd: i32) -> Result<OsString, FileSystemTracerError> {
    let fd_path = format!("/proc/self/fd/{fd}");

    match nix::fcntl::readlink::<OsStr>(fd_path.as_ref()) {
        Ok(path) => Ok(path),
        Err(e) => Err(FileSystemTracerError::FileSystemError(e.to_string())),
    }
}

fn fanotify_to_event(ev: fanotify::FanotifyEventWithInfoRecords) -> FileSystemEvent {
    let target_fd = ev.fd();
    let d = target_fd.is_some();
    println!("{d}");
    // ev.
    let info_records = ev.get_events();

    for record in info_records {
        match record {
            &fanotify::FanotifyInfoRecord::Fid(record) => {
                record
            },
            _ => ()
        }
    }

    let name = match target_fd {
        Some(fd) => {
            let a = fd_to_fullpath(fd.as_raw_fd());
            if a.is_err() {
                let e = a.err().unwrap().to_string();
                println!("Error: {e}");
                None
            } else {
                let name = a.ok().unwrap();
                let string = name.to_str().unwrap();
                println!("{string}");
                Some(name)
            }
        }
        None => None,
    };

    let mask = ev.mask();

    let lib_event = match mask {
        x if x.contains(fanotify::MaskFlags::FAN_CREATE) => FileSystemEventType::Create,
        x if x.contains(fanotify::MaskFlags::FAN_DELETE) => FileSystemEventType::Delete,
        _ => FileSystemEventType::Unknown,
    };

    FileSystemEvent {
        target: name,
        event_type: lib_event,
    }
}
