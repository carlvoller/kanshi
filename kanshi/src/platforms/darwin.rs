use crate::FileSystemTracer;

pub enum DarwinEngines {
  FSEvents,
  // KQueue,
}

pub mod fsevents;
pub mod core_foundation;

pub struct TracerOptions {
  pub force_engine: Option<DarwinEngines>
}