mod platforms;

use std::thread;

use nix::sys::ptrace;
// use nix::sys::personality;

pub struct FileSystemTracer {
    command: Vec<String>,
    is_executing: bool
}

impl FileSystemTracer {

    pub fn new(command: Vec<String>) -> FileSystemTracer {
        FileSystemTracer {
            command,
            is_executing: false,
        }
    }

    pub fn start(&mut self) {
        self.is_executing = true;

        let handle = thread::spawn(|| {
            ptrace::traceme()
            // personality::
        });
    }

}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn it_works() {
//         let result = add(2, 2);
//         assert_eq!(result, 4);
//     }
// }
