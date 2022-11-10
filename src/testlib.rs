//!
//! test code for some of the logics
//! HanishKVC, 2022
//!

use crate::variant;


pub fn test_variant() {
    let ivar = variant::Variant::IntValue(5);
    let svar = variant::Variant::StrValue("666".to_string());
    let bvar = variant::Variant::BufValue(Vec::from("\x07\x00\x00\x00\x00\x00\x00\x00"));
    print!("TEST:Variant:Int:Int[{}]:String[{}]:Buf[{:?}]\n", ivar.get_isize().unwrap(), ivar.get_string(), ivar.get_bufvu8());
    print!("TEST:Variant:Str:Int[{}]:String[{}]:Buf[{:?}]\n", svar.get_isize().unwrap(), svar.get_string(), svar.get_bufvu8());
    print!("TEST:Variant:Buf:Int[{}]:String[{}]:Buf[{:?}]\n", bvar.get_isize().unwrap(), bvar.get_string(), bvar.get_bufvu8());
}
