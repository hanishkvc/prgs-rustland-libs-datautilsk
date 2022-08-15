//!
//! FuzzerK - A fuzzing library
//!
//! HanishKVC, 2022
//!


///
/// The trait that needs to be implemented by the different fuzzers
///
trait Fuzz {
    /// Generate the next fuzzed output and append to the passed buf
    fn append_fuzzed(&mut self, step: usize, buf: &mut Vec<u8>);
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
