use core::fmt::{self, Write};
use core::mem::MaybeUninit;
use core::ptr;
use core::slice::{self, Iter};
use core::str;

#[derive(macros::NoPanicErrorDebug)]
pub enum Error {
    CapacityReached,
    BadSwapRemoveIndex(usize, usize),
    BadGetIndex(usize, usize),
}

pub struct List<T, const N: usize> {
    data: [MaybeUninit<T>; N],
    len: usize,
}

impl<T, const N: usize> List<T, N> {
    const UNINITIALIZED_VALUE: MaybeUninit<T> = MaybeUninit::uninit();

    pub const fn new() -> Self {
        Self {
            data: [Self::UNINITIALIZED_VALUE; N],
            len: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub const fn capacity(&self) -> usize {
        self.data.len()
    }

    pub fn clear(&mut self) {
        unsafe {
            let slice = ptr::slice_from_raw_parts_mut(self.data.as_mut_ptr() as *mut T, self.len);
            self.len = 0;
            ptr::drop_in_place(slice);
        }
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

    pub fn get_mut(&mut self, index: usize) -> Result<&mut T, Error> {
        if index < self.len {
            unsafe {
                Ok(self.get_unchecked_mut(index))
            }
        } else {
            Err(Error::BadGetIndex(index, self.len))
        }
    } 

    pub fn get(&self, index: usize) -> Result<&T, Error> {
        if index < self.len {
            unsafe {
                Ok(self.get_unchecked(index))
            }
        } else {
            Err(Error::BadGetIndex(index, self.len))
        }
    } 

    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut T {
        self.data.get_unchecked_mut(index).assume_init_mut()
    }

    pub unsafe fn get_unchecked(&self, index: usize) -> &T {
        self.data.get_unchecked(index).assume_init_ref()
    }

    pub fn as_slice(&self) -> &[T] {
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

    pub fn last_mut(&mut self) -> Option<&mut T> {
        if self.len > 0 {
            Some(unsafe { self.get_unchecked_mut(self.len - 1) })
        } else {
            None
        }
    }

    pub fn swap_remove(&mut self, index: usize) -> Result<T, Error> {
        let len = self.len;

        if index < len {
            unsafe {
                let last = ptr::read(self.data.as_ptr().add(len - 1));
                let hole = self.data.as_mut_ptr().add(index);
                self.len -= 1;
                Ok(ptr::replace(hole, last).assume_init())
            }
        } else {
            Err(Error::BadSwapRemoveIndex(index, len))
        }
    }
}

impl<T, const N: usize> Drop for List<T, N> {
    fn drop(&mut self) {
        unsafe {
            // Drop initialized `MaybeUninit<T>`s.
            ptr::drop_in_place(self.as_mut_slice());
        }
    }
}

impl<T, const N: usize> Default for List<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Write for List<u8, N> {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        self.write_bytes(s.as_bytes()).map_err(|_| fmt::Error)
    }
}

impl<const N: usize> List<u8, N> {
    pub fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
        if let Some(destination) = self.data.get_mut(self.len..self.len + bytes.len()) {
            // SAFETY: We already checked that the destination slice is valid for source length bytes.
            // Nonoverlapping because mutable references can't alias.
            unsafe {
                ptr::copy_nonoverlapping(
                    bytes.as_ptr().cast(),
                    destination.as_mut_ptr(),
                    destination.len(),
                );
            }
            self.len += destination.len();
            Ok(())
        } else {
            Err(Error::CapacityReached)
        }
    }
}

impl<const N: usize> AsRef<[u8]> for List<u8, N> {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}
