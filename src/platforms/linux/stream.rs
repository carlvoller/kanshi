// use crate::opts::{FileSystemEventStream, FileSystemWatcher};

// struct EventStream {
//     watcher: dyn FileSystemWatcher,
// }

// impl FileSystemEventStream for EventStream {
//     fn watch(&self, dir: std::path::PathBuf) -> Result<(), crate::errors::FileSystemTracerError> {
//         Ok(())
//     }

//     fn unwatch(&self, dir: std::path::PathBuf) -> Result<(), crate::errors::FileSystemTracerError> {
//         Ok(())
//     }

//     fn close(self) -> Result<(), crate::errors::FileSystemTracerError> {
//         Ok(())
//     }
// }
