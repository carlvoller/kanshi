
#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(target_os = "macos")]
pub mod darwin;

#[cfg(target_os = "macos")]
pub use darwin::*;
