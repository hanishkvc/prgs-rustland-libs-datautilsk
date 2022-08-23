//!
//! A virtual program environment
//!
//! HanishKVC, 2022
//!

use std::collections::HashMap;
use std::rc::Rc;
use std::fs;

use crate::iob::IOBridge;

struct Context {
    strs: HashMap<String, String>,
    ints: HashMap<String, isize>,
    iobs: HashMap<String, IOBridge>,
    lbls: HashMap<String, usize>,
}

enum Op {
    None,
    LetStr(Rc<Context>, String, String),
    LetInt(Rc<Context>, String, isize),
    Inc(Rc<Context>, String),
    Dec(Rc<Context>, String),
    IobNew(Rc<Context>, String, String),
    IobWrite(Rc<Context>, String),
    IobFlush(Rc<Context>, String),
    IfLt(Rc<Context>, String, String, String, String),
}


struct VM {
    ctxt: Rc<Context>,
    ops: Vec<Op>,
}

impl VM {

    fn compile_letstr(&mut self, sargs: &str) {
        self.ops.push(Op::None);
    }

    fn compile0_label(&mut self, sargs: &str) {
        self.ctxt.lbls.insert(sargs.to_string(), self.ops.len()-1);
    }

    pub fn compile(ops: Vec<String>) -> VM {
        for sop in ops {
            
        }

    }

    pub fn load_prg(prgfile: String) -> VM {
        let mut ops = Vec::<String>::new();
        let prgdata = fs::read_to_string(prgfile).expect("ERRR:FuzzerK:VM:Loading prg");
        let prgdata: Vec<&str> =  prgdata.split("\n").collect();
        for l in prgdata {
            ops.push(l.to_string());
        }
        Self::compile(ops)
    }


}
