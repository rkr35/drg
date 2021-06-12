#[derive(macros::NoPanicErrorDebug)]
pub enum Error {
    CapacityReached,
}

pub struct List<T: Copy + Default, const N: usize> {
    data: [T; N],
    len: usize,
}

impl<T: Copy + Default, const N: usize> List<T, N> {
    pub fn new() -> Self {
        Self { 
            data: [T::default(); N],
            len: 0,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item=&T> {
        self.data.iter().take(self.len)
    }

    pub fn add(&mut self, value: T) -> Result<(), Error> {
        if self.len < self.data.len() {
            self.data[self.len] = value;
            self.len += 1;
            Ok(())
        } else {
            Err(Error::CapacityReached)
        }
    }
}
