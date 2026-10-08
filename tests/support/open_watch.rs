//! A Linux-only watch that reports whether a file was opened or read.
//!
//! Other platforms have no portable way to see an open whose result is
//! discarded, so the tests that use this are Linux-only as well.

use std::path::PathBuf;

/// Reports whether any watched file has been opened or read since the
/// last call.
pub struct OpenWatch {
    descriptor: libc::c_int,
}

impl OpenWatch {
    pub fn on(paths: &[PathBuf]) -> Self {
        let descriptor = unsafe { libc::inotify_init1(libc::IN_NONBLOCK | libc::IN_CLOEXEC) };
        assert!(descriptor >= 0);
        for path in paths {
            let name = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
            let watch = unsafe {
                libc::inotify_add_watch(descriptor, name.as_ptr(), libc::IN_OPEN | libc::IN_ACCESS)
            };
            assert!(watch >= 0, "{path:?} cannot be watched");
        }
        Self { descriptor }
    }

    pub fn saw_an_open_or_a_read(&self) -> bool {
        let mut events = [0u8; 4096];
        let mut seen = false;
        loop {
            let read =
                unsafe { libc::read(self.descriptor, events.as_mut_ptr().cast(), events.len()) };
            if read > 0 {
                seen = true;
                continue;
            }
            assert_eq!(read, -1);
            assert_eq!(
                std::io::Error::last_os_error().raw_os_error(),
                Some(libc::EAGAIN)
            );
            return seen;
        }
    }
}

impl Drop for OpenWatch {
    fn drop(&mut self) {
        unsafe { libc::close(self.descriptor) };
    }
}
