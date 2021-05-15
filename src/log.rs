// macro_rules! static_assert {
//     ($assertion:expr) => {{
//         const _ASSERT_BOOL: bool = $assertion;
//         const _YOUR_STATIC_ASSERT_FAILED: u8 = _ASSERT_BOOL as u8;
//         const _: u8 = _YOUR_STATIC_ASSERT_FAILED - 1;
//     }};
// }

macro_rules! log {
    ($($arg:tt)*) => {{
        use crate::win;
        use core::fmt::{self, Write};
        use core::ptr;

        struct Stdout;

        impl Write for Stdout {
            fn write_str(&mut self, text: &str) -> fmt::Result { unsafe {
                #[allow(clippy::cast_possible_truncation)]
                win::WriteConsoleA(
                    win::GetStdHandle(win::STD_OUTPUT_HANDLE),
                    text.as_ptr(),
                    text.len() as u32,
                    ptr::null_mut(),
                    ptr::null_mut(),
                );

                Ok(())
            }}
        }
        
        let _ = writeln!(&mut Stdout, $($arg)*);
    }}
}

