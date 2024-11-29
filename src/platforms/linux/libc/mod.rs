pub mod fanotify;

#[macro_use] pub mod macros;

mod fcntl {
    use std::os::{fd::RawFd, raw};

    pub(crate) fn at_rawfd(fd: Option<RawFd>) -> raw::c_int {
        fd.unwrap_or(libc::AT_FDCWD)
    }
}