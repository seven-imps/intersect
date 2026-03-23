use std::fs::File;
use std::io::{BufRead, BufReader};
use std::os::unix::io::FromRawFd;
use std::sync::mpsc::{self, Receiver};

/// Redirects stderr to a pipe and returns a channel that receives lines from it.
/// Must be called before anything writes to stderr.
pub fn capture() -> Receiver<String> {
    let (tx, rx) = mpsc::channel();

    let mut fds = [0i32; 2];
    unsafe {
        libc::pipe(fds.as_mut_ptr());
        libc::dup2(fds[1], libc::STDERR_FILENO);
        libc::close(fds[1]);
    }

    std::thread::spawn(move || {
        let reader = BufReader::new(unsafe { File::from_raw_fd(fds[0]) });
        for line in reader.lines().flatten() {
            if tx.send(line).is_err() {
                break;
            }
        }
    });

    rx
}
