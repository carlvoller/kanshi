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

    fn get_events_stream(&self) -> impl futures::Stream<Item = FileSystemEvent> + Send;

    fn start(&self) -> Result<(), FileSystemTracerError>;

    fn close(&self) -> bool;
}

#[cfg(test)]
mod tests {

    use std::{ffi::OsString, sync::Arc};

    use crate::{platforms::linux::fanotify::FanotifyTracer, FileSystemTracer, TracerOptions};
    use futures::{pin_mut, StreamExt};

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn main() {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        let fan = FanotifyTracer::new(TracerOptions { force_engine: None });
        if let Err(e) = fan {
            panic!("{e}");
        }

        let fanotify = Arc::new(fan.ok().unwrap());
        if let Err(e) = fanotify.watch("/home/carlvoller/someTest") {
            panic!("{e}");
        }

        let f = fanotify.clone();
        let handle1 = tokio::task::spawn(async move {
            let stream = f.get_events_stream();
            pin_mut!(stream);
            while let Some(event) = stream.next().await {
                let file = event
                    .target
                    .unwrap_or(OsString::new())
                    .into_string()
                    .ok()
                    .unwrap();
                let event_type = event.event_type;
                println!("FANOTIFY1 - Event happened: {file} - {:?}", event_type)
            }
        });

        let f = fanotify.clone();
        tokio::task::spawn(async move {
            f.start();
        });

        if let Err(e) = fanotify.watch("/home/carlvoller/fs-tracers") {
            panic!("{e}");
        }

        tokio::task::spawn(async move {
            println!("sleeping");
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;
            println!("running closed!");
            fanotify.close();
        });

        println!("waiting...");

        handle1.await;
        // handle2.await;

        println!("closed");
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;

    }
}

// #[cfg(test)]
// mod tests {
//     use std::ffi::OsString;

//     use futures::{pin_mut, StreamExt};
//     use tokio;

//     use crate::{
//         fanotify::{FanotifyTracer, FanotifyTracerOptions},
//         FileSystemTracer,
//     };

//     #[tokio::test]
//     async fn main() {
//         let opts = FanotifyTracerOptions {};
//         let listener = FanotifyTracer::new(opts);

//         if let Err(e) = listener {

//             panic!("{e}");
//         }

//         let listener = listener.ok().unwrap();

//         println!("A");
//         if let Err(e) = listener.watch("/home/carlvoller") {
//             panic!("{e}");
//         }

//         println!("B");
//         let stream = listener.stream_events();
//         println!("C");
//         pin_mut!(stream);
//         println!("D");
//         while let Some(event) = stream.next().await {
//             println!("here");
//             match event {
//                 Ok(ev) => {
//                     let file = ev
//                         .target
//                         .unwrap_or(OsString::new())
//                         .into_string()
//                         .ok()
//                         .unwrap();
//                     let event_type = ev.event_type;
//                     println!("Event happened: {file} - {:?}", event_type)
//                 }
//                 Err(e) => println!("{e}"),
//             }
//         }
//     }
// }
