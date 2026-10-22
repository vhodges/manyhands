use std::io;

use zeroize::{Zeroize, Zeroizing};

#[cfg(unix)]
#[path = "terminal/unix.rs"]
mod platform;
#[cfg(windows)]
#[path = "terminal/windows.rs"]
mod platform;

pub trait Terminal {
    /// Text that asks nothing, such as a preview. Consumed by Task 3's prompts.
    #[allow(dead_code)]
    fn write(&mut self, text: &str) -> io::Result<()>;
    /// Set reading mode before the prompt; restore it on every exit path.
    /// A masked answer stays in zeroizing storage. None means cancelled.
    #[allow(dead_code)] // Task 3 introduces the first prompt caller.
    fn ask(&mut self, prompt: &str, echo: bool) -> io::Result<Option<Zeroizing<String>>>;
}

pub fn open() -> Option<Box<dyn Terminal>> {
    platform::open()
}

pub const MAX_LINE_BYTES: usize = 4096;

pub struct LineEditor {
    bytes: Zeroizing<Vec<u8>>,
    echo: bool,
}

pub enum Step {
    Pending(Vec<u8>),
    Done,
    Cancel,
}

impl LineEditor {
    pub fn new(echo: bool) -> Self {
        Self {
            // Never reallocate a buffer that may contain a passphrase.
            bytes: Zeroizing::new(Vec::with_capacity(MAX_LINE_BYTES)),
            echo,
        }
    }

    pub fn feed(&mut self, byte: u8) -> Step {
        let mut drawn = Vec::new();
        match byte {
            b'\r' | b'\n' => return Step::Done,
            0x03 => return Step::Cancel,
            0x04 if self.bytes.is_empty() => return Step::Cancel,
            0x7f | 0x08 => {
                if self.erase_last() && self.echo {
                    drawn.extend_from_slice(b"\x08 \x08");
                }
            }
            0x15 => {
                while self.erase_last() {
                    if self.echo {
                        drawn.extend_from_slice(b"\x08 \x08");
                    }
                }
            }
            0..=0x1f => {}
            _ if self.bytes.len() < MAX_LINE_BYTES => {
                self.bytes.push(byte);
                if self.echo {
                    drawn.push(byte);
                }
            }
            _ => {}
        }
        Step::Pending(drawn)
    }

    pub fn finish(self) -> Option<Zeroizing<String>> {
        std::str::from_utf8(&self.bytes)
            .ok()
            .map(|text| Zeroizing::new(text.to_owned()))
    }

    fn erase_last(&mut self) -> bool {
        let Some(mut start) = self.bytes.len().checked_sub(1) else {
            return false;
        };
        while start > 0 && self.bytes[start] & 0xc0 == 0x80 {
            start -= 1;
        }
        // Zero removed bytes before truncating: they must not survive in capacity.
        self.bytes[start..].zeroize();
        self.bytes.truncate(start);
        true
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct FakeTerminal {
    pub answers: std::collections::VecDeque<Option<String>>,
    pub written: String,
    pub asks: Vec<(String, bool)>,
}

#[cfg(test)]
impl Terminal for FakeTerminal {
    fn write(&mut self, text: &str) -> io::Result<()> {
        self.written.push_str(text);
        Ok(())
    }

    fn ask(&mut self, prompt: &str, echo: bool) -> io::Result<Option<Zeroizing<String>>> {
        self.write(prompt)?;
        self.asks.push((prompt.to_owned(), echo));
        Ok(self.answers.pop_front().flatten().map(Zeroizing::new))
    }
}

#[cfg(test)]
#[path = "terminal_tests.rs"]
mod tests;
