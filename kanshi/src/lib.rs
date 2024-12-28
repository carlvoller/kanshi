mod platforms;

use std::{ffi::OsString, io};

use futures::Stream;
use nix::errno::Errno;
use thiserror::Error;

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

    #[error("listener has already started")]
    TracerStartedError,
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

pub trait FileSystemTracer<Opts> {
    /// Creates a new FileSystemTracer instance.
    /// Warning: This method blocks the thread until its finished!
    fn new(opts: Opts) -> Result<impl FileSystemTracer<Opts> + Clone, FileSystemTracerError>;

    /// Watches a new directory.
    /// Warning: This method blocks the thread until its finished!
    fn watch(&self, dir: &str) -> impl futures::Future<Output = Result<(), FileSystemTracerError>>;

    /// Get a new stream where events can be received.
    /// This method does not block and is safe to use in an async context.
    fn get_events_stream(&self) -> impl futures::Stream<Item = FileSystemEvent> + Send;

    /// Start listening for events. The FileSystemTracer will ignore all events until
    /// this method is run.
    /// Warning: This method blocks the thread until its finished!
    fn start(&self) -> impl futures::Future<Output = Result<(), FileSystemTracerError>>;

    fn close(&self) -> bool;
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {

    use std::sync::Arc;

    use futures::{pin_mut, StreamExt};

    use crate::{
        platforms::darwin::{fsevents::FSEventsTracer, TracerOptions},
        FileSystemTracer,
    };

    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn main() {
        let fan = FSEventsTracer::new(TracerOptions { force_engine: None });
        if let Err(e) = fan {
            panic!("{e}");
        }

        let fanotify = Arc::new(fan.ok().unwrap());
        if let Err(e) = fanotify.watch("./why").await {
            panic!("{e}");
        }

        let f = fanotify.clone();
        tokio::task::spawn(async move {
            let stream = f.get_events_stream();
            pin_mut!(stream);
            while let Some(event) = stream.next().await {
                let event_type = event.event_type;
                if let Some(target) = event.target {
                    println!("{:?} - {:?}", event_type, target.path)
                } else {
                    println!("{:?}", event_type)
                }
            }
        });

        let f = fanotify.clone();
        tokio::task::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(400)).await;
            f.close();
        });

        if let Err(e) = fanotify.start().await {
            panic!("{e}");
        }

        println!("closed");
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {

    use crate::{
        platforms::linux::{fanotify::FanotifyTracer, TracerOptions},
        FileSystemTracer,
    };
    use futures::{pin_mut, StreamExt};
    // use nix::sys::fanotify;

    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn main() {
        let fan = FanotifyTracer::new(TracerOptions { force_engine: None });
        if let Err(e) = fan {
            panic!("{e}");
        }

        let fanotify = fan.ok().unwrap();
        if let Err(e) = fanotify.watch("./why").await {
            panic!("{e}");
        }

        let f = fanotify.clone();
        tokio::task::spawn(async move {
            let stream = f.get_events_stream();
            pin_mut!(stream);
            while let Some(event) = stream.next().await {
                let event_type = event.event_type;
                if let Some(target) = event.target {
                    println!("{:?} - {:?}", event_type, target.path)
                } else {
                    println!("{:?}", event_type)
                }
            }
        });

        let f = fanotify.clone();
        tokio::task::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(400)).await;
            f.close();
        });

        if let Err(e) = fanotify.start().await {
            panic!("{e}");
        }

        println!("closed");
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }
}
