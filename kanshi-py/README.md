# kanshipy
>  An easy-to-use, efficient, and performant filesystem watcher

Kanshipy is a high level abstraction above native file system watcher APIs (like inotify, fanotify, etc).

Kanshi listens for events on a separate thread from the Rust side. As such, it functions asyncronously even in syncronous contexts. This library does not function any differently when run in a normal context or using `asyncio`. All subscriptions will be notified whenever an event is received on the Rust side.

### Supported Platforms
1. Linux
    - [inotify](https://man7.org/linux/man-pages/man7/inotify.7.html) for Unprivileged Users
    - [fanotify](https://man7.org/linux/man-pages/man7/fanotify.7.html) for Root
2. Darwin (MacOS) - [Core Services' File System Events API](https://developer.apple.com/documentation/coreservices/file_system_events)


> Kanshi.js currently does not support Windows. Windows support is planned.

### Installation
```zsh
$ pip install kanshipy
```

### Usage
```python
from kanshipy import KanshiPy
import time

# Set up a callback. Your callback can be any Python Callable.
def on_event(event):
  print("Received an event:")
  print(f"Type: {event.event_type}")
  print(f"Path: {event.target.path}")
  print(f"Target Kind: {event.target.kind}")

# Create Kanshi Instance
kanshi = KanshiPy()

# Watch "my_directory"
kanshi.watch("./my_directory")

# Subscribe to file system events
kanshi.subscribe(on_event)

# Start the Kanshi listener
kanshi.start()

# Wait 10 seconds
time.sleep(10)

# Close the Kanshi Listener
kanshi.close()
```

### Docs
Kanshipy exports 3 classes: `KanshiPy`, `KanshiEvent` and `KanshiEventTarget`. The `Kanshi` constructor optionally takes the following parameters:

```python
from kanshipy import KanshiPy

kanshi = KanshiPy(
  force_engine="fanotify"
)
```

- `force_engine` - Forces Kanshi to use a specific underlying engine. Accepted values depends on your environment. This option is currently only useful on Linux.
> On Linux, Kanshi will use inotify for non-root users, and fanotify for root users. Fanotify is more performant than inotify, however requires Root (or **CAP_SYS_ADMIN**) privileges. If you want Kanshi running as Root to use inotify, or an unprivileged Kanshi to use Fanotify, the `force_engine` option would be useful.

> On MacOS, `force_engine` is useless as it only accepts `fsevents`. I may choose to support the `kqueue` interface from FreeBSD at some point, in which this option will allow you to use `kqueue` over `fsevents`. Apple currently encourages the use of their Core Services File System Events API (`fsevents`) [here](https://developer.apple.com/library/archive/documentation/Darwin/Conceptual/FSEvents_ProgGuide/KernelQueues/KernelQueues.html#:~:text=If%20you%20are%20monitoring%20a,additional%20user%2Dkernel%20communication%20involved.).

#### `kanshi.watch(dir: str)`
Watches the specified directory. The `dir` can be an absolute path or a relative path.

```python
kan = KanshiPy()

try:
  kan.watch(".")
except RuntimeError e:
  print("An error occurred watching the directory: " + e)
```

This method is a syncronous error that returns `None`. It is possible for this method to raise a `RuntimeError`. A `RuntimeError` usually occurs if the directory you're trying to watch doesn't exist, or if you lack the permissions to watch the directory.

#### `kanshi.subscribe(callback: Callable[[KanshiEvent], None])`

Subscribes to this Kanshi instance. Use this to receive events from your Kanshi listener.

The `callback` argument can be any python object that conforms to `Callable`. This includes pure functions, class methods and lambdas.

The `callback` callable should conform to this signature: `callback(event: KanshiEvent)`.

`KanshiEvent` has the following properties:
- `event_type` - Can be "move", "create", "delete", "moved_from", "moved_to", "modify", "unknown"
- `target` - A `KanshiEventTarget` instance. This can be `None`.

`KanshiEventTarget` has the following properties:
- `path` - Absolute path to the directory item that produced the event.
- `kind` - The kind of directory item that produced the event. This can be "directory" or "file".
- `previous_path` - This is `None` unless the `event_type` is "moved_from", in which this will contain the absolute path of the file's previous location.
- `new_path` - This is `None` unless the `event_type` is "moved_to", in which this will contain the absolute path of the file's new location.

All events types except for `"unknown"` is expected to have a target. An `"unknown"` event shouldn't occur in normal usage. Please open an issue if you encountered an `"unknown"` event.

There are 3 possible _**move**_ `eventTypes` that Kanshi can produce:
1. `moved_to` - The directory item that exists at `path` has been moved to another watched location. The item's new location can be accessed at `event.target.moved_to`.
2. `moved_from` - The directory item that exists at `path` was moved from another watched location. The item's old location can be accessed at `event.target.moved_from`.
3. `move` - This has 2 possible meanings:
    1. The directory item that exists at `path` was moved somewhere else that is not currently watched.
    2. The directory item at `path` was just moved here from somewhere else that is not currently watched.

Example:

Given a directory tree of:
```
- index.js
- folderA
| - hello.txt
- folderB
| - byebye.txt
```

**main.py**:
```python
from kanshipy import KanshiPy
import os

kan = KanshiPy()

def on_event(event):
  print("Received an event:")
  print(f"Type: {event.event_type}")
  print(f"Path: {event.target.path}")
  print(f"Target Kind: {event.target.kind}")

kan.subscribe(on_event)
kan.watch("./folderA")
kan.start()

# This will cause a "moved_to" THEN a "moved_from" event
os.rename("./folderA/hello.txt", "./folderA/howdy.txt");

# This will cause a "move" event where "path" is "folderA/howdy.txt"
os.rename("./folderA/howdy.txt", "./folderB/howdy.txt");

# This will cause a "move" event where "path" is "folderA/adios.txt"
os.rename("./folderB/byebye.txt", "./folderA/adios.txt");

```

#### `kanshi.start()`
This method starts the Kanshi listener.

```python
from kanshipy import KanshiPy

kan = KanshiPy()

kan.watch(".")
kan.start()
```

> Once a Kanshi instance has been started, you cannot watch any new directories.

#### `kanshi.close() -> boolean`
This method closes the Kanshi listener. Calling this method will deregister all event listeners.

> Once a Kanshi instance is closed, it cannot be reused.

### Contributing
PRs are welcomed! Any help is appreciated. Please refer to the main Kanshi project for more information.

### License
Copyright © 2025, Carl Ian Voller. Released under the BSD-3-Clause License.