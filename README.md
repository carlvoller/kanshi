# kanshi 監視
>  An easy-to-use filesystem watcher for Rust, Python and NodeJS. 

Kanshi provides bindings to native file system APIs. (such as inotify, fanotify, etc)

Kanshi was created with the following goals:
1. Provide a simple API to call low-level system watcher APIs.
2. Memory efficient when watching a large number of directory objects
3. Provide the same API across all major platforms. (WIP)

> Windows support is currently planned.

## Libraries

1. JavaScript - [kanshi-js](https://github.com/carlvoller/kanshi/tree/main/kanshi-js)
2. Python - [kanshipy](https://github.com/carlvoller/kanshi/tree/main/kanshi-py)
3. Rust - [kanshi](https://github.com/carlvoller/kanshi/kanshi) (WIP)

The Rust library is awaiting [this PR](https://github.com/nix-rust/nix/pull/2552) to be merged into Nix.


### Contributing
PRs are welcomed! Any help is appreciated.

Please create an Issue regarding the problems/missing features your PR will address. Please also tag your Issue with the language you are making your contributions to (such as Rust for the main Kanshi library, JavaScript for kanshi-js and Python for kanshipy).

### License
Copyright © 2025, Carl Ian Voller. Released under the BSD-3-Clause License.