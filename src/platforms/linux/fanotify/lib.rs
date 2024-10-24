use std::{
  env,
  fs::{self, File},
  io::Error,
  mem,
  os::{
      fd::{AsRawFd, FromRawFd, IntoRawFd, RawFd}, raw::c_void, unix::ffi::OsStrExt
  },
  path::PathBuf,
  slice,
};

use libc::{
  __s32, __u16, __u32, __u64, __u8, c_uint, fanotify_init, fanotify_mark, AT_FDCWD, EAGAIN,
  FAN_CLASS_CONTENT, FAN_CLASS_NOTIF, FAN_CLOEXEC, FAN_CREATE, FAN_DELETE, FAN_MARK_ADD,
  FAN_MODIFY, FAN_MOVE, FAN_MOVED_FROM, FAN_MOVE_SELF, FAN_RENAME, FAN_REPORT_FID,
  FAN_REPORT_NAME, O_LARGEFILE, O_RDONLY, O_RDWR,
};

pub struct FanotifyEventMetaData {
  pub event_len: __u32,
  pub vers: __u8,
  pub reserved: __u8,
  pub metadata_len: __u16,
  pub mask: __u64,
  pub fd: __s32,
  pub pid: __s32,
}

static WATCH_MASKS: u64 = FAN_MODIFY | FAN_CREATE | FAN_DELETE | FAN_MOVE_SELF | FAN_MOVE;
static FAN_EVENT_METADATA_LEN: usize = mem::size_of::<FanotifyEventMetaData>();

pub struct Fanotify {
  fd: RawFd,
}

impl Fanotify {
  pub fn new() -> Result<Fanotify, Error> {
      unsafe {
          // Calling native C API
          match fanotify_init(
              FAN_CLOEXEC | FAN_REPORT_FID,
              (O_RDONLY | O_LARGEFILE) as u32,
          ) {
              -1 => Err(Error::last_os_error()),
              file_descriptor => Ok(Fanotify {
                  fd: file_descriptor,
              }),
          }
      }
  }

  pub fn watch_directory(&self, dir_to_watch: PathBuf) -> Result<(), Error> {
      let mut path = dir_to_watch.as_os_str().as_bytes().to_vec();
      path.push(0u8);

      unsafe {
          // Calling native C API
          match fanotify_mark(
              self.fd,
              FAN_MARK_ADD,
              WATCH_MASKS,
              AT_FDCWD,
              path.as_ptr().cast(),
          ) {
              0 => Ok(()),
              _ => Err(Error::last_os_error()),
          }
      }
  }

  // TODO: Map masks to actual FS events
  pub fn read_event(&self) -> Option<&[FanotifyEventMetaData]> {
      let mut event_buffer = vec![0u8; FAN_EVENT_METADATA_LEN * 200];
      unsafe {
          let data_len = libc::read(self.fd, event_buffer.as_mut_ptr() as *mut c_void, FAN_EVENT_METADATA_LEN * 200);
          if data_len != EAGAIN as isize && data_len > 0 {
              Some(slice::from_raw_parts(
                  event_buffer.as_ptr().cast::<FanotifyEventMetaData>(),
                  data_len as usize / FAN_EVENT_METADATA_LEN,
              ))
          } else {
              let err = Error::last_os_error();
              panic!("{err}");
              None
          }
      }
  }

  pub fn close(self) {
      unsafe {
          libc::close(self.fd.as_raw_fd());
      }
  }
}

impl Drop for Fanotify {
  fn drop(&mut self) {
      unsafe {
          libc::close(self.fd.as_raw_fd());
      }
  }
}
