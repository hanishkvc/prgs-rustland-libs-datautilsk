//!
//! Some data related utility functions
//!
//! HanishKVC, 2022
//!

use core::convert::From;


pub fn vu8_from_hex(ins: &str) -> Result<Vec<u8>, String> {
    let mut vu8 = Vec::new();
    for i in (0..ins.len()).step_by(2) {
        let cu8 = u8::from_str_radix(&ins[i..i+2], 16);
        if cu8.is_err() {
            return Err(format!("ERRR:DU:VU8FromHex:{}>>{}<<:{}", ins, &ins[i..i+2], cu8.unwrap_err()));
        }
        vu8.push(cu8.unwrap());
    }
    Ok(vu8)
}

#[derive(Debug)]
pub struct U8X(pub u8);

impl TryInto<u8> for U8X {
    type Error = String;
    fn try_into(self) -> Result<u8, Self::Error> {
        if let U8X(u8val) = self {
            return Ok(u8val);
        } else {
            return Err(format!("ERRR:FuzzerK:DataUtils:U8X:TryInto u8:{:?}", self));
        }
    }
}

impl From<isize> for U8X {
    fn from(ival: isize) -> Self {
        if (ival < 0) || (ival > u8::MAX.into()) {
            panic!();
        }
        let uval = ival as usize;
        return U8X(uval as u8);
    }
}

pub fn intvalue<T: std::convert::From<isize>>(sval: &str, exceptmsg: &str) -> T {
    let sval = sval.trim();
    let ival;
    if sval.starts_with("0x") {
        ival = isize::from_str_radix(sval, 16).expect(exceptmsg);
    } else {
        ival = isize::from_str_radix(sval, 10).expect(exceptmsg);
    }
    return T::try_from(ival).unwrap();
}
