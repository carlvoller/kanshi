use std::{borrow::Borrow, pin::Pin};

use crate::{KanshiError, KanshiImpl};

#[derive(Clone)]
pub enum KanshiEngines {
    Fanotify,
    Inotify,
}

impl KanshiEngines {
    pub fn from(string: &str) -> Result<KanshiEngines, KanshiError> {
        match string {
            "fanotify" => Ok(KanshiEngines::Fanotify),
            "inotify" => Ok(KanshiEngines::Inotify),
            _ => Err(KanshiError::InvalidParameter(
                "Invalid engine. Allowed values are: 'fanotify', 'inotify'.".to_owned(),
            )),
        }
    }
}

mod fanotify;
mod inotify;

use async_stream::stream;
pub use fanotify::*;
pub use inotify::*;

pub struct KanshiOptions {
    pub force_engine: Option<KanshiEngines>,
}

#[derive(Clone)]
enum Engines {
    Fanotify(FanotifyTracer),
    INotify(INotifyTracer),
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
        let chosen_engine: KanshiEngines = if let Some(engine) = opts.force_engine.as_ref() {
            engine.clone()
        } else {
            let uid = unsafe { libc::geteuid() };

            if uid == 0 {
                KanshiEngines::Fanotify
            } else {
                KanshiEngines::Inotify
            }
        };

        Ok(Kanshi {
            engine: match chosen_engine {
                KanshiEngines::Inotify => Engines::INotify(INotifyTracer::new(opts)?),
                KanshiEngines::Fanotify => Engines::Fanotify(FanotifyTracer::new(opts)?),
            },
        })
    }

    async fn start(&self) -> Result<(), KanshiError> {
        match self.engine.borrow() {
            Engines::Fanotify(fan) => fan.start().await,
            Engines::INotify(notify) => notify.start().await,
        }
    }

    async fn watch(&self, dir: &str) -> Result<(), KanshiError> {
        match self.engine.borrow() {
            Engines::Fanotify(fan) => fan.watch(dir).await,
            Engines::INotify(notify) => notify.watch(dir).await,
        }
    }

    fn get_events_stream(
        &self,
    ) -> Pin<Box<dyn futures::Stream<Item = crate::FileSystemEvent> + Send>> {
        let events_stream: Pin<Box<dyn futures::Stream<Item = crate::FileSystemEvent> + Send>>;

        match self.engine.borrow() {
            Engines::Fanotify(fan) => {
                let stream = fan.get_events_stream();
                // pin_mut!(stream);
                events_stream = Box::pin(stream);
            }
            Engines::INotify(notify) => {
                let stream = notify.get_events_stream();
                // pin_mut!(stream);
                events_stream = Box::pin(stream);
            }
        };

        // let events_stream = *events_stream;

        Box::pin(stream! {
          for await item in events_stream {
            yield item
          }
        })
    }

    fn close(&self) -> bool {
        match self.engine.borrow() {
            Engines::Fanotify(fan) => fan.close(),
            Engines::INotify(notify) => notify.close(),
        }
    }
}
