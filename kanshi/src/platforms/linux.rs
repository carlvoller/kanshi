use crate::FileSystemTracer;

pub enum LinuxEngines {
  Fanotify,
  Inotify
}

pub mod fanotify;
pub mod inotify;
pub struct TracerOptions {
  pub force_engine: Option<LinuxEngines>
}

// pub fn trace(opts: TracerOptions) -> impl FileSystemTracer<TracerOptions> {




// }