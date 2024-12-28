use std::ffi::{CStr, OsString};
use std::os::raw::c_void;
use std::path::{self, Path, PathBuf};
use std::sync::Arc;

use async_stream::stream;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::broadcast::Sender;
use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;

use super::core_foundation::types::{
    CFIndex, CFMutableArrayRef, FSEventStreamEventFlags, FSEventStreamRef,
};
use super::core_foundation::{self as CoreFoundation, dispatch_release, types as CFTypes};
use super::TracerOptions;
use crate::{
    FileSystemEvent, FileSystemEventType, FileSystemTarget, FileSystemTargetKind, FileSystemTracer,
    FileSystemTracerError,
};

pub struct FSEventsTracer {
    stream: Arc<RwLock<Option<WrappedEventStreamRef>>>,
    sender: tokio::sync::broadcast::Sender<FileSystemEvent>,
    cancellation_token: CancellationToken,
    paths_to_watch: Arc<Mutex<Vec<PathBuf>>>,
}

pub struct WrappedEventStreamRef(FSEventStreamRef);
unsafe impl Send for WrappedEventStreamRef {}
unsafe impl Sync for WrappedEventStreamRef {}

extern "C" fn callback(
    _stream_ref: *const CFTypes::FSEventStreamRef, // ConstFSEventStreamRef - Reference to the stream this event originated from
    info: CFTypes::CFRef, // *mut FSEventStreamContext->info - Optionally supplied context during stream creation.
    num_event: usize,     // numEvents - Number of total events in this callback
    event_paths: *const *const std::os::raw::c_char, // eventPaths - Array of C Strings representing the paths where each event occurred
    event_flags: *const CFTypes::FSEventStreamEventFlags, // eventFlags - Array of EventFlags corresponding to each event
    _event_ids: *const CFTypes::FSEventStreamId, // eventIds - Array of EventIds corresponding to each event. This Id is guaranteed to always be increasing.
) {
    let sender = info as *mut Sender<FileSystemEvent>;
    for idx in 0..num_event {
        let path = unsafe {
            CStr::from_ptr(*event_paths.add(idx))
                .to_str()
                .expect("Path was invalid UTF8 string")
        };
        let flag = unsafe { *event_flags.add(idx) };
        // let event_id = unsafe { *event_ids.add(idx) };

        let kind = if flag.contains(FSEventStreamEventFlags::kFSEventStreamEventFlagItemIsDir) {
            FileSystemTargetKind::Directory
        } else {
            FileSystemTargetKind::File
        };

        println!("flag {:?}", flag);

        let event_type = match flag {
            x if x.contains(FSEventStreamEventFlags::kFSEventStreamEventFlagItemCreated) => {
                if x.contains(FSEventStreamEventFlags::kFSEventStreamEventFlagItemRemoved) {
                    FileSystemEventType::Delete
                } else if x.contains(FSEventStreamEventFlags::kFSEventStreamEventFlagItemRenamed) {
                    FileSystemEventType::Move
                } else {
                    FileSystemEventType::Create
                }
            }
            x if x.contains(FSEventStreamEventFlags::kFSEventStreamEventFlagItemRemoved) => {
                FileSystemEventType::Delete
            }
            x if x.contains(FSEventStreamEventFlags::kFSEventStreamEventFlagItemModified) => {
                FileSystemEventType::Modify
            }
            x if x.contains(FSEventStreamEventFlags::kFSEventStreamEventFlagItemRenamed) => {
                FileSystemEventType::Move
            }
            x => {
                eprintln!("Unknown Mask Received - {:?}", x);
                FileSystemEventType::Unknown
            }
        };

        let event = FileSystemEvent {
            event_type,
            target: Some(FileSystemTarget {
                kind,
                path: OsString::from(path),
            }),
        };

        if let Err(e) = unsafe { (*sender).send(event) } {
            eprintln!("Send Error Occurred - {:?}", e.to_string());
        }
    }
}

impl FileSystemTracer<TracerOptions> for FSEventsTracer {
    fn new(
        _opts: TracerOptions,
    ) -> Result<impl FileSystemTracer<TracerOptions>, FileSystemTracerError> {
        let (tx, _rx) = tokio::sync::broadcast::channel(32);

        Ok(FSEventsTracer {
            stream: Arc::new(RwLock::new(None)),
            sender: tx,
            cancellation_token: CancellationToken::new(),
            paths_to_watch: Arc::new(Mutex::new(Vec::new())),
        })
    }

    async fn watch(&self, dir: &str) -> Result<(), FileSystemTracerError> {
        if let Some(_) = *self.stream.read().await {
            return Err(FileSystemTracerError::TracerStartedError);
        }

        let mut paths_to_watch = self.paths_to_watch.lock().await;
        let path = path::absolute(Path::new(dir));
        if let Ok(path) = path {
            if !path.exists() {
                Err(FileSystemTracerError::FileSystemError(
                    "ENOENT Directory does not exist".to_owned(),
                ))
            } else {
                paths_to_watch.push(path);
                Ok(())
            }
        } else {
            Err(FileSystemTracerError::FileSystemError(
                path.err().unwrap().to_string(),
            ))
        }
    }

    fn get_events_stream(&self) -> impl futures::Stream<Item = FileSystemEvent> + Send {
        let mut listener = self.sender.subscribe();
        let cancel_token = self.cancellation_token.clone();

        stream! {
            loop {
                tokio::select! {
                    _ = cancel_token.cancelled() => {
                        break;
                    }
                    val = listener.recv() => {
                        match val {
                            Ok(x) => yield x,
                            Err(e) => match e {
                                RecvError::Closed => break,
                                _ => ()
                            }
                        }
                    }
                }
            }
        }
    }

    async fn start(&self) -> Result<(), FileSystemTracerError> {
        let sender = self.sender.clone();
        let ptr: *const Sender<FileSystemEvent> = &sender;

        let context = CFTypes::FSEventStreamContext {
            version: 0 as *mut i64,
            copy_description: None,
            retain: None,
            release: None,
            info: ptr as *mut c_void,
        };

        let paths_to_watch = unsafe {
            let paths: CFMutableArrayRef = CoreFoundation::CFArrayCreateMutable(
                CFTypes::kCFAllocatorDefault,
                0 as CFIndex,
                &CoreFoundation::kCFTypeArrayCallBacks,
            );

            let paths_to_watch = self.paths_to_watch.lock().await;

            for path in paths_to_watch.iter() {
                if !path.exists() {
                    return Err(FileSystemTracerError::FileSystemError(format!(
                        "{:?} does not exist",
                        path
                    )));
                }

                let canon_path = path.canonicalize()?;
                let path_as_str = canon_path.to_str().unwrap();
                let err: CFTypes::CFErrorRef = std::ptr::null_mut();
                let cf_path = CoreFoundation::rust_str_to_cf_string(path_as_str, err);
                if cf_path.is_null() {
                    CoreFoundation::CFRelease(err as CFTypes::CFRef);
                    return Err(FileSystemTracerError::FileSystemError(format!(
                        "{:?} does not exist",
                        path
                    )));
                } else {
                    CoreFoundation::CFArrayAppendValue(paths, cf_path);
                    CoreFoundation::CFRelease(cf_path);
                }
            }

            Ok(paths)
        };

        if let Err(e) = paths_to_watch {
            return Err(e);
        }

        let paths_to_watch = paths_to_watch.ok().unwrap();

        let flags = CFTypes::FSEventStreamCreateFlags::kFSEventStreamCreateFlagFileEvents
            | CFTypes::FSEventStreamCreateFlags::kFSEventStreamCreateFlagNoDefer;

        let stream = unsafe {
            CoreFoundation::FSEventStreamCreate(
                CFTypes::kCFAllocatorDefault,
                callback,
                &context,
                paths_to_watch,
                CFTypes::kFSEventStreamEventIdSinceNow,
                0.0,
                flags,
            )
        };

        let dispatch_queue = unsafe {
            CoreFoundation::dispatch_queue_create(std::ptr::null(), CFTypes::DISPATCH_QUEUE_SERIAL)
        };

        unsafe { CoreFoundation::FSEventStreamSetDispatchQueue(stream, dispatch_queue) };
        unsafe { CoreFoundation::FSEventStreamStart(stream) };

        let mut stream_ref = self.stream.write().await;
        *stream_ref = Some(WrappedEventStreamRef(stream));
        drop(stream_ref);

        self.cancellation_token.cancelled().await;

        // Free the DispatchQueue
        unsafe { dispatch_release(dispatch_queue) };

        Ok(())
    }

    fn close(&self) -> bool {
        if self.cancellation_token.is_cancelled() {
            return true;
        }

        self.cancellation_token.cancel();

        let stream_ref = self.stream.try_read();
        if let Ok(stream) = stream_ref {
            if stream.is_some() {
                let stream = stream.as_ref().unwrap();
                unsafe {
                    CoreFoundation::FSEventStreamStop(stream.0);
                    CoreFoundation::FSEventStreamInvalidate(stream.0);
                    CoreFoundation::FSEventStreamRelease(stream.0);
                };
            }
        } else {
            let e = stream_ref.err().unwrap();
            eprintln!("error occurred releasing stream {e}");
            return false;
        }

        true
    }
}
