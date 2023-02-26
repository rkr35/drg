#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        use core::fmt::Write;

        struct Stdout;

        impl Write for Stdout {
            fn write_str(&mut self, text: &str) -> Result<(), core::fmt::Error> { unsafe {
                #[allow(clippy::cast_possible_truncation)]
                $crate::win::WriteConsoleA(
                    $crate::win::GetStdHandle($crate::win::STD_OUTPUT_HANDLE),
                    text.as_ptr(),
                    text.len() as u32,
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                );

                Ok(())
            }}
        }

        let _ = writeln!(&mut Stdout, $($arg)*);
    }}
}

pub fn align(x: usize, alignment: usize) -> usize {
    (x + alignment - 1) & !(alignment - 1)
}
