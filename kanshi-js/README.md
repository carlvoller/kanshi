# kanshi-js
>  An easy-to-use, efficient, and performant filesystem watcher

Kanshi.js is a high level abstraction above native file system watcher APIs (like iNotify, Fanotify, etc).

Unlike many other file system watchers, Kanshi does not rely on the NodeJS core `fs` module. Instead, Kanshi translates responses received directly from your system's Kernel into easy to parse JavaScript objects. This lets Kanshi work _regardless of your JavaScript runtime_.

### Supported Platforms
1. Linux
    - [inotify](https://man7.org/linux/man-pages/man7/inotify.7.html) for Unprivileged Users
    - [fanotify](https://man7.org/linux/man-pages/man7/fanotify.7.html) for Root
2. Darwin (MacOS) - [Core Services' File System Events API](https://developer.apple.com/documentation/coreservices/file_system_events)

> Kanshi.js currently does not support Windows. Windows support is planned.

### Installation
```zsh
$ npm install kanshi-js
```

### Usage
```typescript
import Kanshi, { type KanshiEvent } from 'kanshi-js';

const kan = new Kanshi();

// Set an event listener for events
kan.onEvent((event: KanshiEvent) => {
  const { eventType, target } = event;
  console.log(`A ${eventType} event just happened!`);
  if (target) {
    const { path, kind } = target;
    console.log(`The event happened on a ${kind} at ${path}`);
  }
});

// Recursively watch  "my_directory" and all its children for events
kan.watch("./my_directory").then(() => {

  // Recursively watch "another_directory" and all its children for events
  kan.watch("./another_directory").then(() => {

    // Start Kanshi. This promise will resolve when kanshi closes
    kan.start().then(() => {
      console.log("kanshi closed");
    }).catch(console.error);

  }).catch(console.error);

}).catch(console.error);

// Close kanshi after 10 seconds
setTimeout(() => kan.close(), 10000);
```

### Docs

Kanshi.js exports only a class `Kanshi`. The `Kanshi` constructor optionally takes in an object `KanshiOptions`:

```typescript
import Kanshi, { type KanshiOptions } from 'kanshi-js';

const options = {
  forceEngine: "inotify"
}

const kan = new Kanshi(options);
```

The `KanshiOptions` object has the following optional properties:

- `forceEngine` -  Forces Kanshi to use a specific underlying engine. Accepted values depends on your environment. This option is currently only useful on Linux.
> On Linux, Kanshi will use inotify for non-root users, and fanotify for root users. Fanotify is more performant than inotify, however requires Root (or **CAP_SYS_ADMIN**) privileges. If you want Kanshi running as Root to use inotify, or an unprivileged Kanshi to use Fanotify, the `forceEngine` option would be useful.

> On MacOS, `forceEngine` is useless as it only accepts `fsevents`. I may choose to support the `kqueue` interface from FreeBSD at some point, in which this option will allow you to use `kqueue` over `fsevents`. Apple currently encourages the use of their Core Services File System Events API (`fsevents`) [here](https://developer.apple.com/library/archive/documentation/Darwin/Conceptual/FSEvents_ProgGuide/KernelQueues/KernelQueues.html#:~:text=If%20you%20are%20monitoring%20a,additional%20user%2Dkernel%20communication%20involved.).

#### `kanshi.watch(dir: string): Promise<void>`
Watches the specified directory. The `dir` can be an absolute path or a relative path.

```typescript

const kan = new Kanshi();

kan
  .watch(".")
  .then(() => console.log("watched successfully"))
  .catch((err) => console.error(`error happened when watching directory: ${err}`));

```

This is an async method that resolves to `void` on success.

> Kanshi automatically watches a directory recursively. You don't have to manually watch subdirectories or files. Kanshi does not support watching individual files.

> On Linux, Kanshi supports the **fanotify** engine which can be much more performant for watching large directory trees than inotify. If you intend to watch a large tree of files, or maybe even an entire file system, it is recommended to use **fanotify**.

#### `kanshi.onEvent(callback: KanshiCallback): () => void`

Registers an event listener on this Kanshi instance. Use this to receive events from your Kanshi listener.

```typescript
import type { KanshiEvent } from 'kanshi-js';

const deregister = kan.onEvent((event: KanshiEvent) => {
  const { eventType, target } = event;
  const { path, kind, moved_from, moved_to } = target;
});

// Remove the event listener after 10 seconds
setTimeout(deregister, 10000);
```

`KanshiCallback` is a function that accepts one parameter `KanshiEvent` which has the type:
```typescript
interface KanshiEvent {
  eventType: "create" | "delete" | "modify" | "moved_to" | "moved_from" | "move" | "unknown";
  target?: {
    kind: "file" | "directory";
    path: string;
    moved_to?: string;
    moved_from?: string;
  }
}
```

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

**index.js**:
```typescript
import fs from "node:fs";
const kan = new Kanshi();

kan.onEvent((event) => console.log(event));
kan
  .watch("./folderA")
  .then(() => {
    kan.start();
  });

// This will cause a "moved_to" THEN a "moved_from" event
fs.renameSync("./folderA/hello.txt", "./folderA/howdy.txt");

// This will cause a "move" event where "path" is "folderA/howdy.txt"
fs.renameSync("./folderA/howdy.txt", "./folderB/howdy.txt");

// This will cause a "move" event where "path" is "folderA/adios.txt"
fs.renameSync("./folderB/byebye.txt", "./folderA/adios.txt");
```

This method returns a function. When called, the function deregisters the callback, ensuring the registered callback no longer receives any future events.

> Calling the returned deregister function _does not_ stop Kanshi.

#### `kanshi.start(): Promise<void>`
This method starts the Kanshi listener then returns a promise. The promise resolves when the Kanshi listener closes.

```typescript
import Kanshi from 'kanshi-js';

const kan = new Kanshi();

// Watch the current directory then start Kanshi.
kan.watch(".").then(() => kanshi.start());
```

> Once a Kanshi instance has been started, you cannot watch any new directories.

#### `kanshi.close(): boolean`
This method closes the Kanshi listener. Calling this method will deregister all event listeners and resolve the promise returned by `kanshi.start()`.

> Once a Kanshi instance is closed, it cannot be reused.

### Contributing
PRs are welcomed! Any help is appreciated. Please refer to the main Kanshi project for more information.

### License
Copyright © 2025, Carl Ian Voller. Released under the BSD-3-Clause License.