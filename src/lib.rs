//!
//! FuzzerK - A fuzzing helper library
//!
//! HanishKVC, 2022
//!

use std::rc::Rc;
use std::cell::RefCell;


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

    /// Fuzzer can override this to indicate that it needs a mutable reference
    /// to itself, when fuzz chain will call its append_fuzzed
    fn need_mutable(&self) -> bool {
        false
    }
}

mod fixed;
mod random;
pub mod cfgfiles;
pub mod rtm;
mod datautils;
pub mod iob;
pub mod vm;


///
/// Allow a chain of muttable fuzzers (whose internal contexts can be modified) to be created,
/// so that byte buffer with the reqd pattern of data can be generated.
///
#[allow(dead_code)]
pub struct FuzzChain {
    chain: Vec<Rc<RefCell<dyn Fuzz>>>,
    /// step allows the fuzzer to know (if it wants to) that
    /// it is being called as part of the same step,
    /// if multiple instances of it are in the fuzz chain.
    step: usize,
}

#[allow(dead_code)]
impl FuzzChain {

    pub fn new() -> FuzzChain {
        FuzzChain {
            chain: Vec::new(),
            step: 0,
        }
    }

    /// Chain a muttable fuzzer, as part of setting up to achieve the reqd data pattern
    fn append(&mut self, fuzzer: Rc<RefCell<dyn Fuzz>>) {
        self.chain.push(fuzzer);
    }

    /// Get a byte buffer, whose data matches the pattern specified by the
    /// chain of Fuzzers in this FuzzChain instance
    ///
    /// step: indicates a new call wrt fuzz chain fuzzed data generation.
    ///       Calls to a fuzzer, where step is same, indicates that, that fuzzer has been chained more than once.
    ///       If specified, same will be passed to fuzzers, else the internally maintained step counter's value will be passed.
    fn get(&mut self, step: Option<usize>) -> Vec<u8> {
        let ustep;
        if step.is_none() {
            ustep = self.step;
        } else {
            ustep = step.unwrap();
        }
        let mut buf: Vec<u8> = Vec::new();
        for fuzzer in &mut self.chain {
            let imfuzzer = fuzzer.borrow();
            if imfuzzer.need_mutable() {
                let mut mfuzzer = fuzzer.borrow_mut();
                mfuzzer.append_fuzzed(ustep, &mut buf);
                drop(mfuzzer);
            } else {
                imfuzzer.append_fuzzed_immut(ustep, &mut buf);
            }
            drop(imfuzzer);
        }
        if step.is_none() {
            self.step += 1;
        } else {
            self.step = ustep;
        }
        buf
    }

}



#[cfg(test)]
mod tests {
    use crate::{fixed::{self, RandomFixedStringsFuzzer}, random::{self, RandomFixedFuzzer}, Fuzz, FuzzChain};
    use std::{rc::Rc, cell::RefCell};

    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }

    #[test]
    fn fuzzer_fixedstrings() {
        // LoopFixedStringsFuzzer
        let mut fsf = fixed::LoopFixedStringsFuzzer::new(vec![Vec::from("Hello"), Vec::from("World")]);
        let mut buf:Vec<u8> = Vec::new();
        for i in 0..16 {
            fsf.append_fuzzed(i, &mut buf)
        }
        println!("TEST:FuzzerLoopFixedStrings:{:?}", buf);
        println!("TEST:FuzzerLoopFixedStrings:{:?}", String::from_utf8(buf));
        // RandomFixedStringsFuzzer
        let mut fsf = fixed::RandomFixedStringsFuzzer::new(vec![Vec::from("Hello"), Vec::from("World")]);
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
        for _i in 0..size {
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

        let rfsf = RandomFixedStringsFuzzer::new(vec![Vec::from("Hello"), Vec::from("World")]);
        let rfsf = Rc::new(RefCell::new(rfsf));
        let rspacesf1 = RandomFixedStringsFuzzer::new(vec![Vec::from(" "), Vec::from("  ")]);
        let rspacesf1 = Rc::new(RefCell::new(rspacesf1));
        let rspacesf2 = RandomFixedFuzzer::new(1, 5, vec![' ' as u8]);
        let rspacesf2 = Rc::new(RefCell::new(rspacesf2));
        let rfpf = RandomFixedFuzzer::new_printables(3, 10);
        let rfpf = Rc::new(RefCell::new(rfpf));
        let rfpf2 = RandomFixedFuzzer::new_printables(3, 10);
        let rfpf2 = Rc::new(RefCell::new(rfpf2));

        fc1.append(rfsf);
        fc1.append(rspacesf1);
        fc1.append(rfpf.clone());
        fc1.append(rspacesf2);
        fc1.append(rfpf); // The same fuzzer instance can be chained multiple times, if data pattern reqd dictates it.
        fc1.append(rfpf2);
        for i in 0..8 {
            let fuzzed = fc1.get(None);
            println!("TEST:FuzzChainT1:{}:{:?}:{:?}", i, fuzzed.clone(), String::from_utf8(fuzzed));
        }
    }

}
