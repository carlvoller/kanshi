use crate::FileSystemTracer;

pub enum DarwinEngines {
  FSEvents,
  // KQueue,
}

mod fsevents;
mod core_foundation;

pub struct TracerOptions {
  pub force_engine: DarwinEngines
}

pub use fsevents::FSEventsTracer;

pub struct Kanshi {}

impl Kanshi {
  pub fn new(opts: Option<TracerOptions>) -> Result<impl FileSystemTracer<TracerOptions> + Clone, crate::FileSystemTracerError> {

    let opts = if let Some(opts) = opts {
      opts
    } else {
      TracerOptions {
        force_engine: DarwinEngines::FSEvents
      }
    };
      
    // On MacOS, only use FSEvents for now
    Ok(fsevents::FSEventsTracer::new(opts)?)

  }
}