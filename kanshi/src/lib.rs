mod platforms;

pub use platforms::*;

use std::{ffi::OsString, io, pin::Pin};

use thiserror::Error;

#[cfg(unix)]
use nix::errno::Errno;

#[derive(Error, Debug, Clone)]
pub enum KanshiError {
    #[error("unable to attach ptrace to subprocess thread: {0}")]
    PTraceError(String),

    #[error("invalid command supplied: {0}")]
    InvalidCommand(String),

    #[error("file system error {0}")]
    FileSystemError(String),

    #[error("the file system listener was closed")]
    StreamClosedError,

    #[error("listener has already started")]
    ListenerStartedError,

    #[error("invalid parameter supplied: {0}")]
    InvalidParameter(String),
}

impl From<io::Error> for KanshiError {
    fn from(value: io::Error) -> Self {
        KanshiError::FileSystemError(value.to_string())
    }
}


#[cfg(unix)]
impl From<Errno> for KanshiError {
    fn from(value: Errno) -> Self {
        KanshiError::FileSystemError(value.to_string())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileSystemEventType {
    Create,
    Delete,
    Modify,
    Move,
    MovedTo(OsString),
    MovedFrom(OsString),
    Unknown,
}

impl ToString for FileSystemEventType {
    fn to_string(&self) -> String {
        match self {
            FileSystemEventType::MovedTo(_) => "moved_to",
            FileSystemEventType::MovedFrom(_) => "moved_from",
            FileSystemEventType::Create => "create",
            FileSystemEventType::Delete => "delete",
            FileSystemEventType::Modify => "modify",
            FileSystemEventType::Move => "move",
            FileSystemEventType::Unknown => "unknown",
        }
        .to_owned()
    }
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

#[derive(Clone, Debug)]
pub struct FileSystemEvent {
    pub event_type: FileSystemEventType,
    pub target: Option<FileSystemTarget>,
}

pub trait KanshiImpl<Opts>: Clone + Send + Sync {
    /// Creates a new Kanshi instance.
    /// Warning: This method blocks the thread until its finished!
    fn new(opts: Opts) -> Result<Self, KanshiError>
    where
        Self: Sized + Clone;

    /// Watches a new directory.
    /// Warning: This method blocks the thread until its finished!
    fn watch(&self, dir: &str) -> impl futures::Future<Output = Result<(), KanshiError>>;

    /// Get a new stream where events can be received.
    /// This method does not block and is safe to use in an async context.
    fn get_events_stream(&self) -> Pin<Box<dyn futures::Stream<Item = FileSystemEvent> + Send>>;

    /// Start listening for events. Kanshi will ignore all events until this method is run.
    /// Warning: This method blocks the thread until its finished!
    fn start(&self) -> impl futures::Future<Output = Result<(), KanshiError>>;

    fn close(&self) -> bool;
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {

    use futures::StreamExt;

    use crate::{Kanshi, KanshiImpl, KanshiOptions};

    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn main() {
        let kanshi = Kanshi::new(KanshiOptions { force_engine: None });
        if let Err(e) = kanshi {
            panic!("{e}");
        }

        let kanshi = kanshi.ok().unwrap();
        if let Err(e) = kanshi.watch(".").await {
            panic!("{e}");
        }

        let kan = kanshi.clone();
        tokio::task::spawn(async move {
            let mut stream = kan.get_events_stream();
            while let Some(event) = stream.next().await {
                let event_type = event.event_type;
                if let Some(target) = event.target {
                    println!("{:?} - {:?}", event_type, target.path)
                } else {
                    println!("{:?}", event_type)
                }
            }
        });

        let kan = kanshi.clone();
        tokio::task::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            kan.close();
        });

        tokio::task::spawn(async move { kanshi.start().await });

        // if let Err(e) = kanshi.start().await {
        //     panic!("{e}");
        // }

        println!("closed");
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {

    use crate::{Kanshi, KanshiImpl, KanshiOptions};
    use futures::StreamExt;

    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn main() {
        let kanshi = Kanshi::new(KanshiOptions { force_engine: None });
        if let Err(e) = kanshi {
            panic!("{e}");
        }

        let kanshi = kanshi.ok().unwrap();
        if let Err(e) = kanshi.watch("./why").await {
            panic!("{e}");
        }

        let kan = kanshi.clone();
        tokio::task::spawn(async move {
            let mut stream = kan.get_events_stream();
            while let Some(event) = stream.next().await {
                let event_type = event.event_type;
                if let Some(target) = event.target {
                    println!("{:?} - {:?}", event_type, target.path)
                } else {
                    println!("{:?}", event_type)
                }
            }
        });

        let kan = kanshi.clone();
        tokio::task::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            kan.close();
        });

        if let Err(e) = kanshi.start().await {
            panic!("{e}");
        }

        println!("closed");
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }
}
