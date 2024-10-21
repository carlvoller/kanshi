mod platforms;
mod opts;
mod errors;

use std::fs;
use std::thread;

use platforms::linux::fanotify::Fanotify;
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
    use core::time;
    use std::path::PathBuf;

    use platforms::linux::fanotify;

    use super::*;

    #[test]
    fn it_works() {
        let listener = match Fanotify::new() {
            Ok(listener) => listener,
            Err(e) => panic!("{e}"),
        };

        match listener.watch_directory(PathBuf::from("./test")) {
            Ok(_) => println!("It works!!!"),
            Err(e) => {
                panic!("{e}")
            }
        };

        match listener.read_event() {
            Some(event) => {
                for e in event.iter() {
                    let mask = e.mask;
                    println!("found event with mask {mask}")
                }
                println!("got here!")
            },
            None => println!("no events happened!")
        }

        listener.close();
    }
}
