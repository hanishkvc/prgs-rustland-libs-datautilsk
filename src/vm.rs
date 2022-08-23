//!
//! A virtual program environment
//!
//! HanishKVC, 2022
//!

use std::collections::HashMap;
use std::rc::Rc;

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

    pub fn compile(prgfile: String) -> VM {

    }

}
