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

struct FuzzChain<'a> {
    chain: Vec<&'a mut dyn Fuzz>,
    step: usize,
}

impl<'a> FuzzChain<'a> {
    pub fn new() -> FuzzChain<'a> {
        FuzzChain {
            chain: Vec::new(),
            step: 0,
        }
    }

    fn append(&mut self, fuzzer: &'a mut dyn Fuzz) {
        self.chain.push(fuzzer);
    }

    fn get(&mut self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::new();
        /*
        for fuzzer in &mut self.chain {
            fuzzer.append_fuzzed(self.step, &mut buf)
        }
         */
        for i in 0..self.chain.len() {
            self.chain[i].append_fuzzed(self.step, &mut buf)
        }
        self.step += 1;
        buf
    }

}



#[cfg(test)]
mod tests {
    use crate::{fixed::{self, RandomFixedStringsFuzzer}, random::{self, RandomFixedFuzzer}, Fuzz, FuzzChain};

    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }

    #[test]
    fn fuzzer_fixedstrings() {
        // LoopFixedStringsFuzzer
        let mut fsf = fixed::LoopFixedStringsFuzzer::new(vec!["Hello".to_string(), "World".to_string()]);
        let mut buf:Vec<u8> = Vec::new();
        for i in 0..16 {
            fsf.append_fuzzed(i, &mut buf)
        }
        println!("TEST:FuzzerLoopFixedStrings:{:?}", buf);
        println!("TEST:FuzzerLoopFixedStrings:{:?}", String::from_utf8(buf));
        // RandomFixedStringsFuzzer
        let mut fsf = fixed::RandomFixedStringsFuzzer::new(vec!["Hello".to_string(), "World".to_string()]);
        let mut buf:Vec<u8> = Vec::new();
        for i in 0..16 {
            fsf.append_fuzzed(i, &mut buf)
        }
        println!("TEST:FuzzerRandomFixedStrings:{:?}", buf);
        println!("TEST:FuzzerRandomFixedStrings:{:?}", String::from_utf8(buf));
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

    fn gen_randbytes(size: usize) -> Vec<u8> {
        let mut randbytes = Vec::new();
        for i in 0..size {
            randbytes.push(rand::random());
        }
        randbytes
    }

    #[test]
    fn fuzzer_randomfixed() {
        const MINLEN: usize = 3;
        const MAXLEN: usize = 10;
        // RandomFixedFuzzer - binary
        let mut rfb = random::RandomFixedFuzzer::new(MINLEN, MAXLEN, gen_randbytes(128));
        let mut buf:Vec<u8> = Vec::new();
        for i in 0..16 {
            rfb.append_fuzzed(i, &mut buf);
            println!("TEST:FuzzerRandomFixed<Binary>:{}:BufLen:{}", i, buf.len());
        }
        println!("TEST:FuzzerRandomFixed<Binary>:{:?}", buf);
        // RandomFixedFuzzer - Printable chars
        buf.clear();
        let mut rfp = random::RandomFixedFuzzer::new_printables(MINLEN, MAXLEN);
        for i in 0..16 {
            rfp.append_fuzzed(i, &mut buf);
            println!("TEST:FuzzerRandomFixed<Printables>:{}:BufLen:{}", i, buf.len());
        }
        println!("TEST:FuzzerRandomFixed<Printables>:{:?}", String::from_utf8(buf));
    }

    #[test]
    fn fuzzchain_t1() {
        let mut fc1 = FuzzChain::new();
        let mut rfsf = RandomFixedStringsFuzzer::new(vec!["Hello".to_string(), "World".to_string()]);
        let mut rspacesf1 = RandomFixedStringsFuzzer::new(vec![" ".to_string(), "  ".to_string()]);
        let mut rspacesf2 = RandomFixedFuzzer::new(1, 5, vec![' ' as u8]);
        let mut rfpf = RandomFixedFuzzer::new_printables(3, 10);
        let mut rfpf2 = RandomFixedFuzzer::new_printables(3, 10);
        fc1.append(&mut rfsf);
        fc1.append(&mut rspacesf1);
        fc1.append(&mut rfpf);
        fc1.append(&mut rspacesf2);
        //fc1.append(&mut rfpf); // Cant do mutable borrow more than once
        fc1.append(&mut rfpf2);
        for i in 0..8 {
            let fuzzed = fc1.get();
            println!("TEST:FuzzChainT1:{}:{:?}:{:?}", i, fuzzed.clone(), String::from_utf8(fuzzed));
        }
    }

}
