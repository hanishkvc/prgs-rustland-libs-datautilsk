//!
//! Some helpers to work with Vector of numeric values
//! HanishKVC, 2022
//!

use std::ops::{AddAssign, Div};


/// Find average of the passed vector of numeric values
///
/// TODO: Currently the number of elements in the passed vector is limited
/// to be within 2^16
/// 
fn vec_avg<T: AddAssign + From<u16> + Div<Output = T> + Copy>(vdata: &Vec<T>) -> T {
    let mut d = vdata[0];
    for i in 1..vdata.len() {
        d += vdata[i];
    }
    d/(vdata.len() as u16).into()
}

#[allow(dead_code)]
fn vec_avg_f32(vdata: &Vec<f32>) -> f32 {
    let mut d = vdata[0];
    for i in 1..vdata.len() {
        d += vdata[i];
    }
    d/vdata.len() as f32
}

