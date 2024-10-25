use crate::{errors::FileSystemTracerError, events::Event};

use std::path::PathBuf;

use futures::Stream;

pub enum WatcherMode {
    INotify,
    Fanotify,
    SysCalls, // Experimental, just wanted to learn this myself
}
pub trait FileSystemWatcher: Sized + Stream {
    type Options;

    fn new(opts: Self::Options) -> Result<Self, FileSystemTracerError>;

    fn watch(&mut self, dir: PathBuf) -> Result<(), FileSystemTracerError>;

    fn unwatch(&mut self, dir: PathBuf) -> Result<(), FileSystemTracerError>;

    // fn into_stream(self) -> dyn FileSystemEventStream<Item = Event>;

    fn close(self) -> Result<(), FileSystemTracerError>;
}

// impl
