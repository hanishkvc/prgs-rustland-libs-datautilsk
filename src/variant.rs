//!
//! A Variant data type
//! HanishKVC, 2022
//!

use std::time;

use crate::datautils;


#[derive(Debug)]
pub enum VDataType {
    Unknown,
    Integer,
    String,
    Buffer,
    Special,
}


#[derive(Debug, Clone)]
/// Maintain either a Integer or String or a Binary buffer
/// in a given variable of this type.
///
/// Inturn operate on them in a transparent way, to a great
/// extent, in that one can try to fetch the stored value
/// either has a integer or a string or a binary buffer.
/// The logic tries to convert the stored data into the
/// requested type in a predefined and potentially sane
/// way, which should be fine in many cases.
pub enum Variant {
    IntValue(isize),
    StrValue(String),
    BufValue(Vec<u8>),
    XTimeStamp,
}

impl Variant {

    pub fn get_type(&self) -> VDataType {
        match self {
            Variant::IntValue(_) => VDataType::Integer,
            Variant::StrValue(_) => VDataType::String,
            Variant::BufValue(_) => VDataType::Buffer,
            Variant::XTimeStamp => VDataType::Special,
        }
    }

    ///
    /// * Int -> Int
    /// * String -> Try interpret the string as a textual literal value of a integer
    /// * Buf -> Try interpret the buf as the underlying raw byte values of a integer
    /// * XTimeStamp -> milliseconds from UnixEpoch truncated
    ///
    pub fn get_isize(&self) -> Result<isize, String> {
        match self {
            Self::IntValue(ival) => {
                return Ok(*ival);
            },
            Self::StrValue(sval) => {
                let ival = datautils::intvalue(sval);
                if ival.is_ok() {
                    return Ok(ival.unwrap());
                }
                return Err(format!("Variant:GetISize:StrValue:[{}]:Conversion failed:{}", sval, ival.unwrap_err()));
            },
            Self::BufValue(bval) => {
                let bsval = bval.as_slice().try_into();
                if bsval.is_ok() {
                    let bval = bsval.unwrap();
                    return Ok(isize::from_ne_bytes(bval));
                }
                return Err(format!("Variant:GetISize:BufValue:[{:?}]:Adapting buf for int failed? Wrong number of bytes or?:{}", bval, bsval.unwrap_err()));
            },
            Self::XTimeStamp => {
                let ts = time::SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap();
                let uts = ts.as_millis();
                return Ok(uts as isize);
            },
        }
    }

    ///
    /// Return a unsigned (ie positive interger value), this is built upon
    /// get_isize. If the underlying value is negative, then it will panic
    ///
    #[allow(dead_code)]
    fn get_usize(&self) -> Result<usize, String> {
        let ival = self.get_isize();
        match ival {
            Ok(ival) => {
                if ival < 0 {
                    panic!("Variant:GetUSize: Negative int value not supported here")
                }
                return Ok(ival as usize);
            }
            Err(msg) => return Err(format!("Variant:GetUSize:{}", msg)),
        }
    }

    ///
    /// * Returns Int values as equivalent string literal form
    /// * Returns String as is
    /// * Returns Buf8 data as a hex string
    /// * XTimeStamp returns current System time converted to milliseconds since UNIX Epoch, as a string
    ///
    pub fn get_string(&self) -> String {
        match self {
            Self::IntValue(ival) => {
                return ival.to_string();
            },
            Self::StrValue(sval) => {
                return sval.to_string();
            },
            Self::BufValue(bval) => {
                return datautils::hex_from_vu8(bval);
            },
            Self::XTimeStamp => {
                let ts = time::SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap();
                let uts = ts.as_millis();
                return uts.to_string();
            },
         }
    }

    ///
    /// * returns int values as underlying byte values based vector in the native endianess format
    /// * Returns String as the underlying byte values based vector
    /// * Returns Buf8 data as is (rather a cloned buf)
    /// * XTimeStamp -> milliseconds from UnixEpoch, as the underlying byte values of the int
    ///
    /// TODO:ThinkAgain: Should I return a fixed endian format like network byte order (BigEndian) or little endian
    /// rather than native byte order (If testing between systems having different endianess, it could help)
    pub fn get_bufvu8(&self) -> Vec<u8> {
        match self {
            Self::IntValue(ival) => {
                return ival.to_ne_bytes().to_vec();
            },
            Self::StrValue(sval) => {
                return Vec::from(sval.to_string());
            },
            Self::BufValue(bval) => {
                return bval.clone();
            },
            Self::XTimeStamp => {
                let ts = time::SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap();
                let uts = ts.as_millis();
                return uts.to_ne_bytes().to_vec();
            },
         }
    }

    pub fn get_bufvu8_mut(&mut self) -> Option<&mut Vec<u8>> {
        if let Self::BufValue(thebuf) = self {
            return Some(thebuf.as_mut());
        }
        return None;
    }

    #[allow(dead_code)]
    /// Get the byte value at the given byte offset within the underlying/raw bytes
    /// of the data stored in the variant.
    pub fn get_byteelement(&self, index: usize) -> u8 {
        let bval = self.get_bufvu8();
        let aval = bval[index];
        return aval;
    }

    /// Get a appropriate data element at the given offset within the data.
    ///
    /// Int: the offset maps to byte offset
    /// String: the offset maps to char offset (and not byte offset)
    ///     should help with multibyte unicode chars which are stored internally.
    /// Buf: the offset maps to byte offset.
    pub fn get_arrayelement(&self, index: usize) -> Result<Variant, String> {
        match self {
            Self::IntValue(_ival) => {
                let bval = self.get_bufvu8();
                if index >= bval.len() {
                    return Err(format!("Variant:GetArrayEle:IntValue:Invalid index {}, available length {}", index, bval.len()));
                }
                return Ok(Variant::IntValue(bval[index] as isize));
            }
            Self::StrValue(sval) => {
                let cval = sval.chars().nth(index);
                let rval;
                if cval.is_none() {
                    return Err(format!("Variant:GetArrayEle:StrValue:Invalid index {}, beyond string", index));
                } else {
                    rval = cval.unwrap().to_string();
                    return Ok(Variant::StrValue(rval));
                }
            }
            Self::BufValue(bval) => {
                if index >= bval.len() {
                    return Err(format!("Variant:GetArrayEle:BufValue:Invalid index {}, available length {}", index, bval.len()));
                }
                let rval = &bval[index..index+1];
                return Ok(Variant::BufValue(rval.to_vec()));
            }
            _ => {
                let bval = self.get_bufvu8();
                if index >= bval.len() {
                    return Err(format!("Variant:GetArrayEle:{:?}:Invalid index {}, available length {}", self, index, bval.len()));
                }
                let rval = &bval[index..index+1];
                return Ok(Variant::BufValue(rval.to_vec()));
            }
        }
    }

}