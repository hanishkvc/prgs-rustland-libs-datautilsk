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
mod random;


#[cfg(test)]
mod tests {
    use crate::{fixed, random, Fuzz};

    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }

    #[test]
    fn fuzzer_fixedstrings() {
        let mut fsf = fixed::FixedStringsFuzzer::new(vec!["Hello".to_string(), "World".to_string()]);
        let mut buf:Vec<u8> = Vec::new();
        for i in 0..16 {
            fsf.append_fuzzed(i, &mut buf)
        }
        println!("TEST:FuzzerFixedStrings:{:?}", buf);
        println!("TEST:FuzzerFixedStrings:{:?}", String::from_utf8(buf));
    }

    #[test]
    fn fuzzer_randomrandom() {
        const MINLEN: usize = 3;
        const MAXLEN: usize = 5;
        let mut rrf = random::RandomRandomFuzzer::new(MINLEN, MAXLEN);
        let mut buf:Vec<u8> = Vec::new();
        for i in 0..16 {
            rrf.append_fuzzed(i, &mut buf);
            println!("TEST:FuzzerRandomRandom:{}:BufLen:{}", i, buf.len());
        }
        println!("TEST:FuzzerRandomRandom:{:?}", buf);
    }
}
