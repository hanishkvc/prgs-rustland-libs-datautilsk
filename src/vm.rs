//!
//! A virtual program environment
//!
//! HanishKVC, 2022
//!

use std::collections::HashMap;
use std::fs;

use loggerk::log_e;
use crate::iob::IOBridge;


struct Context {
    strs: HashMap<String, String>,
    ints: HashMap<String, isize>,
    iobs: HashMap<String, IOBridge>,
    lbls: HashMap<String, usize>,
    bufs: HashMap<String, Vec<u8>>,
}

impl Context {
    fn new() -> Context {
        Context {
            strs: HashMap::new(),
            ints: HashMap::new(),
            iobs: HashMap::new(),
            lbls: HashMap::new(),
            bufs: HashMap::new(),
        }
    }
}


enum Op {
    None,
    LetStr(String, String),
    LetInt(String, isize),
    Inc(String),
    Dec(String),
    IobNew(String, String, HashMap<String, String>),
    IobWrite(String, String),
    IobFlush(String),
    IobClose(String),
    IfLt(String, String, String, String),
}

impl Op {

    fn run(&self, ctxt: &mut Context) {
        match self {
            Self::None => (),
            Self::LetStr(vid, vval ) => {
                ctxt.strs.insert(vid.to_string(), vval.to_string());
            },
            Self::LetInt(vid, vval) => {
                ctxt.ints.insert(vid.to_string(), *vval);
            },
            Self::Inc(vid) => {
                let mut val = *ctxt.ints.get(vid).unwrap();
                val += 1;
                ctxt.ints.insert(vid.to_string(), val);
            }
            Self::Dec(vid) => {
                let mut val = *ctxt.ints.get(vid).unwrap();
                val -= 1;
                ctxt.ints.insert(vid.to_string(), val);
            }
            Self::IobNew(ioid, ioaddr, ioargs) => {
                let zenio = ctxt.iobs.get_mut(ioid);
                if zenio.is_some() {
                    let zenio = zenio.unwrap();
                    if let IOBridge::None = zenio {
                    } else {
                        let gotr = zenio.close();
                        if gotr.is_err() {
                            log_e(&format!("ERRR:FuzzerK:VM:Op:IobNew:Close4New:{}:{}", ioid, gotr.unwrap_err()));
                        }
                    }
                }
                let zenio = IOBridge::new(&ioaddr, &ioargs);
                ctxt.iobs.insert(ioid.to_string(), zenio);
            }
            Self::IobWrite(ioid, bufid) => {
                let buf = ctxt.bufs.get(bufid).unwrap();
                let zenio = ctxt.iobs.get_mut(ioid).unwrap();
                let gotr = zenio.write(buf);
                if gotr.is_err() {
                    log_e(&format!("ERRR:FuzzerK:VM:Op:IobWrite:{}:{}:{}", ioid, bufid, gotr.unwrap_err()));
                }
            }
            Self::IobFlush(ioid) => {
                let zenio = ctxt.iobs.get_mut(ioid).unwrap();
                let gotr = zenio.flush();
                if gotr.is_err() {
                    log_e(&format!("ERRR:FuzzerK:VM:Op:IobFlush:{}:{}", ioid, gotr.unwrap_err()));
                }
            }
            Self::IobClose(ioid) => {
                let zenio = ctxt.iobs.get_mut(ioid).unwrap();
                let gotr = zenio.close();
                if gotr.is_err() {
                    log_e(&format!("ERRR:FuzzerK:VM:Op:IobClose:{}:{}", ioid, gotr.unwrap_err()));
                }
                ctxt.iobs.remove(ioid);
            }
            _ => todo!(),
        }
    }

}


struct VM {
    ctxt: Context,
    ops: Vec<Op>,
}

impl VM {

    fn new() -> VM {
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

    pub fn compile(ops: Vec<String>) -> VM {
        let vm = VM::new();
        for sop in ops {
            let sop = sop.trim();
            if sop.starts_with("#") {
                continue;
            }
        }
        vm
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
