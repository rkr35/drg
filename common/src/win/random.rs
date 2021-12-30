use core::mem;
use core::ptr;


pub fn u32() -> u32 {
    const BCRYPT_USE_SYSTEM_PREFERRED_RNG: u32 = 2;

    let mut buffer = [0; mem::size_of::<u32>()];
    
    unsafe {
        super::BCryptGenRandom(ptr::null_mut(), buffer.as_mut_ptr(), buffer.len() as u32, BCRYPT_USE_SYSTEM_PREFERRED_RNG);
    }

    u32::from_le_bytes(buffer)
}