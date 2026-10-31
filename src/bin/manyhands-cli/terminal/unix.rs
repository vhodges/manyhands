use std::{
    fs::File,
    io::{self, Write},
    mem::MaybeUninit,
    os::fd::{AsRawFd, FromRawFd, RawFd},
};

use super::{LineEditor, Step, Terminal, Zeroizing};

struct Tty(File);

pub fn open() -> Option<Box<dyn Terminal>> {
    // SAFETY: the path is NUL-terminated and these flags need no mode argument.
    let fd = unsafe { libc::open(c"/dev/tty".as_ptr(), libc::O_RDWR | libc::O_CLOEXEC) };
    if fd < 0 {
        return None;
    }
    // SAFETY: open returned a fresh owned descriptor; File closes it exactly once.
    Some(Box::new(Tty(unsafe { File::from_raw_fd(fd) })))
}

struct RestoreMode {
    fd: RawFd,
    saved: libc::termios,
}

fn set_mode(fd: RawFd, action: libc::c_int, mode: &libc::termios) -> io::Result<()> {
    loop {
        // SAFETY: the caller keeps fd open; mode points to initialized termios.
        if unsafe { libc::tcsetattr(fd, action, mode) } == 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
}

impl Drop for RestoreMode {
    fn drop(&mut self) {
        let _ = set_mode(self.fd, libc::TCSANOW, &self.saved);
    }
}

impl Terminal for Tty {
    fn write(&mut self, text: &str) -> io::Result<()> {
        self.0.write_all(text.as_bytes())
    }

    fn ask(&mut self, prompt: &str, echo: bool) -> io::Result<Option<Zeroizing<String>>> {
        let fd = self.0.as_raw_fd();
        let mut saved = MaybeUninit::uninit();
        // SAFETY: fd is open and saved points to writable termios storage.
        if unsafe { libc::tcgetattr(fd, saved.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: tcgetattr succeeded and initialized saved.
        let saved = unsafe { saved.assume_init() };
        let _restore = RestoreMode { fd, saved };
        let mut reading = saved;
        reading.c_lflag &= !(libc::ICANON | libc::ECHO | libc::ISIG);
        reading.c_cc[libc::VMIN] = 1;
        reading.c_cc[libc::VTIME] = 0;
        set_mode(fd, libc::TCSAFLUSH, &reading)?;
        self.write(prompt)?;

        let mut editor = LineEditor::new(echo);
        let mut byte = Zeroizing::new([0u8; 1]);
        loop {
            // SAFETY: fd remains open; byte has space for the single requested byte.
            let read = unsafe { libc::read(fd, byte.as_mut_ptr().cast(), 1) };
            if read < 0 {
                let error = io::Error::last_os_error();
                if error.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(error);
            }
            match if read == 0 {
                Step::Cancel
            } else {
                editor.feed(byte[0])
            } {
                Step::Pending(bytes) => self.0.write_all(&bytes)?,
                Step::Cancel => {
                    self.write("\n")?;
                    return Ok(None);
                }
                Step::Done => {
                    self.write("\n")?;
                    let line = editor.finish();
                    if line.is_none() {
                        self.write("Input is not valid UTF-8; cancelled.\n")?;
                    }
                    return Ok(line);
                }
            }
        }
    }
}
