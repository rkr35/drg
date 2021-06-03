use core::fmt::{self, Write};

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

    pub fn as_ptr(&self) -> *const u8 {
        self.data.as_ptr()
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.data.as_mut_ptr()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub const fn capacity(&self) -> usize {
        N
    }

    pub fn advance(&mut self, n: usize) {
        self.len += n;
    }
}

fn unreachable() -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}

impl<const N: usize> Write for Buffer<N> {
    fn write_str(&mut self, source: &str) -> fmt::Result {
        let source = source.as_bytes();
        let max_write_bytes = N - self.len;
        let num_bytes_to_write = source.len().min(max_write_bytes);
        let start_write = self.len;
        let end_write = start_write + num_bytes_to_write;
        let destination_slice = self
            .data
            .get_mut(start_write..end_write)
            .unwrap_or_else(|| unreachable());
        let source_slice = &source[..num_bytes_to_write];
        assert!(destination_slice.len() == source_slice.len());
        destination_slice.copy_from_slice(source_slice);
        self.len += num_bytes_to_write;
        Ok(())
    }
}