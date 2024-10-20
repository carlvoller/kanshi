mod platforms;

use std::fs;
use std::thread;

use nix::sys::personality;
use nix::sys::ptrace;
use platforms::linux::fanotify::Fanotify;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FileSystemTracerError {
    #[error("unable to attach ptrace to subprocess thread: {0}")]
    PTraceError(String),

    #[error("invalid command supplied: {0}")]
    InvalidCommand(String),
}

pub struct FileSystemTracer {
    command: Vec<String>,
    is_executing: bool,
}

impl FileSystemTracer {
    pub fn new(command: Vec<String>) -> FileSystemTracer {
        FileSystemTracer {
            command,
            is_executing: false,
        }
    }

    pub fn start(&mut self, command: &[String]) -> Result<u32, FileSystemTracerError> {
        self.is_executing = true;

        let cmd = command.to_owned();

        let handle = thread::spawn(move || -> Result<u32, FileSystemTracerError> {
            ptrace::traceme().map_err(|e| FileSystemTracerError::PTraceError(e.to_string()))?;
            personality::set(personality::Persona::ADDR_NO_RANDOMIZE)
                .map_err(|e| FileSystemTracerError::PTraceError(e.to_string()))?;

            let mut binary_to_execute = cmd
                .get(0)
                .ok_or(FileSystemTracerError::InvalidCommand(
                    "command is of length < 1".to_owned(),
                ))?
                .to_string();

            if let Ok(bin) = fs::canonicalize(&binary_to_execute) {
                binary_to_execute = bin
                    .to_str()
                    .ok_or(FileSystemTracerError::InvalidCommand(
                        "unable to find binary. is the binary in your path?".to_owned(),
                    ))?
                    .to_string()
            }

            // personality::
            Ok(0)
        });

        handle.join().unwrap()?;

        Ok(0)
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
