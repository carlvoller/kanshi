use crate::errors::FileSystemTracerError;

use std::path::PathBuf;

pub enum WatcherMode {
    INotify,
    Fanotify,
    SysCalls, // Experimental, just wanted to learn this myself
}

pub trait FileSystemWatcher: Sized {
    type Options;

    fn new(opts: Self::Options) -> Result<Self, FileSystemTracerError>;

    fn watch(&self, dir: PathBuf) -> Result<(), FileSystemTracerError>;

    fn unwatch(&self, dir: PathBuf) -> Result<(), FileSystemTracerError>;

    fn close(self) -> Result<(), FileSystemTracerError>;
}

// impl
