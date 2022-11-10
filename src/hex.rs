//!
//! Hex data related utility functions
//!
//! HanishKVC, 2022
//!

use core::convert::From;
use std::num::ParseIntError;


///
/// Routines to help convert between hex string and Vec<u8>
///


///
/// Convert hex string to Vec<u8>
///
pub fn vu8_from_hex(ins: &str) -> Result<Vec<u8>, String> {
    if ins.len() % 2 != 0 {
        return Err("ERRR:DU:Vu8FromHex:Hex string length not even, something wrong???".to_string());
    }
    let mut vu8 = Vec::new();
    for i in (0..ins.len()-1).step_by(2) {
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
    let hex = vec!["0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "A", "B", "C", "D", "E", "F"];
    let mut outs = String::new();
    for i in 0..inv.len() {
        let cu8 = inv[i];
        let bhigh = (cu8 & 0xF0) >> 4;
        let blow = cu8 & 0x0F;
        //log_d(&format!("DBUG:DU:HexFromVU8:{}+{}+{}", outs, bhigh, blow));
        outs.push_str(hex[bhigh as usize]);
        outs.push_str(hex[blow as usize]);
    }
    outs
}

