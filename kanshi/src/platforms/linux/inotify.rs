use crate::FileSystemTracer;

use super::TracerOptions;

pub struct INotifyTracer {

}

impl FileSystemTracer<TracerOptions> for INotifyTracer {
  fn new(opts: TracerOptions) -> Result<impl FileSystemTracer<TracerOptions>, crate::FileSystemTracerError> {
      
  }

  fn watch(&self, dir: &str) -> Result<(), crate::FileSystemTracerError> {
      
  }

  fn get_events_stream(&self) -> impl futures::Stream<Item = crate::FileSystemEvent> + Send {
      
  }

  fn start(&self) -> Result<(), crate::FileSystemTracerError> {
      
  }

  fn close(&self) -> bool {
      
  }
}