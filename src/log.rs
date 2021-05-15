macro_rules! static_assert {
    ($assertion:expr) => {{
        const _ASSERT_BOOL: bool = $assertion;
        const _YOUR_STATIC_ASSERT_FAILED: u8 = _ASSERT_BOOL as u8;
        const _: u8 = _YOUR_STATIC_ASSERT_FAILED - 1;
    }};
}

macro_rules! log {
    ($($arg:tt)*) => {{
        use crate::buffer::Buffer;
        use crate::win;
        use core::fmt::Write;
        use core::ptr;

        const LOG_BUFFER: usize = 128;
        static_assert!(LOG_BUFFER < u32::MAX as usize);

        let mut buffer = Buffer::<LOG_BUFFER>::new();
        let _ = writeln!(&mut buffer, $($arg)*);

        // We statically assert that LOG_BUFFER < u32::MAX(), so we don't truncate.
        #[allow(clippy::cast_possible_truncation)]
        win::WriteConsoleA(
            win::GetStdHandle(win::STD_OUTPUT_HANDLE),
            buffer.as_ptr(),
            buffer.len() as u32,
            ptr::null_mut(),
            ptr::null_mut(),
        );
    }}
}
