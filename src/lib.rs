//!
//! FuzzerK - A fuzzing helper library
//!
//! HanishKVC, 2022
//!


///
/// The trait that needs to be implemented by the different fuzzers
///
trait Fuzz {
    /// Generate the next fuzzed output and append to the passed buf
    /// The fuzzer can update itself if reqd
    fn append_fuzzed(&mut self, step: usize, buf: &mut Vec<u8>);

    /// Generate the next fuzzed output and append to the passed buf
    /// The fuzzer cant/doesnt update itself in this case.
    fn append_fuzzed_immut(&self, step: usize, buf: &mut Vec<u8>);
}

mod fixed;
mod random;
pub mod cfgfiles;


///
/// Allow a chain of muttable fuzzers (whose internal contexts can be modified) to be created,
/// so that byte buffer with the reqd pattern of data can be generated.
///
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

    /// Chain a muttable fuzzer, as part of setting up to achieve the reqd data pattern
    fn append(&mut self, fuzzer: &'a mut dyn Fuzz) {
        self.chain.push(fuzzer);
    }

    /// Get a byte buffer, whose data matches the pattern specified by the
    /// chain of Fuzzers in this FuzzChain instance
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


///
/// Allow a chain of immuttable fuzzers (whose internal contexts cant change) to be created,
/// so that byte buffer with the reqd pattern of data can be generated.
///
struct FuzzChainImmuts<'a> {
    chain: Vec<&'a dyn Fuzz>,
    step: usize,
}

impl<'a> FuzzChainImmuts<'a> {

    pub fn new() -> FuzzChainImmuts<'a> {
        FuzzChainImmuts {
            chain: Vec::new(),
            step: 0,
        }
    }

    /// Chain a immuttable fuzzer, as part of setting up to achieve the reqd data pattern
    fn append(&mut self, fuzzer: &'a dyn Fuzz) {
        self.chain.push(fuzzer);
    }

    /// Get a byte buffer, whose data matches the pattern specified by the
    /// chain of Fuzzers in this FuzzChainImmuts instance
    fn get(&mut self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::new();
        for i in 0..self.chain.len() {
            self.chain[i].append_fuzzed_immut(self.step, &mut buf)
        }
        self.step += 1;
        buf
    }

}



#[cfg(test)]
mod tests {
    use crate::{fixed::{self, RandomFixedStringsFuzzer}, random::{self, RandomFixedFuzzer}, Fuzz, FuzzChain, FuzzChainImmuts};

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

    #[test]
    fn fuzzchainimmuts_t2() {
        let mut fc1 = FuzzChainImmuts::new();
        let rfsf = RandomFixedStringsFuzzer::new(vec!["Hello".to_string(), "World".to_string()]);
        let rspacesf1 = RandomFixedStringsFuzzer::new(vec![" ".to_string(), "  ".to_string()]);
        let rspacesf2 = RandomFixedFuzzer::new(1, 5, vec![' ' as u8]);
        let rfpf = RandomFixedFuzzer::new_printables(3, 10);
        fc1.append(&rfsf);
        fc1.append(&rspacesf1);
        fc1.append(&rfpf);
        fc1.append(&rspacesf2);
        fc1.append(&rfpf); // The same fuzzer instance can be chained multiple times, if data pattern reqd dictates it.
        for i in 0..8 {
            let fuzzed = fc1.get();
            println!("TEST:FuzzChainImmutsT2:{}:{:?}:{:?}", i, fuzzed.clone(), String::from_utf8(fuzzed));
        }
    }

}
