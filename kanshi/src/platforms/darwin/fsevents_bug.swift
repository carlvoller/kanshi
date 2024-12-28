//
//  main.swift
//  FSEvents Bug Demonstration in Swift
//
//  Created by Carl Ian Voller on 28/12/24.
//

import Foundation
import CoreFoundation
import CoreServices

let dp = DispatchQueue(label: "")
var context = FSEventStreamContext()

// CHANGE THIS PATH TO AN ABSOLUTE PATH ON YOUR SYSTEM
// Then, open a terminal session in the path below and run `touch testing`
// and `rm testing`. Notice that that the `rm testing` is reported as both
// a creation and removed. (a 0x0300 event, which shouldn't be possible as 0x0100 is create and 0x200 is remove, so the event is both at the same time)
let pathsToWatch = ["/Users/carlvoller/Desktop/FSEvents Bug/FSEvents Bug/test/" as CFString] as CFArray
let flags = kFSEventStreamCreateFlagFileEvents | kFSEventStreamCreateFlagUseCFTypes | kFSEventStreamCreateFlagUseExtendedData

let contextPtr = withUnsafeMutablePointer(to: &context) {
    UnsafeMutablePointer($0)
}

public enum FSEvent {
    case changeInDirectory
    case rootChanged
    case itemChangedOwner
    case itemCreated
    case itemCloned
    case itemModified
    case itemRemoved
    case itemRenamed
    
    var isModificationOrDeletion: Bool {
        switch self {
            case .itemRenamed, .itemModified, .itemRemoved:
                return true
            default:
                return false
        }
    }
    
    init?(rawValue: FSEventStreamEventFlags) {
            if rawValue == 0 {
                self = .changeInDirectory
            } else if rawValue & UInt32(kFSEventStreamEventFlagRootChanged) > 0 {
                self = .rootChanged
            } else if rawValue & UInt32(kFSEventStreamEventFlagItemChangeOwner) > 0 {
                self = .itemChangedOwner
            } else if rawValue & UInt32(kFSEventStreamEventFlagItemCreated) > 0 {
                self = .itemCreated
            } else if rawValue & UInt32(kFSEventStreamEventFlagItemCloned) > 0 {
                self = .itemCloned
            } else if rawValue & UInt32(kFSEventStreamEventFlagItemModified) > 0 {
                self = .itemModified
            } else if rawValue & UInt32(kFSEventStreamEventFlagItemRemoved) > 0 {
                self = .itemRemoved
            } else if rawValue & UInt32(kFSEventStreamEventFlagItemRenamed) > 0 {
                self = .itemRenamed
            } else {
                return nil
            }
        }
}


if let stream = FSEventStreamCreate(
    kCFAllocatorDefault,
    { stream_ref, info, num_events, paths, flags, event_ids in
        guard let eventDictionaries = unsafeBitCast(paths, to: NSArray.self) as? [NSDictionary] else {
            return
        }
        
        for (index, dictionary) in eventDictionaries.enumerated() {
            guard let path = dictionary[kFSEventStreamEventExtendedDataPathKey] as? String,
                  let event = FSEvent(rawValue: flags[index])
            else {
                continue
            }
            
            print("\(event) Event")
            print("Hex: 0x\(String(flags[index], radix: 16, uppercase: true))")
            print("Happened at: \(path)")
            print("---")
        }
    },
    contextPtr,
    pathsToWatch,
    FSEventStreamEventId(kFSEventStreamEventIdSinceNow),
    0,
    FSEventStreamCreateFlags(flags)
) {
    FSEventStreamSetDispatchQueue(stream, dp)
    FSEventStreamStart(stream)
    print("FSEvents Listening started...")
}


sleep(1000)
