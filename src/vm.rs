//!
//! A virtual program environment
//!
//! HanishKVC, 2022
//!

use std::collections::HashMap;
use std::fs;

use crate::iob::IOBridge;


struct Context {
    strs: HashMap<String, String>,
    ints: HashMap<String, isize>,
    iobs: HashMap<String, IOBridge>,
    lbls: HashMap<String, usize>,
}

impl Context {
    fn new() -> Context {
        Context {
            strs: HashMap::new(),
            ints: HashMap::new(),
            iobs: HashMap::new(),
            lbls: HashMap::new(),
        }
    }
}


enum Op<'a> {
    None,
    LetStr(&'a mut Context, String, String),
    LetInt(&'a mut Context, String, isize),
    Inc(&'a mut Context, String),
    Dec(&'a mut Context, String),
    IobNew(&'a mut Context, String, String),
    IobWrite(&'a mut Context, String),
    IobFlush(&'a mut Context, String),
    IfLt(&'a mut Context, String, String, String, String),
}


struct VM<'a> {
    ctxt: Context,
    ops: Vec<Op<'a>>,
}

impl<'a> VM<'a> {

    fn new() -> VM<'a> {
        VM {
            ctxt: Context::new(),
            ops: Vec::new(),
        }
    }

    fn compile_letstr(&mut self, sargs: &str) {
        self.ops.push(Op::None);
    }

    fn compile0_label(&mut self, sargs: &str) {
        self.ctxt.lbls.insert(sargs.to_string(), self.ops.len()-1);
    }

    pub fn compile(ops: Vec<String>) -> VM<'a> {
        let vm = VM::new();
        for sop in ops {
            let sop = sop.trim();
            if sop.starts_with("#") {
                continue;
            }
        }
        vm
    }

    pub fn load_prg(prgfile: String) -> VM<'a> {
        let mut ops = Vec::<String>::new();
        let prgdata = fs::read_to_string(prgfile).expect("ERRR:FuzzerK:VM:Loading prg");
        let prgdata: Vec<&str> =  prgdata.split("\n").collect();
        for l in prgdata {
            ops.push(l.to_string());
        }
        Self::compile(ops)
    }


}
