use core::fmt::{self, Write};

#[derive(macros::NoPanicErrorDebug)]
pub enum Error {
    CreateFile,
}

pub struct File {
    handle: *mut core::ffi::c_void,
}

impl File {
    pub unsafe fn new<T: AsRef<[u8]>>(name: T) -> Result<Self, Error> {
        const INVALID_HANDLE_VALUE: usize = usize::MAX;
        const GENERIC_WRITE: u32 = 0x4000_0000;
        const FILE_SHARE_READ: u32 = 1;
        const CREATE_ALWAYS: u32 = 2;
        const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;

        let handle = super::CreateFileA(
            name.as_ref().as_ptr(),
            GENERIC_WRITE,
            FILE_SHARE_READ,
            core::ptr::null_mut(),
            CREATE_ALWAYS,
            FILE_ATTRIBUTE_NORMAL,
            core::ptr::null_mut(),
        );

        if handle as usize == INVALID_HANDLE_VALUE {
            return Err(Error::CreateFile);
        }

        Ok(Self { handle })
    }
}

impl Drop for File {
    fn drop(&mut self) {
        unsafe {
            super::FlushFileBuffers(self.handle);
            super::CloseHandle(self.handle);
        }
    }
}

impl Write for File {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        unsafe {
            let mut num_written = 0;

            #[allow(clippy::cast_possible_truncation)]
            let result = super::WriteFile(
                self.handle,
                s.as_ptr(),
                s.len() as u32,
                &mut num_written,
                core::ptr::null_mut(),
            );

            if result == 0 {
                Err(fmt::Error::default())
            } else {
                Ok(())
            }
        }
    }
}
