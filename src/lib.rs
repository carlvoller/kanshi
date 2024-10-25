mod platforms;
mod opts;
mod errors;
mod events;

use std::fs;
use std::thread;

// use platforms::linux::fanotify::Fanotify;
use errors::FileSystemTracerError;

pub struct FileSystemTracer {
    command: Vec<String>,
    is_executing: bool,
}

impl FileSystemTracer {
    pub fn new(command: Vec<String>, opts: opts::WatcherMode) -> FileSystemTracer {
        FileSystemTracer {
            command,
            is_executing: false,
        }
    }

    pub fn start(&mut self, command: &[String]) -> Result<u32, FileSystemTracerError> {
    }
}

#[cfg(test)]
mod tests {
    
}
