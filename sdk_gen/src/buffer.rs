use core::fmt::{self, Display, Formatter, Write};
use core::ops::Deref;

pub struct Buffer<const N: usize> {
    data: [u8; N],
    len: usize,
}

impl<const N: usize> Buffer<N> {
    pub fn new() -> Self {
        Self {
            data: [0; N],
            len: 0,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        if let Some(data) = self.data.get(..self.len) {
            data
        } else {
            crate::log!("unexpected: self.len ({}) is greater than self.data.len() ({})", self.len, self.data.len());
            &[]
        }
    }
}

impl<const N: usize> Deref for Buffer<N> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        unsafe { core::str::from_utf8_unchecked(self.as_bytes()) }
    }
}

impl<const N: usize> Display for Buffer<N> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        f.write_str(self)
    }
}

impl<const N: usize> Write for Buffer<N> {
    fn write_str(&mut self, source: &str) -> Result<(), fmt::Error> {
        let bytes = source.as_bytes();
        let num_bytes_to_write = bytes.len();
        let bytes_left = self.data.len() - self.len;

        if self.len >= self.data.len() || bytes_left < num_bytes_to_write {
            // Not great. Wish I had a way to return a custom error.
            crate::log!("error: bytes_left({}) < num_bytes_to_write({}) when trying to write \"{}\" into a Buffer<{}>.",
                bytes_left, num_bytes_to_write, source, N);
            return Err(fmt::Error::default());
        }

        let start = self.len;
        let end = start + num_bytes_to_write;

        if let Some(destination) = self.data.get_mut(start..end) {
            if destination.len() == bytes.len() {
                destination.copy_from_slice(bytes);
                self.len += num_bytes_to_write;
            } else {
                crate::log!("unexpected: destination.len() ({}) != bytes.len() ({})", destination.len(), bytes.len())
            }
        } else {
            crate::log!("unexpected: out-of-bounds start,end ({}, {}) in Buffer<{}>::write_str.", start, end, N);
        }

        Ok(())
    }
}