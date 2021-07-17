use crate::list::{self, List};
use crate::split::ReverseSplitIterator;
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
    let space = name
        .iter()
        .position(|c| *c == b' ')
        .ok_or(Error::NoSpaceBetweenClassAndOuters)?;
    Ok((&name[..space], &name[space + 1..]))
}

impl<'name, const NUM_OUTERS: usize> TryFrom<&'name str> for FullName<'name, NUM_OUTERS> {
    type Error = Error;

    fn try_from(full_name: &str) -> Result<FullName<NUM_OUTERS>, Self::Error> {
        let (class, outers) = split_class_and_outers(full_name)?;

        // Reverse split because outers are organized inside-out within an
        // object.
        let mut outers = ReverseSplitIterator::new(outers, b'.');

        // The first "outer" in the input name is actually the object name.
        let name = outers.next().ok_or(Error::NoName)?;

        let mut list = List::new();

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
