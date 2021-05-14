use crate::win;
use core::fmt::{self, Write};
use core::ptr;
use log::{LevelFilter, Log, Metadata, Record, SetLoggerError};

macro_rules! static_assert {
    ($assertion:expr) => {
        const _ASSERT_BOOL: bool = $assertion;
        const _YOUR_STATIC_ASSERT_FAILED: u8 = _ASSERT_BOOL as u8;
        const _: u8 = _YOUR_STATIC_ASSERT_FAILED - 1;
    }
}

pub struct Logger;

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= LevelFilter::Info
    }

    fn log(&self, record: &Record) {
        const LOG_BUFFER: usize = 128;
        static_assert!(LOG_BUFFER < u32::MAX as usize);
        
        let mut buffer = Buffer::<LOG_BUFFER>::new();
        let _ = writeln!(&mut buffer, "{}", record.args());

        unsafe {
            // We statically assert that LOG_BUFFER < u32::MAX(), so we don't truncate.
            #[allow(clippy::cast_possible_truncation)]
            win::WriteConsoleA(
                win::GetStdHandle(win::STD_OUTPUT_HANDLE),
                buffer.as_ptr(),
                buffer.len() as u32,
                ptr::null_mut(),
                ptr::null_mut(),
            );
        }
    }

    fn flush(&self) {}
}

struct Buffer<const N: usize> {
    data: [u8; N],
    len: usize,
}

impl<const N: usize> Buffer<N> {
    fn new() -> Self {
        Self {
            data: [0; N],
            len: 0,
        }
    }

    fn as_ptr(&self) -> *const u8 {
        self.data.as_ptr()
    }

    fn len(&self) -> usize {
        self.len
    }
}

impl<const N: usize> Write for Buffer<N> {
    fn write_str(&mut self, source: &str) -> fmt::Result {
        let source = source.as_bytes();
        let max_write_bytes = N - self.len;
        let num_bytes_to_write = source.len().min(max_write_bytes);
        let start_write = self.len;
        let end_write = start_write + num_bytes_to_write;
        let destination_slice = &mut self.data[start_write..end_write];
        let source_slice = &source[..num_bytes_to_write];
        destination_slice.copy_from_slice(source_slice);
        self.len += num_bytes_to_write;
        Ok(())
    }
}

static LOGGER: Logger = Logger;

pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(&LOGGER)?;
    log::set_max_level(LevelFilter::Info);
    Ok(())
}
