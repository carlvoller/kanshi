use std::pin::Pin;

use crate::{FileSystemEvent, KanshiError, KanshiImpl};

use super::KanshiOptions;


#[derive(Clone)]
pub struct ReadDirectoryChangesTracer {

}

impl KanshiImpl<KanshiOptions> for ReadDirectoryChangesTracer {
  fn new(opts: KanshiOptions) -> Result<Self, KanshiError>
      where
          Self: Sized + Clone {
      
  }

  async fn start(&self) -> Result<(), KanshiError> {
      
  }

  fn get_events_stream(&self) -> Pin<Box<dyn futures::Stream<Item = FileSystemEvent> + Send>> {
      
  }

  async fn watch(&self, dir: &str) -> Result<(), KanshiError> {
      
  }

  fn close(&self) -> bool {
      
  }
}