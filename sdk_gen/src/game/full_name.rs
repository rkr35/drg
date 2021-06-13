use crate::list::{self, List};
use core::convert::TryFrom;

#[derive(macros::NoPanicErrorDebug)]
pub enum Error {
    MissingSpaceBetweenClassAndOuters,
    MissingName,
    List(#[from] list::Error),
}

pub struct FullName<'name, const NUM_OUTERS: usize> {
    pub class: &'name [u8],
    pub name: &'name [u8],
    pub outers: List<&'name [u8], NUM_OUTERS>,
}

fn split_class_and_outers(name: &str) -> Result<(&[u8], &[u8]), Error> {
    let name = name.as_bytes();
    let space = name.iter().position(|&c| c == b' ').ok_or(Error::MissingSpaceBetweenClassAndOuters)?;
    Ok((&name[..space], &name[space+1..]))
}

struct OutersIterator<'name> {
    v: &'name [u8]
}

// Non-panicking alternative to `slice::rsplit`.
impl<'name> Iterator for OutersIterator<'name> {
    type Item = &'name [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(split) = self.v.iter().rposition(|c| *c == b'.') {
            // Elide panic branch. Technically undefined behavior if split ==
            // usize::MAX since split+1 will overflow in an undefined way.
            unsafe { crate::assert!(split+1 <= self.v.len()); }

            // Return everything after the split.
            let ret = &self.v[split+1..];

            // Shrink the remaining portion up to and excluding the split.
            self.v = &self.v[..split];

            Some(ret)
        } else if self.v.is_empty() {
            // No remaining pieces. Iteration is done.
            None
        } else {
            // Return the remaining piece.
            let ret = self.v;

            // Clear saved piece to signal iteration end.
            self.v = &[];

            Some(ret)
        }
    }
}

impl<'name, const NUM_OUTERS: usize> TryFrom<&'name str> for FullName<'name, NUM_OUTERS> {
    type Error = Error;

    fn try_from(name: &str) -> Result<FullName<NUM_OUTERS>, Self::Error> {
        // "Class Outer3.Outer2.Outer1.Name"

        // ("Class", "Outer3.Outer2.Outer1.Name")
        let (class, outers) = split_class_and_outers(name)?;
        
        let mut list = List::<&[u8], NUM_OUTERS>::new();
        
        let mut outers = OutersIterator { v: outers };

        // The first "outer" in the input name is actually the object name.
        // "Name"
        let name = outers.next().ok_or(Error::MissingName)?;

        // ["Outer1", "Outer2", "Outer3"]
        for outer in outers {
            list.add(outer)?;
        }

        Ok(FullName {
            class, // "Class"
            name, // "Name"
            outers: list, // ["Outer1", "Outer2", "Outer3"]
        })
    }
}