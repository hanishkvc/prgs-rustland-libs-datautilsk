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

mod fixed;


#[cfg(test)]
mod tests {
    use crate::{fixed, Fuzz};

    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }

    #[test]
    fn test_fixedstringfuzzer() {
        let mut fsf = fixed::FixedStringsFuzzer::new(vec!["Hello".to_string(), "World".to_string()]);
        let mut buf:Vec<u8> = Vec::new();
        for i in 0..16 {
            fsf.append_fuzzed(i, &mut buf)
        }
    }
}
