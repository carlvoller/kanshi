#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(unused)]

use bitflags::bitflags;
use std::{
    cell::LazyCell,
    os::raw::{c_long, c_uchar, c_uint, c_void},
};

use once_cell::sync::Lazy;

use super::CFStringCreateWithBytes;

//
// MacOS CoreFoundation types
//
pub type Boolean = c_uchar;

pub enum CFError {}
pub type CFRef = *mut c_void;
pub type CFErrorRef = *mut CFError;
pub type CFStringRef = CFRef;
pub type CFNumberRef = CFRef;
pub type CFArrayRef = CFRef;
pub type CFDictionaryRef = CFRef;
pub type CFAllocatorRef = CFRef;
pub type CFMutableArrayRef = CFRef;
pub type CFURLRef = CFRef;
pub type CFIndex = *mut c_long;
pub type CFURLPathStyle = CFIndex;
pub type CFAllocatorRetainCallBack = extern "C" fn(*const c_void) -> *const c_void;
pub type CFAllocatorReleaseCallBack = extern "C" fn(*const c_void);
pub type CFAllocatorCopyDescriptionCallBack = extern "C" fn(*const c_void) -> *const CFStringRef;
pub type CFTimeInterval = f64;
pub type CFAbsoluteTime = CFTimeInterval;
pub type CFArrayRetainCallBack = extern "C" fn(CFAllocatorRef, *const c_void) -> *const c_void;
pub type CFArrayReleaseCallBack = extern "C" fn(CFAllocatorRef, *const c_void);
pub type CFArrayCopyDescriptionCallBack = extern "C" fn(*const c_void) -> CFStringRef;
pub type CFArrayEqualCallBack = extern "C" fn(*const c_void, *const c_void) -> Boolean;

#[repr(C)]
pub struct CFRange {
    pub location: isize,
    pub length: isize,
}

#[repr(C)]
pub struct CFArrayCallBacks {
    version: CFIndex,
    retain: Option<CFArrayRetainCallBack>,
    release: Option<CFArrayReleaseCallBack>,
    cp: Option<CFArrayCopyDescriptionCallBack>,
    equal: Option<CFArrayEqualCallBack>,
}

pub const NULL: CFRef = 0 as CFRef;
pub const kCFAllocatorDefault: CFAllocatorRef = NULL;
pub const kCFURLPOSIXPathStyle: CFURLPathStyle = 0 as CFIndex;
pub const kCFNumberSInt64Type: u32 = 4;

//
// MacOS DispatchQueue types
//
pub type dispatch_object_s = c_void;
pub type dispatch_queue_t = *mut dispatch_object_s;
pub type dispatch_queue_attr_t = *const dispatch_object_s;
pub const DISPATCH_QUEUE_SERIAL: dispatch_queue_attr_t = 0 as dispatch_queue_attr_t;

//
// MacOS FSEvents types
//
pub type FSEventStreamRef = CFRef;
pub type FSEventStreamId = u64;
pub type FSEventStreamCallback = extern "C" fn(
    *const FSEventStreamRef, // ConstFSEventStreamRef - Reference to the stream this event originated from
    CFRef, // *mut FSEventStreamContext->info - Optionally supplied context during stream creation.
    usize, // numEvents - Number of total events in this callback
    CFRef, // eventPaths - Array of C Strings representing the paths where each event occurred
    *const FSEventStreamEventFlags, // eventFlags - Array of EventFlags corresponding to each event
    *const FSEventStreamId, // eventIds - Array of EventIds corresponding to each event. This Id is guaranteed to always be increasing.
);

pub const kCFStringEncodingUTF8: u32 = 0x08000100;
pub const kFSEventStreamEventIdSinceNow: FSEventStreamId = u64::MAX;
pub const kFSEventStreamEventExtendedDataPathKey: Lazy<CFStringRef> = Lazy::new(|| unsafe {
    CFStringCreateWithBytes(
        kCFAllocatorDefault,
        "path".as_ptr(),
        "path".len() as isize,
        kCFStringEncodingUTF8,
        false as Boolean,
    )
});
pub const kFSEventStreamEventExtendedFileIDKey: Lazy<CFStringRef> = Lazy::new(|| unsafe {
    CFStringCreateWithBytes(
        kCFAllocatorDefault,
        "fileID".as_ptr(),
        "fileID".len() as isize,
        kCFStringEncodingUTF8,
        false as Boolean,
    )
});

#[repr(C)]
pub struct FSEventStreamContext {
    pub version: CFIndex,
    pub info: CFRef,
    pub retain: Option<CFAllocatorRetainCallBack>,
    pub release: Option<CFAllocatorReleaseCallBack>,
    pub copy_description: Option<CFAllocatorCopyDescriptionCallBack>,
}

bitflags! {
  #[repr(C)]
  pub struct FSEventStreamCreateFlags: c_uint {
    const kFSEventStreamCreateFlagNone = 0x00000000;
    #[doc(hidden)]
    const kFSEventStreamCreateFlagUseCFTypes = 0x00000001; // MUST NOT BE SET
    const kFSEventStreamCreateFlagNoDefer = 0x00000002;
    const kFSEventStreamCreateFlagWatchRoot = 0x00000004;
    const kFSEventStreamCreateFlagIgnoreSelf = 0x00000008;
    const kFSEventStreamCreateFlagFileEvents = 0x00000010;
    const kFSEventStreamCreateFlagMarkSelf = 0x00000020;
    const kFSEventStreamCreateFlagUseExtendedData = 0x00000040;
    const kFSEventStreamCreateFlagFullHistory = 0x00000080;
  }
}

bitflags! {
  #[repr(C)]
  #[derive(Clone, Copy, Debug)]
  pub struct FSEventStreamEventFlags: c_uint {
    const kFSEventStreamEventFlagNone = 0x00000000;
    const kFSEventStreamEventFlagMustScanSubDirs = 0x00000001;
    const kFSEventStreamEventFlagUserDropped = 0x00000002;
    const kFSEventStreamEventFlagKernelDropped = 0x00000004;
    const kFSEventStreamEventFlagEventIdsWrapped = 0x00000008;
    const kFSEventStreamEventFlagHistoryDone = 0x00000010;
    const kFSEventStreamEventFlagRootChanged = 0x00000020;
    const kFSEventStreamEventFlagMount = 0x00000040;
    const kFSEventStreamEventFlagUnmount = 0x00000080;
    const kFSEventStreamEventFlagItemCreated = 0x00000100;
    const kFSEventStreamEventFlagItemRemoved = 0x00000200;
    const kFSEventStreamEventFlagItemInodeMetaMod = 0x00000400;
    const kFSEventStreamEventFlagItemRenamed = 0x00000800;
    const kFSEventStreamEventFlagItemModified = 0x00001000;
    const kFSEventStreamEventFlagItemFinderInfoMod = 0x00002000;
    const kFSEventStreamEventFlagItemChangeOwner = 0x00004000;
    const kFSEventStreamEventFlagItemXattrMod = 0x00008000;
    const kFSEventStreamEventFlagItemIsFile = 0x00010000;
    const kFSEventStreamEventFlagItemIsDir = 0x00020000;
    const kFSEventStreamEventFlagItemIsSymlink = 0x00040000;
    const kFSEventStreamEventFlagOwnEvent = 0x00080000;
    const kFSEventStreamEventFlagItemIsHardlink = 0x00100000;
    const kFSEventStreamEventFlagItemIsLastHardlink = 0x00200000;
    const kFSEventStreamEventFlagItemCloned = 0x00400000;
  }
}
