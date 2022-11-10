//!
//! Some Integer data related utility functions
//!
//! HanishKVC, 2022
//!

use core::convert::From;
use std::num::ParseIntError;


///
/// Allow conversion btw isize and u8 through a minimal wrapper around u8
/// Additionally this allows conversion only if the isize value fits within u8 space
/// else it will panic with a error message.
/// This also helps make intvalue generic wrt the types I want (ie isize and u8 immidiately)
///

#[derive(Debug)]
pub struct U8X(pub u8);

impl Into<u8> for U8X {
    fn into(self) -> u8 {
        let U8X(u8val) = self;
        return u8val;
    }
}

impl From<isize> for U8X {
    fn from(ival: isize) -> Self {
        if (ival < 0) || (ival > u8::MAX.into()) {
            panic!("ERRR:DU:U8XFromISize:isize{} beyond u8 range", ival);
        }
        let uval = ival as usize;
        return U8X(uval as u8);
    }
}

///
/// Convert given string value to a isize, by treating it has a decimal
/// or hexdecimal (if starts with 0x) string value.
///
/// Inturn try convert the isize to specified type.
pub fn intvalue<T: std::convert::From<isize>>(sval: &str) -> Result<T, ParseIntError> {
    let sval = sval.trim();
    let ival;
    if sval.starts_with("0x") {
        ival = isize::from_str_radix(&sval[2..], 16)?;
    } else {
        ival = isize::from_str_radix(sval, 10)?;
    }
    return Ok(T::try_from(ival).unwrap());
}
