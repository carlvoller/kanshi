use std::{borrow::Borrow, pin::Pin};

use crate::{KanshiError, KanshiImpl};

pub enum DarwinEngines {
    FSEvents,
    // KQueue,
}

mod core_foundation;
mod fsevents;

pub struct KanshiOptions {
    pub force_engine: Option<DarwinEngines>,
}

use async_stream::stream;
pub use fsevents::FSEventsTracer;

#[derive(Clone)]
enum Engines {
    FSEvents(FSEventsTracer),
}

#[derive(Clone)]
pub struct Kanshi {
    engine: Engines,
}

impl KanshiImpl<KanshiOptions> for Kanshi {
    fn new(opts: KanshiOptions) -> Result<Self, KanshiError>
    where
        Self: Sized + Clone,
    {
        Ok(Kanshi {
            engine: Engines::FSEvents(FSEventsTracer::new(opts)?),
        })
    }

    async fn start(&self) -> Result<(), KanshiError> {
        match self.engine.borrow() {
            Engines::FSEvents(fsevents) => fsevents.start().await,
        }
    }

    async fn watch(&self, dir: &str) -> Result<(), KanshiError> {
        match self.engine.borrow() {
            Engines::FSEvents(fsevents) => fsevents.watch(dir).await,
        }
    }

    fn get_events_stream(
        &self,
    ) -> Pin<Box<dyn futures::Stream<Item = crate::FileSystemEvent> + Send>> {
        let events_stream: Pin<Box<dyn futures::Stream<Item = crate::FileSystemEvent> + Send>>;

        match self.engine.borrow() {
            Engines::FSEvents(fsevents) => {
                events_stream = Box::pin(fsevents.get_events_stream());
            }
        };

        Box::pin(stream! {
          for await item in events_stream {
            yield item
          }
        })
    }

    fn close(&self) -> bool {
        match self.engine.borrow() {
            Engines::FSEvents(fsevents) => fsevents.close(),
        }
    }
}
