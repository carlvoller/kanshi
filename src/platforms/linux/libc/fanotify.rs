use super::fcntl::at_rawfd;
use crate::libc_bitflags;
use nix::errno::Errno;
use nix::sys::fanotify::FANOTIFY_METADATA_VERSION;
use nix::{NixPath, Result};
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd, RawFd};

// Not defined in libc yet
pub const FAN_EVENT_INFO_TYPE_FID: u8 = 1;
pub const FAN_EVENT_INFO_TYPE_DFID_NAME: u8 = 2;
pub const FAN_EVENT_INFO_TYPE_DFID: u8 = 3;
pub const FAN_EVENT_INFO_TYPE_PIDFD: u8 = 4;
pub const FAN_EVENT_INFO_TYPE_ERROR: u8 = 5;
pub const FAN_EVENT_INFO_TYPE_OLD_DFID_NAME: u8 = 10;
pub const FAN_EVENT_INFO_TYPE_NEW_DFID_NAME: u8 = 12;

#[derive(Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
#[allow(missing_copy_implementations)]
pub struct FanotifyEvent(libc::fanotify_event_metadata);

impl FanotifyEvent {
    /// Version number for the structure. It must be compared to
    /// `FANOTIFY_METADATA_VERSION` to verify compile version and runtime
    /// version does match. It can be done with the
    /// `FanotifyEvent::check_version` method.
    pub fn version(&self) -> u8 {
        self.0.vers
    }

    /// Checks that compile fanotify API version is equal to the version of the
    /// event.
    pub fn check_version(&self) -> bool {
        self.version() == FANOTIFY_METADATA_VERSION
    }

    /// Mask flags of the events.
    pub fn mask(&self) -> MaskFlags {
        MaskFlags::from_bits_truncate(self.0.mask)
    }

    /// The file descriptor of the event. If the value is `None` when reading
    /// from the fanotify group, this event is to notify that a group queue
    /// overflow occured.
    pub fn fd(&self) -> Option<BorrowedFd> {
        if self.0.fd == libc::FAN_NOFD {
            None
        } else {
            // SAFETY: self.0.fd will be opened for the lifetime of `Self`,
            // which is longer than the lifetime of the returned BorrowedFd, so
            // it is safe.
            Some(unsafe { BorrowedFd::borrow_raw(self.0.fd) })
        }
    }

    /// PID of the process that caused the event. TID in case flag
    /// `FAN_REPORT_TID` was set at group initialization.
    pub fn pid(&self) -> i32 {
        self.0.pid
    }
}

impl Drop for FanotifyEvent {
    fn drop(&mut self) {
        if self.0.fd == libc::FAN_NOFD {
            return;
        }
        let e = nix::unistd::close(self.0.fd);
        if !std::thread::panicking() && e == Err(Errno::EBADF) {
            panic!("Closing an invalid file descriptor!");
        };
    }
}

#[derive(Debug, Eq, Hash, PartialEq)]
#[allow(missing_copy_implementations)]
pub enum FanotifyInfoRecord {
    Fid(libc::fanotify_event_info_fid),
    Error(libc::fanotify_event_info_error),
    Pidfd(libc::fanotify_event_info_pidfd),
    Unimplemented(Vec<u8>),
}

#[derive(Debug, Eq, Hash, PartialEq)]
#[allow(missing_copy_implementations)]
pub struct FanotifyEventWithInfoRecords(libc::fanotify_event_metadata, Vec<FanotifyInfoRecord>);

impl FanotifyEventWithInfoRecords {
    /// Version number for the structure. It must be compared to
    /// `FANOTIFY_METADATA_VERSION` to verify compile version and runtime
    /// version does match. It can be done with the
    /// `FanotifyEvent::check_version` method.
    pub fn version(&self) -> u8 {
        self.0.vers
    }

    /// Checks that compile fanotify API version is equal to the version of the
    /// event.
    pub fn check_version(&self) -> bool {
        self.version() == FANOTIFY_METADATA_VERSION
    }

    /// Mask flags of the events.
    pub fn mask(&self) -> MaskFlags {
        MaskFlags::from_bits_truncate(self.0.mask)
    }

    /// The file descriptor of the event. If the value is `None` when reading
    /// from the fanotify group, this event is to notify that a group queue
    /// overflow occured.
    pub fn fd(&self) -> Option<BorrowedFd> {
        if self.0.fd == libc::FAN_NOFD {
            None
        } else {
            // SAFETY: self.0.fd will be opened for the lifetime of `Self`,
            // which is longer than the lifetime of the returned BorrowedFd, so
            // it is safe.
            Some(unsafe { BorrowedFd::borrow_raw(self.0.fd) })
        }
    }

    /// PID of the process that caused the event. TID in case flag
    /// `FAN_REPORT_TID` was set at group initialization.
    pub fn pid(&self) -> i32 {
        self.0.pid
    }

    pub fn get_events(&self) -> &[FanotifyInfoRecord] {
        &self.1
    }
}

pub struct Fanotify {
    fd: OwnedFd,
}

impl Fanotify {
    pub fn init(flags: InitFlags, event_f_flags: EventFFlags) -> Result<Fanotify> {
        let res = Errno::result(unsafe { libc::fanotify_init(flags.bits(), event_f_flags.bits()) });
        res.map(|fd| Fanotify {
            fd: unsafe { OwnedFd::from_raw_fd(fd) },
        })
    }

    pub fn mark<P: ?Sized + NixPath>(
        &self,
        flags: MarkFlags,
        mask: MaskFlags,
        dirfd: Option<RawFd>,
        path: Option<&P>,
    ) -> Result<()> {
        let res = path.unwrap().with_nix_path(|cstr| unsafe {
            libc::fanotify_mark(
                self.fd.as_raw_fd(),
                flags.bits(),
                mask.bits(),
                at_rawfd(dirfd),
                cstr.as_ptr(),
            )
        })?;

        Errno::result(res).map(|_| ())
    }

    pub fn read_events(&self) -> Result<Vec<FanotifyEvent>> {
        let metadata_size = size_of::<libc::fanotify_event_metadata>();
        const BUFSIZ: usize = 4096;
        let mut buffer = [0u8; BUFSIZ];
        let mut events = Vec::new();
        let mut offset = 0;

        let nread = nix::unistd::read(self.fd.as_raw_fd(), &mut buffer)?;

        while (nread - offset) >= metadata_size {
            let metadata = unsafe {
                let mut metadata = MaybeUninit::<libc::fanotify_event_metadata>::uninit();
                std::ptr::copy_nonoverlapping(
                    buffer.as_ptr().add(offset),
                    metadata.as_mut_ptr().cast(),
                    (BUFSIZ - offset).min(metadata_size),
                );
                metadata.assume_init()
            };

            events.push(FanotifyEvent(metadata));
            offset += metadata.event_len as usize;
        }

        Ok(events)
    }

    fn get_struct<T>(&self, buffer: &[u8; 4096], offset: usize) -> T {
        let struct_size = size_of::<T>();
        let struct_obj = unsafe {
            let mut struct_obj = MaybeUninit::<T>::uninit();
            std::ptr::copy_nonoverlapping(
                buffer.as_ptr().add(offset),
                struct_obj.as_mut_ptr().cast(),
                (4096 - offset).min(struct_size),
            );
            struct_obj.assume_init()
        };

        struct_obj
    }

    pub fn read_events_with_info_records(&self) -> Result<Vec<FanotifyEventWithInfoRecords>> {
        let metadata_size = size_of::<libc::fanotify_event_metadata>();
        const BUFSIZ: usize = 4096;
        let mut buffer = [0u8; BUFSIZ];
        let mut events = Vec::new();
        let mut offset = 0;

        let nread = nix::unistd::read(self.fd.as_raw_fd(), &mut buffer)?;

        while (nread - offset) >= metadata_size {
            let metadata = unsafe {
                let mut metadata = MaybeUninit::<libc::fanotify_event_metadata>::uninit();
                std::ptr::copy_nonoverlapping(
                    buffer.as_ptr().add(offset),
                    metadata.as_mut_ptr().cast(),
                    (BUFSIZ - offset).min(metadata_size),
                );
                metadata.assume_init()
            };

            let mut remaining_len = metadata.event_len;
            let mut info_records = Vec::new();
            let mut current_event_offset = offset + metadata_size;

            while remaining_len > 0 {
                let header = self
                    .get_struct::<libc::fanotify_event_info_header>(&buffer, current_event_offset);

                let info_record = match header.info_type {
                    FAN_EVENT_INFO_TYPE_FID => {
                        let event_fid = self.get_struct::<libc::fanotify_event_info_fid>(
                            &buffer,
                            current_event_offset,
                        );
                        FanotifyInfoRecord::Fid(event_fid)
                    }
                    FAN_EVENT_INFO_TYPE_ERROR => {
                        let error_fid = self.get_struct::<libc::fanotify_event_info_error>(
                            &buffer,
                            current_event_offset,
                        );
                        FanotifyInfoRecord::Error(error_fid)
                    }
                    FAN_EVENT_INFO_TYPE_PIDFD => {
                        let error_fid = self.get_struct::<libc::fanotify_event_info_pidfd>(
                            &buffer,
                            current_event_offset,
                        );
                        FanotifyInfoRecord::Pidfd(error_fid)
                    }
                    _ => {
                        let end_of_event = current_event_offset + header.len as usize;
                        FanotifyInfoRecord::Unimplemented(
                            buffer[current_event_offset..end_of_event].to_owned(),
                        )
                    }
                };

                info_records.push(info_record);

                remaining_len -= header.len as u32;
                current_event_offset += header.len as usize;
            }

            // libc::fanotify_event_info_header

            events.push(FanotifyEventWithInfoRecords(metadata, info_records));
            offset += metadata.event_len as usize;
        }

        Ok(events)
    }
}

//
// The flags below were taken from the nix project and modified to include
// new fanotify flags introduced in Kernel >=5.1
// nix-0.29.0/src/sys/fanotify.rs
//
libc_bitflags! {
    /// Configuration options for [`fanotify_mark`](fn.fanotify_mark.html).
    pub struct MarkFlags: libc::c_uint {
        /// Add the events to the marks.
        FAN_MARK_ADD;
        /// Remove the events to the marks.
        FAN_MARK_REMOVE;
        /// Don't follow symlinks, mark them.
        FAN_MARK_DONT_FOLLOW;
        /// Raise an error if filesystem to be marked is not a directory.
        FAN_MARK_ONLYDIR;
        /// Events added to or removed from the marks.
        FAN_MARK_IGNORED_MASK;
        /// Ignore mask shall survive modify events.
        FAN_MARK_IGNORED_SURV_MODIFY;
        /// Remove all marks.
        FAN_MARK_FLUSH;
        /// Do not pin inode object in the inode cache. Since Linux 5.19.
        FAN_MARK_EVICTABLE;
        /// Events added to or removed from the marks. Since Linux 6.0.
        FAN_MARK_IGNORE;

        /// Default flag.
        FAN_MARK_INODE;
        /// Mark the mount specified by pathname.
        FAN_MARK_MOUNT;
        /// Mark the filesystem specified by pathname. Since Linux 4.20.
        FAN_MARK_FILESYSTEM;

        /// Combination of `FAN_MARK_IGNORE` and `FAN_MARK_IGNORED_SURV_MODIFY`.
        FAN_MARK_IGNORE_SURV;
    }
}

libc_bitflags! {
    /// File status flags for fanotify events file descriptors.
    pub struct EventFFlags: libc::c_uint {
        /// Read only access.
        O_RDONLY as libc::c_uint;
        /// Write only access.
        O_WRONLY as libc::c_uint;
        /// Read and write access.
        O_RDWR as libc::c_uint;
        /// Support for files exceeded 2 GB.
        O_LARGEFILE as libc::c_uint;
        /// Close-on-exec flag for the file descriptor. Since Linux 3.18.
        O_CLOEXEC as libc::c_uint;
        /// Append mode for the file descriptor.
        O_APPEND as libc::c_uint;
        /// Synchronized I/O data integrity completion.
        O_DSYNC as libc::c_uint;
        /// No file last access time update.
        O_NOATIME as libc::c_uint;
        /// Nonblocking mode for the file descriptor.
        O_NONBLOCK as libc::c_uint;
        /// Synchronized I/O file integrity completion.
        O_SYNC as libc::c_uint;
    }
}

libc_bitflags! {
    /// Mask for defining which events shall be listened with
    /// [`fanotify_mark`](fn.fanotify_mark.html) and for querying notifications.
    pub struct MaskFlags: u64 {
        /// File was accessed.
        FAN_ACCESS;
        /// File was modified.
        FAN_MODIFY;
        /// Metadata has changed. Since Linux 5.1.
        FAN_ATTRIB;
        /// Writtable file was closed.
        FAN_CLOSE_WRITE;
        /// Unwrittable file was closed.
        FAN_CLOSE_NOWRITE;
        /// File was opened.
        FAN_OPEN;
        /// File was moved from X. Since Linux 5.1.
        FAN_MOVED_FROM;
        /// File was moved to Y. Since Linux 5.1.
        FAN_MOVED_TO;
        /// Subfile was created. Since Linux 5.1.
        FAN_CREATE;
        /// Subfile was deleted. Since Linux 5.1.
        FAN_DELETE;
        /// Self was deleted. Since Linux 5.1.
        FAN_DELETE_SELF;
        /// Self was moved. Since Linux 5.1.
        FAN_MOVE_SELF;
        /// File was opened for execution. Since Linux 5.0.
        FAN_OPEN_EXEC;

        /// Event queue overflowed.
        FAN_Q_OVERFLOW;
        /// Filesystem error. Since Linux 5.16.
        FAN_FS_ERROR;

        /// Permission to open file was requested.
        FAN_OPEN_PERM;
        /// Permission to access file was requested.
        FAN_ACCESS_PERM;
        /// Permission to open file for execution was requested. Since Linux
        /// 5.0.
        FAN_OPEN_EXEC_PERM;

        /// Interested in child events.
        FAN_EVENT_ON_CHILD;

        /// File was renamed. Since Linux 5.17.
        FAN_RENAME;

        /// Event occurred against dir.
        FAN_ONDIR;

        /// Combination of `FAN_CLOSE_WRITE` and `FAN_CLOSE_NOWRITE`.
        FAN_CLOSE;
        /// Combination of `FAN_MOVED_FROM` and `FAN_MOVED_TO`.
        FAN_MOVE;
    }
}

libc_bitflags! {
    /// Configuration options for [`fanotify_init`](fn.fanotify_init.html).
    pub struct InitFlags: libc::c_uint {
        /// Close-on-exec flag set on the file descriptor.
        FAN_CLOEXEC;
        /// Nonblocking flag set on the file descriptor.
        FAN_NONBLOCK;

        /// Receipt of events notifications.
        FAN_CLASS_NOTIF;
        /// Receipt of events for permission decisions, after they contain final
        /// data.
        FAN_CLASS_CONTENT;
        /// Receipt of events for permission decisions, before they contain
        /// final data.
        FAN_CLASS_PRE_CONTENT;

        /// Remove the limit on the number of events in the event queue.
        ///
        /// Prior to Linux kernel 5.13, this limit was hardcoded to 16384. After
        /// 5.13, one can change it via file `/proc/sys/fs/fanotify/max_queued_events`.
        ///
        /// See `fanotify(7)` for details about this limit. Use of this flag
        /// requires the `CAP_SYS_ADMIN` capability.
        FAN_UNLIMITED_QUEUE;
        /// Remove the limit on the number of fanotify marks per user.
        ///
        /// Prior to Linux kernel 5.13, this limit was hardcoded to 8192 (per
        /// group, not per user). After 5.13, one can change it via file
        /// `/proc/sys/fs/fanotify/max_user_marks`.
        ///
        /// See `fanotify(7)` for details about this limit. Use of this flag
        /// requires the `CAP_SYS_ADMIN` capability.
        FAN_UNLIMITED_MARKS;

        /// Make `FanotifyEvent::pid` return pidfd. Since Linux 5.15.
        FAN_REPORT_PIDFD;
        /// Make `FanotifyEvent::pid` return thread id. Since Linux 4.20.
        FAN_REPORT_TID;

        // TODO: Add description
        FAN_REPORT_FID;
    }
}
