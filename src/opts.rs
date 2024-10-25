use crate::errors::FileSystemTracerError;

use std::path::PathBuf;

use futures_core::Stream;

pub enum WatcherMode {
    INotify,
    Fanotify,
    SysCalls, // Experimental, just wanted to learn this myself
}

pub trait FileSystemEventStream: Stream {
    fn watch(&self, dir: PathBuf) -> Result<(), FileSystemTracerError>;

    fn unwatch(&self, dir: PathBuf) -> Result<(), FileSystemTracerError>;

    fn close(self) -> Result<(), FileSystemTracerError>;
}

pub trait FileSystemWatcher: Sized {
    type Options;

    fn new(opts: Self::Options) -> Result<Self, FileSystemTracerError>;

    fn watch(&self, dir: PathBuf) -> Result<(), FileSystemTracerError>;

    fn unwatch(&self, dir: PathBuf) -> Result<(), FileSystemTracerError>;

    fn into_stream(self) -> FileSystemEventStream;

    fn close(self) -> Result<(), FileSystemTracerError>;
}

// impl
