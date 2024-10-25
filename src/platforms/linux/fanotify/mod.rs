use crate::opts::FileSystemWatcher;

struct FanotifyOptions {

}

pub struct FanotifyWatcher {

}   

impl FileSystemWatcher for FanotifyWatcher {
    type Options = FanotifyOptions;

    fn new(opts: Self::Options) -> Result<Self, crate::errors::FileSystemTracerError> {
        Ok(FanotifyWatcher {  })
    }

    fn watch(&self, dir: std::path::PathBuf) -> Result<(), crate::errors::FileSystemTracerError> {
        Ok(())
    }

    fn unwatch(&self, dir: std::path::PathBuf) -> Result<(), crate::errors::FileSystemTracerError> {
        Ok(())
    }

    fn into_stream(self) -> crate::opts::FileSystemEventStream {
        
    }

    fn close(self) -> Result<(), crate::errors::FileSystemTracerError> {
        Ok(())
    }
}