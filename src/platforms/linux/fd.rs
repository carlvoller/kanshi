use std::{
    io::{Error, Result},
    os::fd::{AsFd, BorrowedFd, IntoRawFd, RawFd}, path::PathBuf,
};

// A safe Fd wrapper
pub struct Fd {
    fd: RawFd,
}

impl Fd {
    fn get_filename(&self) -> Result<PathBuf> {
        const MAXPATHLEN: usize = 1024;

        let mut buf = [0; MAXPATHLEN];
        // TODO: Implement readlink
        // let ret = unsafe { libc::fcntl(self.fd, libc::F_, &mut buf) };

        // if ret == -1 {
        //     Err(Error::last_os_error())
        // } else {
        //     let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        //     Ok(PathBuf::from(OsStr::from_bytes(&buf[..end])))
        // }
        // self.fd
    }
}

impl Drop for Fd {
    fn drop(&mut self) {
        unsafe {
            // Panic on error
            match libc::close(self.fd) {
                -1 => {
                    let err = Error::last_os_error().to_string();
                    panic!("{err}")
                }
                _ => (),
            }
        }
    }
}

impl IntoRawFd for Fd {
    fn into_raw_fd(self) -> RawFd {
        self.fd
    }
}

impl From<i32> for Fd {
    fn from(value: i32) -> Self {
        Fd { fd: value }
    }
}

impl Into<i32> for Fd {
    fn into(self) -> i32 {
        self.fd
    }
}

impl PartialEq for Fd {
    fn eq(&self, other: &Fd) -> bool {
        self.fd == other.fd
    }
}

impl AsFd for Fd {
    fn as_fd(&self) -> std::os::unix::prelude::BorrowedFd<'_> {
        unsafe { BorrowedFd::borrow_raw(self.fd) }
    }
}

impl AsRef<i32> for Fd {
    fn as_ref(&self) -> &i32 {
        return &self.fd;
    }
}
