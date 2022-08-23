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


struct Prg {
    ctxt: Context,
    ops: Vec<Op>,
}

impl Prg {

    fn compile_letstr(&mut self, sargs: &str) -> Op {
        Op::None
    }

}
