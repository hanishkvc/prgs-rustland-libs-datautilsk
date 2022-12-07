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

/// Sliding window averaging over a given window size
///
/// The data is expected to be a vector of tuple of usize,f32,
/// inturn the f32 part will be averaged wrt/over specified sliding window size
///
/// Data at either end, which doesnt have sufficient elements on either side for
/// sliding window based averaging, is left, as is.
///
/// NOTE: A even window width/size will favor forward side bit more than backword side.
/// NOTE: This acts like a low pass filter to an extent.
///
pub fn sw_average_f_of_xf<M: Copy>(vdata: &Vec<(M, f32)>, ww: usize) -> Vec<(M, f32)> {
    let wwh = ww/2;
    let iwwh = wwh as isize;
    let weight = 1.0/(ww as f32);
    let mut vnew = Vec::new();
    let mut vbtw = Vec::new();
    for i in 0..vdata.len() {
        vbtw.push(vdata[i].1*weight);
    }
    for i in 0..wwh {
        vnew.push(vdata[i]);
    }
    let wsi;
    let wei;
    if (ww > 0) && (ww % 2 == 0) {
        wsi = -iwwh+1;
        wei = iwwh;
    } else {
        wsi = -iwwh;
        wei = iwwh;
    }
    for i in wwh..vbtw.len()-wwh {
        let mut d = 0.0;
        for j in wsi..=wei {
            let di = (i as isize + j) as usize;
            d += vbtw[di];
        }
        vnew.push((vdata[i].0, d));
    }
    for i in (1..=wwh).rev() {
        vnew.push(vdata[vdata.len()-i]);
    }
    vnew
}

/// Sliding window cross-correlation of given data with given weights
///
/// The data is expected to be a vector of tuple (AnyTypeSupportingCopy,f32),
/// inturn the f32 part will be cross-correlated with passed weights.
///
/// Datas at either end, which dont have enough elements on their either side to
/// apply given weights over them to find the cross-correlated values, is replaced
/// with value on either end, which can be computed fully wrt given weights vector.
///
pub fn sw_crosscorr_f_of_xf<M: Copy>(vdata: &Vec<(M, f32)>, vweights: &Vec<f32>) -> Vec<(M, f32)> {
    let ww = vweights.len();
    let wwh = (ww/2) as usize;
    let iwwh = wwh as isize;
    let mut vnew = Vec::new();
    // Initial placeholders
    for i in 0..wwh {
        vnew.push(vdata[i]);
    }
    // CrossCorrelated data
    for i in wwh..(vdata.len()-wwh) {
        let mut d = 0.0;
        for j in -iwwh..=iwwh {
            let wi = (j + iwwh) as usize;
            let di = (i as isize + j) as usize;
            d += vdata[di].1 * vweights[wi];
        }
        vnew.push((vdata[i].0, d/ww as f32));
    }
    // Extend data at begin
    for i in 0..wwh {
        vnew[i] = vnew[wwh];
    }
    // Extend data at end.
    let di = vdata.len() - wwh - 1;
    for _i in (1..=wwh).rev() {
        //eprintln!("DBUG:SdlX:CrossCorrWeighted:{:?}:{:?}:{:?}:{}",vdata, vweights, vnew, fi);
        vnew.push(vnew[di]);
    }
    vnew
}
