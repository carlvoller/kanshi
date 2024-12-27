mod platforms;

use std::{ffi::OsString, io};

use futures::Stream;
use nix::errno::Errno;
use thiserror::Error;

#[cfg(target_os = "linux")]
pub use platforms::linux::*;

#[derive(Error, Debug, Clone)]
pub enum FileSystemTracerError {
    #[error("unable to attach ptrace to subprocess thread: {0}")]
    PTraceError(String),

    #[error("invalid command supplied: {0}")]
    InvalidCommand(String),

    #[error("file system error {0}")]
    FileSystemError(String),

    #[error("the file system listener was closed")]
    StreamClosedError,
}

impl From<io::Error> for FileSystemTracerError {
    fn from(value: io::Error) -> Self {
        FileSystemTracerError::FileSystemError(value.to_string())
    }
}

impl From<Errno> for FileSystemTracerError {
    fn from(value: Errno) -> Self {
        FileSystemTracerError::FileSystemError(value.to_string())
    }
}

#[derive(Clone, Debug)]
pub enum FileSystemEventType {
    Create,
    Delete,
    Modify,
    Move,
    MovedTo(OsString),
    MovedFrom(OsString),
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FileSystemTargetKind {
    Directory,
    File,
}

#[derive(Clone, Debug)]
pub struct FileSystemTarget {
    pub kind: FileSystemTargetKind,
    pub path: OsString,
}

#[derive(Clone)]
pub struct FileSystemEvent {
    pub event_type: FileSystemEventType,
    pub target: Option<FileSystemTarget>,
}

pub trait FileSystemStream: Stream {}

pub trait FileSystemTracer<Opts> {
    fn new(opts: Opts) -> Result<impl FileSystemTracer<Opts>, FileSystemTracerError>;

    fn watch(&self, dir: &str) -> Result<(), FileSystemTracerError>;

    fn get_events_stream(&self) -> impl futures::Stream<Item = FileSystemEvent> + Send;

    fn start(&self) -> Result<(), FileSystemTracerError>;

    fn close(&self) -> bool;
}

#[cfg(test)]
mod tests {

    use std::{ffi::OsString, sync::Arc};

    use crate::{platforms::linux::fanotify::FanotifyTracer, FileSystemTracer, TracerOptions};
    use futures::{pin_mut, StreamExt};

    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn main() {
        let fan = FanotifyTracer::new(TracerOptions { force_engine: None });
        if let Err(e) = fan {
            panic!("{e}");
        }

        let fanotify = Arc::new(fan.ok().unwrap());
        if let Err(e) = fanotify.watch("./") {
            panic!("{e}");
        }

        let f = fanotify.clone();
        let handle1 = tokio::task::spawn(async move {
            let stream = f.get_events_stream();
            pin_mut!(stream);
            while let Some(event) = stream.next().await {
                // let event_type = event.event_type;
                // if let Some(target) = event.target {
                //     println!("{:?} - {:?}", event_type, target.path)
                // } else {
                //     println!("{:?}", event_type)
                // }
            }
        });

        let f = fanotify.clone();
        tokio::task::spawn_blocking(move || {
            if let Err(e) = f.start() {
                panic!("{e}")
            }
        });

        tokio::task::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(400)).await;
            fanotify.close();
        });

        handle1.await;
        // handle2.await;

        println!("closed");
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}