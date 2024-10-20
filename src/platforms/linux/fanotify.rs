use std::{
    env,
    fs::{self, File},
    io::Error,
    os::{
        fd::{AsRawFd, FromRawFd, IntoRawFd},
        unix::ffi::OsStrExt,
    },
    path::PathBuf,
};

use libc::{
    __s32, __u16, __u32, __u64, __u8, c_uint, fanotify_init, fanotify_mark, AT_FDCWD, FAN_CLASS_CONTENT, FAN_CLASS_NOTIF, FAN_CLOEXEC, FAN_CREATE, FAN_DELETE, FAN_MARK_ADD, FAN_MODIFY, FAN_MOVE, FAN_MOVED_FROM, FAN_MOVE_SELF, FAN_RENAME, FAN_REPORT_FID, O_LARGEFILE, O_RDONLY, O_RDWR
};

struct FanotifyEventMetaData {
    pub event_len: __u32,
    pub vers: __u8,
    pub reserved: __u8,
    pub metadata_len: __u16,
    pub mask: __u64,
    pub fd: __s32,
    pub pid: __s32,
}

static WATCH_MASKS: u64 =
    FAN_MODIFY | FAN_CREATE | FAN_DELETE | FAN_MOVE | FAN_RENAME | FAN_MOVE_SELF;

pub struct Fanotify {
    fd: File,
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
                    fd: File::from_raw_fd(file_descriptor),
                }),
            }
        }
    }

    pub fn watch_directory(&self, dir_to_watch: PathBuf) -> Result<(), Error> {
        unsafe {
            // Calling native C API
            let fd = self.fd.as_raw_fd();
            let flags = FAN_MARK_ADD;
            let e_flags = WATCH_MASKS;
            let dirfd = AT_FDCWD;
            let mut path = dir_to_watch.as_os_str().as_bytes().to_vec();
            println!("{fd} {flags} {e_flags} {dirfd}");
            path.push(0u8);
            match fanotify_mark(
                self.fd.as_raw_fd(),
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

    pub fn close(self) {}
}
