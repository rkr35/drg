use crate::list::List;
use core::fmt::{self, Write};
use core::str;

pub struct BufWriter<W: Write> {
    writer: Option<W>,
    buffer: List<u8, 8192>,
}

impl<W: Write> BufWriter<W> {
    pub fn new() -> BufWriter<W> {
        BufWriter {
            writer: None,
            buffer: List::new(),
        }
    }

    pub fn with_writer(writer: W) -> BufWriter<W> {
        BufWriter {
            writer: Some(writer),
            buffer: List::new(),
        }
    }

    fn flush(&mut self) -> Result<(), fmt::Error> {
        if let Some(writer) = &mut self.writer {
            let s = unsafe {
                str::from_utf8_unchecked(self.buffer.as_slice())
            };
            writer.write_str(s)?;
        }
        self.buffer.clear();
        Ok(())
    }
}

impl<W: Write> Write for BufWriter<W> {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        let s = s.as_bytes();
        let mut cursor = 0;
        
        while cursor < s.len() {
            let mut space_left_in_buffer = self.buffer.capacity() - self.buffer.len();

            // Flush if buffer is full.
            if space_left_in_buffer == 0 {
                self.flush()?;
                space_left_in_buffer = self.buffer.capacity();
            }

            // Write min(space left in buffer, string bytes left to write) bytes to buffer.
            let num_bytes_to_write_now = space_left_in_buffer.min(s.len() - cursor);

            let piece = unsafe {
                // SAFETY: cursor slice range, at this point, is always [0, s.len()).
                // Proof:
                //   Base case: cursor left endpoint at iteration 0 is 0. cursor right endpoint is min(self.buffer.capacity(), s.len()), so always <= s.len().
                //   Inductive hypothesis (IH): cursors left and right endpoints at iteration k is within bounds.
                //   Inductive step: At iteration k+1, cursor can advance at most min(space_left_in_buffer, s.len() - cursor) bytes.
                //   CASE 1: Cursor advances `s.len() - cursor` bytes.
                //     Left endpoint is `cursor`. Per IH, left endpoint is within bounds.
                //     Right endpoint is `cursor + s.len() - cursor` = s.len(). Right endpoint is within bounds (second index in range is exclusive).
                //   CASE 2: Cursor advances space_left_in_buffer bytes.
                //     Left endpoint is `cursor`. Per IH, left endpoint is within bounds.
                //     Right endpoint is `cursor + space_left_in_buffer`. Since space_left_in_buffer <= s.len() - cursor, cursor + space_left_in_buffer <= s.len.
                s.get_unchecked(cursor..cursor + num_bytes_to_write_now)
            };

            self.buffer.write_bytes(piece).map_err(|_| fmt::Error)?;

            // Advance cursor to next position we'll read from.
            cursor += num_bytes_to_write_now;
        }

        Ok(())
    }
}

impl<W: Write> Drop for BufWriter<W> {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}