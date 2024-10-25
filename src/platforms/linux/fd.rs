use std::{
    io::Error,
    os::fd::{AsFd, AsRawFd, BorrowedFd, IntoRawFd, RawFd},
};

// A safe Fd wrapper
pub struct Fd {
    fd: RawFd,
}

impl Drop for Fd {
    fn drop(&mut self) {
        unsafe {
            // Panic on error
            match libc::close(self.fd) {
                -1 => panic!(Error::last_os_error().to_string()),
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
