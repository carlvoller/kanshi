

// mod readdirectorychangesw;

// pub use readdirectorychangesw::*;

use crate::KanshiError;

#[derive(Clone)]
pub enum KanshiEngines {
    ReadDirectoryChangesW
}

impl KanshiEngines {
    pub fn from(string: &str) -> Result<KanshiEngines, KanshiError> {
        match string {
            "readdirectorychangesw" => Ok(KanshiEngines::ReadDirectoryChangesW),
            _ => Err(KanshiError::InvalidParameter(
                "Invalid engine. Allowed values are: 'fanotify', 'inotify'.".to_owned(),
            )),
        }
    }
}

pub struct KanshiOptions {
  pub force_engine: Option<KanshiEngines>,
}

