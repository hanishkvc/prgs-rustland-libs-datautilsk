//!
//! Some helpers to work with Vector of numeric values
//! HanishKVC, 2022
//!

use std::ops::{AddAssign, Div};


/// Find average of the passed vector of supported numeric values.
///
/// ALERT: Using u16 gives a nice compromise between supported vector length
/// and what data types can be averaged.
/// One can convert from u16 to f32 or f64.
/// Similarly one can convert from u16 to u32 or i32.
///
/// So it supports f32,f64,u32,i32,u16. But doesnt support u8,i8,i16.
///
pub fn vec_avg<T: AddAssign + From<u16> + Div<Output = T> + Copy>(vdata: &Vec<T>) -> T {
    let mut d = vdata[0];
    for i in 1..vdata.len() {
        d += vdata[i];
    }
    d/(vdata.len() as u16).into()
}

