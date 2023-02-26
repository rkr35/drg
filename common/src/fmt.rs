use core::fmt::{self, Display, Formatter};
use core::ops::{DivAssign, Rem};
use core::str;

pub trait Hexable: Copy + Rem<Output = Self> + DivAssign + From<u8> + PartialEq {
    const BASE: Self;
    const ZERO: Self;
    fn to_u8(self) -> u8;
}

macro_rules! impl_hexable {
    ($($t:ty)*) => {
        $(
            impl Hexable for $t {
                const BASE: Self = 16;
                const ZERO: Self = 0;
            
                fn to_u8(self) -> u8 {
                    self as u8
                }
            }
        )*
    }
}

impl_hexable! { i32 u8 usize }

pub struct Hex<T>(pub T);

impl<T: Hexable> Display for Hex<T> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        const PREFIX_LEN: usize = 2;
        const MAX_HEX_DIGITS: usize = 16; // TODO: i128/u128
        const MAX_FORMATTED_LEN: usize = PREFIX_LEN + MAX_HEX_DIGITS;

        let mut buffer = [0; MAX_FORMATTED_LEN];
        let mut cursor = buffer.len();

        let mut n = self.0;

        for digit in buffer[PREFIX_LEN..].iter_mut().rev() {
            cursor -= 1;

            *digit = match (n % T::BASE).to_u8() {
                d @ 0..=9 => b'0' + d,
                d => b'a' + (d - 10),
            };

            n /= T::BASE;
            
            if n == T::ZERO {
                break;
            }
        }

        cursor -= PREFIX_LEN;
        buffer[cursor] = b'0';
        buffer[cursor + 1] = b'x';

        // SAFETY: We fill `buffer` with only characters in [0-9a-fx], which are ASCII.
        // TODO(safety): negative numbers
        f.write_str(unsafe { str::from_utf8_unchecked(&buffer[cursor..]) })
    }
}

impl<T> Display for Hex<*mut T> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        Hex(self.0 as usize).fmt(f)
    }
}

impl<T> Display for Hex<*const T> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        Hex(self.0 as usize).fmt(f)
    }
}