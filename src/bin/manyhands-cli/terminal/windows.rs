use super::{LineEditor, Step, Terminal, Zeroizing};
use std::{
    io,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    ptr::{null, null_mut},
};
use windows_sys::Win32::{
    Foundation::{GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE},
    Storage::FileSystem::{CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING},
    System::Console::{
        ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT, FlushConsoleInputBuffer,
        GetConsoleMode, ReadConsoleW, SetConsoleMode, WriteConsoleW,
    },
};
struct Console {
    input: OwnedHandle,
    output: OwnedHandle,
}
fn device(name: &str) -> Option<OwnedHandle> {
    let name: Vec<u16> = name.encode_utf16().chain([0]).collect();
    let access = GENERIC_READ | GENERIC_WRITE;
    let share = FILE_SHARE_READ | FILE_SHARE_WRITE;
    // SAFETY: name is NUL-terminated; null security/template means no inheritance.
    let handle = unsafe {
        CreateFileW(
            name.as_ptr(),
            access,
            share,
            null(),
            OPEN_EXISTING,
            0,
            null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return None;
    }
    // SAFETY: CreateFileW returned a fresh owned handle, closed exactly once.
    Some(unsafe { OwnedHandle::from_raw_handle(handle) })
}
pub fn open() -> Option<Box<dyn Terminal>> {
    Some(Box::new(Console {
        input: device("CONIN$")?,
        output: device("CONOUT$")?,
    }))
}
fn checked(result: i32) -> io::Result<()> {
    (result != 0)
        .then_some(())
        .ok_or_else(io::Error::last_os_error)
}
struct RestoreMode {
    handle: HANDLE,
    saved: u32,
}
impl Drop for RestoreMode {
    fn drop(&mut self) {
        // SAFETY: ask keeps the input handle open until after this guard drops.
        let _ = unsafe { SetConsoleMode(self.handle, self.saved) };
    }
}
impl Console {
    fn unit(&self) -> io::Result<Option<Zeroizing<u16>>> {
        let mut unit = Zeroizing::new(0u16);
        let mut count = 0;
        let handle = self.input.as_raw_handle();
        let buffer = (&mut *unit as *mut u16).cast();
        // SAFETY: input is open; unit and count are writable for the requested unit.
        checked(unsafe { ReadConsoleW(handle, buffer, 1, &mut count, null()) })?;
        Ok((count != 0).then_some(unit))
    }
}

impl Terminal for Console {
    fn write(&mut self, text: &str) -> io::Result<()> {
        let wide: Vec<u16> = text.encode_utf16().collect();
        let mut rest = wide.as_slice();
        while !rest.is_empty() {
            let mut count = 0;
            let handle = self.output.as_raw_handle();
            let length = rest.len().min(u32::MAX as usize) as u32;
            // SAFETY: output is open; rest has the requested units; count is writable.
            checked(unsafe { WriteConsoleW(handle, rest.as_ptr(), length, &mut count, null()) })?;
            if count == 0 {
                return Err(io::ErrorKind::WriteZero.into());
            }
            rest = &rest[count as usize..];
        }
        Ok(())
    }

    fn ask(&mut self, prompt: &str, echo: bool) -> io::Result<Option<Zeroizing<String>>> {
        let handle = self.input.as_raw_handle();
        let mut saved = 0;
        // SAFETY: handle is open and saved is writable mode storage.
        checked(unsafe { GetConsoleMode(handle, &mut saved) })?;
        let _restore = RestoreMode { handle, saved };
        let reading = saved & !(ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT | ENABLE_PROCESSED_INPUT);
        // SAFETY: handle stays open; flags are valid console input-mode flags.
        checked(unsafe { SetConsoleMode(handle, reading) })?;
        // SAFETY: handle is the open console input buffer, before the prompt.
        checked(unsafe { FlushConsoleInputBuffer(handle) })?;
        self.write(prompt)?;
        let mut editor = LineEditor::new(echo);
        let mut invalid = false;
        let line = 'read: loop {
            let Some(first) = self.unit()? else {
                break None;
            };
            let mut units = Zeroizing::new([*first, 0]);
            let length = if (0xd800..=0xdbff).contains(&units[0]) {
                let Some(second) = self.unit()? else {
                    break None;
                };
                units[1] = *second;
                2
            } else {
                1
            };
            let Some(Ok(character)) = char::decode_utf16(units[..length].iter().copied()).next()
            else {
                invalid = true;
                break None;
            };
            let mut utf8 = Zeroizing::new([0u8; 4]);
            let mut drawn = Vec::new();
            for byte in character.encode_utf8(&mut *utf8).bytes() {
                match editor.feed(byte) {
                    Step::Pending(bytes) => drawn.extend(bytes),
                    Step::Cancel => break 'read None,
                    Step::Done => {
                        let line = editor.finish();
                        invalid = line.is_none();
                        break 'read line;
                    }
                }
            }
            // Echo is non-secret; collect a whole character before UTF-16 conversion.
            if let Ok(text) = std::str::from_utf8(&drawn) {
                self.write(text)?;
            }
        };
        self.write("\r\n")?;
        if invalid {
            self.write("Input is not valid Unicode; cancelled.\r\n")?;
        }
        Ok(line)
    }
}
