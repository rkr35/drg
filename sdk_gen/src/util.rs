#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        use crate::win;
        use core::fmt::{self, Write};
        use core::ptr;

        struct Stdout;

        impl Write for Stdout {
            fn write_str(&mut self, text: &str) -> Result<(), fmt::Error> { unsafe {
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

#[macro_export]
macro_rules! sdk_file {
    ($filename:literal) => {{
        concat!(sdk_path!(), '\\', $filename, '\0')
    }};
}

#[macro_export]
macro_rules! sdk_path {
    () => {
        include_str!(concat!(env!("OUT_DIR"), "/sdk_path"))
    };
}
