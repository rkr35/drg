use core::fmt::{self, Display, Formatter};
use core::mem;
use core::str;

pub struct Hex(usize);

impl Display for Hex {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        const BASE: usize = 16;
        const PREFIX_LEN: usize = 2;
        const MAX_HEX_DIGITS: usize = 2 * mem::size_of::<usize>();
        const MAX_FORMATTED_LEN: usize = PREFIX_LEN + MAX_HEX_DIGITS;

        let mut buffer = [0; MAX_FORMATTED_LEN];
        let mut cursor = buffer.len();

        let mut n = self.0;

        for digit in (&mut buffer[PREFIX_LEN..]).iter_mut().rev() {
            cursor -= 1;

            *digit = match (n % BASE) as u8 {
                d @ 0..=9 => b'0' + d,
                d => b'a' + (d - 10),
            };

            n /= BASE;
            
            if n == 0 {
                break;
            }
        }

        cursor -= 2;
        buffer[cursor] = b'0';
        buffer[cursor + 1] = b'x';

        // SAFETY: We fill `buffer` with only characters in [0-9a-fx], which are ASCII.
        f.write_str(unsafe { str::from_utf8_unchecked(&buffer[cursor..]) })
    }
}