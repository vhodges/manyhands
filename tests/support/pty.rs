//! A private controlling terminal, separate from all three standard streams.
use super::cli::{CliEnv, CliRun, cli_environment};
use std::{
    cell::RefCell,
    ffi::{CStr, CString, OsStr},
    fs::File,
    io::{self, Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::process::CommandExt,
    },
    process::{Child, Command, ExitStatus, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub struct PtyCli {
    child: RefCell<Option<Child>>,
    master: File,
    slave: Option<File>,
    exited: Arc<AtomicBool>,
    reader: Option<JoinHandle<()>>,
    terminal: Receiver<io::Result<Vec<u8>>>,
    pipes: Receiver<(bool, io::Result<Vec<u8>>)>,
    workers: Vec<JoinHandle<()>>,
    transcript: Vec<u8>,
    consumed: usize,
    before: libc::tcflag_t,
}

fn owned(fd: libc::c_int) -> io::Result<File> {
    if fd < 0 {
        Err(io::Error::last_os_error())
    } else {
        // SAFETY: the successful OS call transfers a newly opened descriptor.
        Ok(unsafe { File::from_raw_fd(fd) })
    }
}
fn lflag(file: &File) -> libc::tcflag_t {
    let mut modes = std::mem::MaybeUninit::uninit();
    // SAFETY: modes has writable termios storage and the descriptor is live.
    assert_eq!(
        unsafe { libc::tcgetattr(file.as_raw_fd(), modes.as_mut_ptr()) },
        0
    );
    // SAFETY: successful tcgetattr initialized the storage.
    unsafe { modes.assume_init() }.c_lflag
}
fn slave_name(master: &File) -> CString {
    #[cfg(target_os = "linux")]
    {
        let mut buffer = [0 as libc::c_char; 1024];
        // SAFETY: live PTY descriptor, writable buffer and its exact length.
        assert_eq!(
            unsafe { libc::ptsname_r(master.as_raw_fd(), buffer.as_mut_ptr(), buffer.len()) },
            0
        );
        // SAFETY: successful ptsname_r returns a NUL-terminated path.
        unsafe { CStr::from_ptr(buffer.as_ptr()) }.to_owned()
    }
    #[cfg(not(target_os = "linux"))]
    {
        static NAME_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _lock = NAME_LOCK.lock().unwrap();
        // SAFETY: the static ptsname buffer is copied under the process lock.
        let name = unsafe { libc::ptsname(master.as_raw_fd()) };
        assert!(!name.is_null());
        unsafe { CStr::from_ptr(name) }.to_owned()
    }
}

impl PtyCli {
    pub fn spawn(args: &[&OsStr], env: &CliEnv, stdin: &[u8]) -> Self {
        // SAFETY: no pointers; the returned descriptor is owned immediately.
        let master =
            owned(unsafe { libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY | libc::O_CLOEXEC) })
                .unwrap();
        // SAFETY: master is an open PTY descriptor, owned for both calls.
        assert_eq!(unsafe { libc::grantpt(master.as_raw_fd()) }, 0);
        assert_eq!(unsafe { libc::unlockpt(master.as_raw_fd()) }, 0);
        let name = slave_name(&master);
        // SAFETY: name is NUL terminated. O_NOCTTY prevents a parent terminal change.
        let slave = owned(unsafe {
            libc::open(
                name.as_ptr(),
                libc::O_RDWR | libc::O_NOCTTY | libc::O_CLOEXEC,
            )
        })
        .unwrap();
        let before = lflag(&master);
        let mut command = Command::new(env!("CARGO_BIN_EXE_manyhands-cli"));
        command.args(args);
        cli_environment(&mut command, env);
        command.stdin(Stdio::piped());
        // SAFETY: only async-signal-safe syscalls run after fork. CString was
        // allocated before fork. No locks, allocations or Rust I/O in the child.
        unsafe {
            command.pre_exec(move || {
                if libc::setsid() == -1 {
                    return Err(io::Error::last_os_error());
                }
                let fd = libc::open(name.as_ptr(), libc::O_RDWR);
                if fd < 0 {
                    return Err(io::Error::last_os_error());
                }
                let result = libc::ioctl(fd, libc::TIOCSCTTY as _, 0);
                let error = if result < 0 {
                    Some(io::Error::last_os_error())
                } else {
                    None
                };
                libc::close(fd);
                match error {
                    Some(error) => Err(error),
                    None => Ok(()),
                }
            });
        }
        let mut child = command.spawn().unwrap();
        let mut workers = Vec::new();
        let (sender, pipes) = mpsc::channel();
        for (is_stdout, mut pipe) in [
            (
                true,
                Box::new(child.stdout.take().unwrap()) as Box<dyn Read + Send>,
            ),
            (
                false,
                Box::new(child.stderr.take().unwrap()) as Box<dyn Read + Send>,
            ),
        ] {
            let sender = sender.clone();
            workers.push(thread::spawn(move || {
                let mut bytes = Vec::new();
                let result = pipe.read_to_end(&mut bytes).map(|_| bytes);
                let _ = sender.send((is_stdout, result));
            }));
        }
        drop(sender);
        let mut input = child.stdin.take().unwrap();
        let bytes = stdin.to_vec();
        workers.push(thread::spawn(move || {
            let _ = input.write_all(&bytes);
        }));
        let exited = Arc::new(AtomicBool::new(false));
        let done = exited.clone();
        let mut read_master = master.try_clone().unwrap();
        let (sender, terminal) = mpsc::channel();
        let reader = thread::spawn(move || {
            let mut buffer = [0; 4096];
            loop {
                let mut fd = libc::pollfd {
                    fd: read_master.as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                };
                // SAFETY: one initialized pollfd, live descriptor, bounded wait.
                let ready = unsafe { libc::poll(&mut fd, 1, 50) };
                if ready < 0 {
                    let e = io::Error::last_os_error();
                    if e.kind() == io::ErrorKind::Interrupted {
                        continue;
                    }
                    let _ = sender.send(Err(e));
                    break;
                }
                if ready == 0 {
                    if done.load(Ordering::Acquire) {
                        break;
                    }
                    continue;
                }
                match read_master.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => {
                        if sender.send(Ok(buffer[..count].to_vec())).is_err() {
                            break;
                        }
                    }
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) if e.raw_os_error() == Some(libc::EIO) => {
                        // The kept slave prevents the pre-spawn EIO race. Only
                        // reaping the child makes EIO end-of-input, never spawn.
                        if done.load(Ordering::Acquire) {
                            break;
                        }
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(e) => {
                        let _ = sender.send(Err(e));
                        break;
                    }
                }
            }
        });
        Self {
            child: RefCell::new(Some(child)),
            master,
            slave: Some(slave),
            exited,
            reader: Some(reader),
            terminal,
            pipes,
            workers,
            transcript: Vec::new(),
            consumed: 0,
            before,
        }
    }
    pub fn expect(&mut self, text: &str) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(index) = self.transcript[self.consumed..]
                .windows(text.len())
                .position(|bytes| bytes == text.as_bytes())
            {
                self.consumed += index + text.len();
                return;
            }
            assert!(
                Instant::now() < deadline,
                "PTY prompt deadline: {text}; transcript: {}",
                String::from_utf8_lossy(&self.transcript)
            );
            match self.terminal.recv_timeout(Duration::from_millis(25)) {
                Ok(bytes) => self.transcript.extend(bytes.unwrap()),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    panic!("PTY closed before prompt: {text}")
                }
            }
            if self
                .child
                .borrow_mut()
                .as_mut()
                .unwrap()
                .try_wait()
                .unwrap()
                .is_some()
            {
                self.exited.store(true, Ordering::Release);
            }
        }
    }
    pub fn send(&mut self, bytes: &[u8]) {
        self.master.write_all(bytes).unwrap();
    }
    fn wait(&self) -> ExitStatus {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = self
                .child
                .borrow_mut()
                .as_mut()
                .unwrap()
                .try_wait()
                .unwrap()
            {
                self.exited.store(true, Ordering::Release);
                return status;
            }
            if Instant::now() >= deadline {
                self.stop();
                panic!("PTY child exceeded 10 seconds");
            }
            thread::sleep(Duration::from_millis(5));
        }
    }
    fn stop(&self) {
        if let Some(child) = self.child.borrow_mut().as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.exited.store(true, Ordering::Release);
    }
    pub fn finish(mut self) -> (CliRun, String) {
        let status = self.wait();
        self.slave.take();
        if let Some(reader) = self.reader.take() {
            reader.join().unwrap();
        }
        for bytes in self.terminal.try_iter() {
            self.transcript.extend(bytes.unwrap());
        }
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(1);
        for _ in 0..2 {
            let (is_stdout, bytes) = self
                .pipes
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .expect("PTY standard pipe EOF deadline");
            if is_stdout {
                stdout = bytes.unwrap()
            } else {
                stderr = bytes.unwrap()
            }
        }
        for worker in self.workers.drain(..) {
            worker.join().unwrap();
        }
        self.child.borrow_mut().take();
        (
            CliRun {
                stdout,
                stderr,
                status: status.code().expect("PTY child died by signal"),
            },
            String::from_utf8(std::mem::take(&mut self.transcript)).unwrap(),
        )
    }
    pub fn lflag_before_and_after(&self) -> (libc::tcflag_t, libc::tcflag_t) {
        self.wait();
        (self.before, lflag(&self.master))
    }
}
impl Drop for PtyCli {
    fn drop(&mut self) {
        self.stop();
        self.slave.take();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}
