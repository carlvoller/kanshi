mod platforms;

use std::ffi::OsString;

use futures::Stream;
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

#[derive(Clone, Debug)]
pub enum FileSystemEventType {
    Create,
    Delete,
    Unknown,
}

#[derive(Clone)]
pub struct FileSystemEvent {
    pub event_type: FileSystemEventType,
    pub target: Option<OsString>,
}

pub trait FileSystemStream: Stream {}

pub trait FileSystemTracer<Opts> {
    fn new(opts: Opts) -> Result<impl FileSystemTracer<Opts>, FileSystemTracerError>;

    fn watch(&self, dir: &str) -> Result<(), FileSystemTracerError>;

    fn stream_events(&self) -> impl Stream<Item = Result<FileSystemEvent, FileSystemTracerError>>;

    fn close(self);
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use futures::{pin_mut, StreamExt};
    use tokio;

    use crate::{
        fanotify::{FanotifyTracer, FanotifyTracerOptions},
        FileSystemTracer,
    };

    #[tokio::test]
    async fn main() {
        let opts = FanotifyTracerOptions {};
        let listener = FanotifyTracer::new(opts);

        if let Err(e) = listener {

            panic!("{e}");
        }

        let listener = listener.ok().unwrap();

        println!("A");
        if let Err(e) = listener.watch("/") {
            panic!("{e}");
        }

        println!("B");
        let stream = listener.stream_events();
        println!("C");
        pin_mut!(stream);
        println!("D");
        while let Some(event) = stream.next().await {
            println!("here");
            match event {
                Ok(ev) => {
                    let file = ev
                        .target
                        .unwrap_or(OsString::new())
                        .into_string()
                        .ok()
                        .unwrap();
                    let event_type = ev.event_type;
                    println!("Event happened: {file} - {:?}", event_type)
                }
                Err(e) => println!("{e}"),
            }
        }
    }
}
