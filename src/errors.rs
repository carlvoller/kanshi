use std::io;

use thiserror::Error;


#[derive(Error, Debug)]
pub enum FileSystemTracerError {
    #[error("unable to attach ptrace to subprocess thread: {0}")]
    PTraceError(String),

    #[error("invalid command supplied: {0}")]
    InvalidCommand(String),

    #[error("file system error")]
    FileSystemError(#[from] io::Error)
}