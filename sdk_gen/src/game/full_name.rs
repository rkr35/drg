use crate::list::{self, List};
use core::convert::TryFrom;

#[derive(macros::NoPanicErrorDebug)]
pub enum Error {
    NoSpaceBetweenClassAndOuters,
    NoName,
    List(#[from] list::Error),
}

pub struct FullName<'name, const NUM_OUTERS: usize> {
    pub name: &'name [u8],
    pub class: &'name [u8],
    pub outers: List<&'name [u8], NUM_OUTERS>,
}

fn split_class_and_outers(name: &str) -> Result<(&[u8], &[u8]), Error> {
    let name = name.as_bytes();
    let space = name.iter().position(|c| *c == b' ').ok_or(Error::NoSpaceBetweenClassAndOuters)?;
    Ok((&name[..space], &name[space+1..]))
}

struct OutersIterator<'name> {
    outers: &'name [u8],
}

impl<'name> Iterator for OutersIterator<'name> {
    type Item = &'name [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(split) = self.outers.iter().rposition(|c| *c == b'.') {
            // Elide panic branch. Technically undefined behavior if split == usize::MAX.
            #[allow(clippy::int_plus_one)]
            unsafe { crate::assert!(split+1 <= self.outers.len()); }

            // Return everything after the delimiter.
            let ret = &self.outers[split+1..];

            // Shrink the input up to and excluding the delimiter.
            self.outers = &self.outers[..split];
            
            Some(ret)
        } else if self.outers.is_empty() {
            // We've exhausted the input, and there's nothing else to return.
            None
        } else {
            // Return the remaining piece.
            let ret = self.outers;

            // Signal that we exhausted the input.
            self.outers = &[];
            
            Some(ret)
        }
    }
}

impl<'name, const NUM_OUTERS: usize> TryFrom<&'name str> for FullName<'name, NUM_OUTERS> {
    type Error = Error;

    fn try_from(name: &str) -> Result<FullName<NUM_OUTERS>, Self::Error> {
        let (class, outers) = split_class_and_outers(name)?;

        let mut list = List::<&[u8], NUM_OUTERS>::new();
        
        // Reverse split because outers are organized inside-out within an
        // object.
        let mut outers = OutersIterator { outers };

        // The first "outer" in the input name is actually the object name.
        let name = outers.next().ok_or(Error::NoName)?;

        for outer in outers {
            list.push(outer)?;
        }

        Ok(FullName {
            name,
            class,
            outers: list,
        })
    }
}