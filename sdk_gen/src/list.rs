use core::mem::MaybeUninit;
use core::ptr;
use core::slice::{self, Iter};

#[derive(macros::NoPanicErrorDebug)]
pub enum Error {
    CapacityReached,
}

pub struct List<T, const N: usize> {
    data: [MaybeUninit<T>; N],
    len: usize,
}

impl<T, const N: usize> List<T, N> {
    const UNINITIALIZED_VALUE: MaybeUninit<T> = MaybeUninit::uninit();

    pub fn new() -> Self {
        Self {
            data: [Self::UNINITIALIZED_VALUE; N],
            len: 0,
        }
    }

    pub const fn capacity(&self) -> usize {
        self.data.len()
    }

    pub fn iter(&self) -> Iter<T> {
        self.as_slice().iter()
    }

    pub fn push(&mut self, value: T) -> Result<(), Error> {
        if self.len < self.capacity() {
            // Safe to use direct assignment since dropping a MaybeUninit<T> is a no-op.
            self.data[self.len] = MaybeUninit::new(value);
            self.len += 1;
            Ok(())
        } else {
            Err(Error::CapacityReached)
        }
    }

    fn as_slice(&self) -> &[T] {
        unsafe {
            // SAFETY: We ensure that &self.data[..self.len] contains initialized values.
            slice::from_raw_parts(self.data.as_ptr() as *const T, self.len)
        }
    }

    fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe {
            // SAFETY: We ensure that &self.data[..self.len] contains initialized values.
            slice::from_raw_parts_mut(self.data.as_mut_ptr() as *mut T, self.len)
        }
    }
}

impl<T, const NUM_OUTERS: usize> Drop for List<T, NUM_OUTERS> {
    fn drop(&mut self) {
        unsafe {
            // Drop initialized `MaybeUninit<T>`s.
            ptr::drop_in_place(self.as_mut_slice());
        }
    }
}
