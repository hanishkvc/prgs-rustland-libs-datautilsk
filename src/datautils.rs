//!
//! Some data related utility functions
//!
//! HanishKVC, 2022
//!

use core::convert::From;

///
/// Routines to help convert between hex string and Vec<u8>
///


///
/// Convert hex string to Vec<u8>
///
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

///
/// Convert Vec<u8> to hex string
///
pub fn hex_from_vu8(inv: &Vec<u8>) -> String {
    let mut outs = String::new();
    for i in 0..inv.len() {
        let cu8 = inv[i];
        let bhigh = (cu8 & 0xF0) >> 4;
        let blow = cu8 & 0x0F;
        outs.push_str(&bhigh.to_string());
        outs.push_str(&blow.to_string());
    }
    outs
}


///
/// Remove extra space (ie beyond a single space) outside double quoted text in a line.
/// Any whitespace inbetween a begin and end double quote, will be retained.
///
/// Outside double quoted text, \ is not treated has a escape sequence marker.
/// Inside double quoted text, \ is treated has a escape sequence marker, and the char next to it,
/// will be treated has a normal char and not treated has special, even if it is " or \.
///
pub fn remove_extra_whitespaces(ins: &str) -> String {
    let mut outs = String::new();
    let mut besc = false;
    let mut binquotes = false;
    let mut bwhitespace = false;
    let incv: Vec<char> = ins.chars().collect();
    for i in 0..incv.len() {
        let c = incv[i];

        if c.is_whitespace() {
            if binquotes {
                outs.push(c);
            } else {
                if !bwhitespace {
                    bwhitespace = true;
                    outs.push(' ');
                }
            }
            continue;
        }
        bwhitespace = false;
        outs.push(c);

        if besc {
            besc = false;
            continue;
        }

        if c == '"' {
            if binquotes {
                binquotes = false;
            } else {
                binquotes = true;
            }
            continue;
        }

        if c == '\\' {
            if binquotes {
                besc = true;
            }
            continue;
        }
    }
    outs
}


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
pub fn intvalue<T: std::convert::From<isize>>(sval: &str, exceptmsg: &str) -> T {
    let sval = sval.trim();
    let ival;
    if sval.starts_with("0x") {
        ival = isize::from_str_radix(&sval[2..], 16).expect(exceptmsg);
    } else {
        ival = isize::from_str_radix(sval, 10).expect(exceptmsg);
    }
    return T::try_from(ival).unwrap();
}
