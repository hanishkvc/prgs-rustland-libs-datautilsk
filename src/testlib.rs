//!
//! test code for some of the logics
//! HanishKVC, 2022
//!

use crate::variant::Variant;
use crate::hex;
use crate::sigpro;


pub fn test_variant() {
    let ivar = Variant::IntValue(5);
    let svar = Variant::StrValue("666".to_string());
    let bvar = Variant::BufValue(Vec::from("\x07\x00\x00\x00\x00\x00\x00\x00"));
    print!("TEST:Variant:Int:Int[{}]:String[{}]:Buf[{:?}]\n", ivar.get_isize().unwrap(), ivar.get_string(), ivar.get_bufvu8());
    print!("TEST:Variant:Str:Int[{}]:String[{}]:Buf[{:?}]\n", svar.get_isize().unwrap(), svar.get_string(), svar.get_bufvu8());
    print!("TEST:Variant:Buf:Int[{}]:String[{}]:Buf[{:?}]\n", bvar.get_isize().unwrap(), bvar.get_string(), bvar.get_bufvu8());
    let ivar = Variant::from("     123  ");
    let svar = Variant::from("  \" 456 but a string \"             ");
    let bvar = Variant::from("  $0x1122334455 ");
    let tvar = Variant::from("    __TIME__STAMP__ ");
    print!("TEST:Variant:UsingFrom:IntV[{}]:StrV[{}]:BufV[{}]:TSV[{}]\n", ivar, svar, bvar, tvar);
    let mut isvar = Variant::IntValue(123);
    let isorig = isvar.clone();
    isvar.set_string("Set a int variant to string variant");
    print!("TEST:Variant:Set:Initial[{}]:Set[{}]\n", isorig, isvar);
}

pub fn test_bufhex() {
    let mut vbuf = hex::vu8_from_hex("001122eeff00").unwrap();
    vbuf[0] = 99;
    let shex = hex::hex_from_vu8(&vbuf);
    print!("TEST:BufHex:vbuf[{:?}], shex[{}]\n", vbuf, shex);
}

pub fn test_vecavg() {
    let vtd11 = vec![1,2,3,4,5];
    let vtd12 = vec![1u32,2,3,4,5];
    let vtd13 = vec![1i32,2,3,4,5];
    let vtd21 = vec![1.1f32,2.1,3.1,4.1,5.1];
    let vtd22 = vec![1.1f64,2.1,3.1,4.1,5.1];
    eprintln!("TEST:VecAvg:{:?}:{}", vtd11, sigpro::vec_avg(&vtd11));
    eprintln!("TEST:VecAvg:{:?}:{}", vtd12, sigpro::vec_avg(&vtd12));
    eprintln!("TEST:VecAvg:{:?}:{}", vtd13, sigpro::vec_avg(&vtd13));
    eprintln!("TEST:VecAvg:{:?}:{}", vtd21, sigpro::vec_avg(&vtd21));
    eprintln!("TEST:VecAvg:{:?}:{}", vtd22, sigpro::vec_avg(&vtd22));
}
