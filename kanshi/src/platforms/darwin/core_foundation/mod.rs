pub mod types;

use std::{
    ffi::{CStr, CString},
    os::raw::c_char,
    ptr::null_mut,
    str,
};

use types::*;

#[link(name = "CoreServices", kind = "framework")]
extern "C" {
    /// https://developer.apple.com/documentation/coreservices/1443980-fseventstreamcreate?language=objc
    pub fn FSEventStreamCreate(
        allocator: CFAllocatorRef,
        callback: FSEventStreamCallback,
        context: *const FSEventStreamContext,
        pathsToWatch: CFArrayRef,
        sinceWhen: FSEventStreamId,
        latency: CFTimeInterval,
        flags: FSEventStreamCreateFlags,
    ) -> FSEventStreamRef;

    /// https://developer.apple.com/documentation/coreservices/1443980-fseventstreamcreate?language=objc
    pub fn FSEventStreamStart(streamRef: FSEventStreamRef) -> Boolean;

    /// https://developer.apple.com/documentation/coreservices/1447673-fseventstreamstop?language=objc
    pub fn FSEventStreamStop(streamRef: FSEventStreamRef);

    /// https://developer.apple.com/documentation/coreservices/1446990-fseventstreaminvalidate?language=objc
    pub fn FSEventStreamInvalidate(streamRef: FSEventStreamRef);

    /// https://developer.apple.com/documentation/coreservices/1445989-fseventstreamrelease?language=objc
    pub fn FSEventStreamRelease(streamRef: FSEventStreamRef);
}

// Implements https://developer.apple.com/documentation/coreservices/file_system_events?language=objc
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {

    pub static kCFTypeArrayCallBacks: CFArrayCallBacks;

    /// https://developer.apple.com/documentation/dispatch/1453030-dispatch_queue_create/
    pub fn dispatch_queue_create(
        label: *const c_char,
        attr: dispatch_queue_attr_t,
    ) -> dispatch_queue_t;

    /// https://developer.apple.com/documentation/dispatch/1496328-dispatch_release
    pub fn dispatch_release(object: dispatch_queue_t);

    /// https://developer.apple.com/documentation/coreservices/1443980-fseventstreamcreate?language=objc
    pub fn FSEventStreamSetDispatchQueue(streamRef: FSEventStreamRef, q: dispatch_queue_t);

    pub fn FSEventStreamFlushSync(streamRef: FSEventStreamRef);

    pub fn CFArrayCreateMutable(
        allocator: CFRef,
        capacity: CFIndex,
        callbacks: *const CFArrayCallBacks,
    ) -> CFMutableArrayRef;

    pub fn CFURLCreateFromFileSystemRepresentation(
        allocator: CFRef,
        path: *const ::std::os::raw::c_char,
        len: CFIndex,
        is_directory: bool,
    ) -> CFURLRef;

    pub fn CFURLCopyAbsoluteURL(res: CFURLRef) -> CFURLRef;

    pub fn CFRelease(res: CFRef);

    pub fn CFURLResourceIsReachable(res: CFURLRef, err: *mut CFErrorRef) -> bool;

    pub fn CFURLCopyLastPathComponent(res: CFURLRef) -> CFStringRef;

    pub fn CFArrayInsertValueAtIndex(arr: CFMutableArrayRef, position: CFIndex, element: CFRef);

    pub fn CFURLCreateCopyDeletingLastPathComponent(allocator: CFRef, url: CFURLRef) -> CFURLRef;

    pub fn CFURLCreateFileReferenceURL(
        allocator: CFRef,
        url: CFURLRef,
        err: CFErrorRef,
    ) -> CFURLRef;

    pub fn CFURLCreateFilePathURL(allocator: CFRef, url: CFURLRef, err: CFErrorRef) -> CFURLRef;

    pub fn CFArrayGetCount(arr: CFMutableArrayRef) -> CFIndex;
    pub fn CFArrayGetValueAtIndex(arr: CFMutableArrayRef, index: CFIndex) -> CFRef;
    pub fn CFDictionaryGetValue(dict: CFDictionaryRef, key: CFRef) -> CFRef;

    pub fn CFURLCreateCopyAppendingPathComponent(
        allocation: CFRef,
        url: CFURLRef,
        path: CFStringRef,
        is_directory: bool,
    ) -> CFURLRef;

    pub fn CFURLCopyFileSystemPath(anUrl: CFURLRef, path_style: CFURLPathStyle) -> CFStringRef;

    pub fn CFArrayAppendValue(aff: CFMutableArrayRef, element: CFRef);

    pub fn CFStringCreateWithBytes(
        allocator: CFRef,
        bytes: *const u8,
        numBytes: isize,
        encoding: u32,
        isExternalRepresenation: u8,
    ) -> CFStringRef;

    pub fn CFNumberGetValue(number: CFNumberRef, theType: u32, valuePtr: *mut CFRef) -> bool;

    pub fn CFStringGetCStringPtr(theString: CFStringRef, encoding: u32) -> *const i8;

    pub fn CFStringGetLength(theString: CFStringRef) -> isize;

    pub fn CFStringGetBytes(
        theString: CFStringRef,
        range: CFRange,
        encoding: u32,
        lossByte: u8,
        isExternalRepresentation: u8,
        buffer: *mut u8,
        maxBufLen: isize,
        usedBufLen: *mut isize,
    ) -> isize;

}

pub unsafe fn rust_str_to_cf_string(rust_str: &str, err: CFErrorRef) -> CFStringRef {
    let c_str = CString::new(rust_str).unwrap();
    let c_str_len = libc::strlen(c_str.as_ptr());

    let mut url = CFURLCreateFromFileSystemRepresentation(
        kCFAllocatorDefault,
        c_str.as_ptr(),
        c_str_len as CFIndex,
        false,
    );

    if url.is_null() {
        return null_mut();
    }

    let mut placeholder = CFURLCopyAbsoluteURL(url);
    CFRelease(url);
    if placeholder.is_null() {
        return null_mut();
    }

    let mut imaginary: CFRef = null_mut();

    while !CFURLResourceIsReachable(placeholder, null_mut()) {
        if imaginary.is_null() {
            imaginary =
                CFArrayCreateMutable(kCFAllocatorDefault, 0 as CFIndex, &kCFTypeArrayCallBacks);
            if imaginary.is_null() {
                CFRelease(placeholder);
                return null_mut();
            }
        }

        let child = CFURLCopyLastPathComponent(placeholder);
        CFArrayInsertValueAtIndex(imaginary, 0 as CFIndex, child);
        CFRelease(child);

        url = CFURLCreateCopyDeletingLastPathComponent(kCFAllocatorDefault, placeholder);
        CFRelease(placeholder);
        placeholder = url;
    }

    url = CFURLCreateFileReferenceURL(kCFAllocatorDefault, placeholder, err);
    CFRelease(placeholder);
    if url.is_null() {
        if !imaginary.is_null() {
            CFRelease(imaginary);
        }
        return null_mut();
    }

    placeholder = CFURLCreateFilePathURL(kCFAllocatorDefault, url, err);
    CFRelease(url);
    if placeholder.is_null() {
        if !imaginary.is_null() {
            CFRelease(imaginary);
        }
        return null_mut();
    }

    if !imaginary.is_null() {
        let mut count: i64 = 0;
        while count < *CFArrayGetCount(imaginary) {
            let component = CFArrayGetValueAtIndex(imaginary, &mut count);
            url = CFURLCreateCopyAppendingPathComponent(
                kCFAllocatorDefault,
                placeholder,
                component,
                false,
            );
            CFRelease(placeholder);
            if url.is_null() {
                CFRelease(imaginary);
                return null_mut();
            }
            placeholder = url;
            count += 1;
        }
        CFRelease(imaginary);
    }

    let cf_path = CFURLCopyFileSystemPath(placeholder, kCFURLPOSIXPathStyle);
    CFRelease(placeholder);
    cf_path
}

pub unsafe fn cfstr_to_str(string: CFStringRef) -> String {
    let cstr_ptr = CFStringGetCStringPtr(string, kCFStringEncodingUTF8);
    if !cstr_ptr.is_null() {
        str::from_utf8_unchecked(CStr::from_ptr(cstr_ptr).to_bytes()).to_owned()
    } else {
        let str_len = CFStringGetLength(string);
        let mut bytes_required = 0isize;
        CFStringGetBytes(
            string,
            CFRange {
                location: 0,
                length: str_len,
            },
            kCFStringEncodingUTF8,
            0,
            false as Boolean,
            std::ptr::null_mut(),
            0,
            &mut bytes_required,
        );

        println!("BYTES REQUIRED -- {bytes_required}");
        let mut final_string_buffer = Vec::with_capacity(bytes_required as usize);
        final_string_buffer.fill(b'\x00');

        let mut bytes_used = 0;
        CFStringGetBytes(
            string,
            CFRange {
                location: 0,
                length: str_len,
            },
            kCFStringEncodingUTF8,
            0,
            false as Boolean,
            final_string_buffer.as_mut_ptr(),
            final_string_buffer.len() as isize,
            &mut bytes_used,
        );

        println!("what {:?}", final_string_buffer);

        String::from_utf8_unchecked(final_string_buffer)

        // CStr::from_bytes_with_nul_unchecked(&final_string_buffer).to_str().ok().unwrap()
    }
}
