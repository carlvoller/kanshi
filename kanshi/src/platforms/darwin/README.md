# Darwin (MacOS)
## Bug
While using CoreServices's File System Events API, a file created on a watched directory that is later removed will produce a EVENT_FLAGS bitmask that claims that the file was simultaneously created and removed. This is a known bug in Darwin's FSEvents API.

> Refer to fsevents_bug.swift for an example of how to reproduce this bug.