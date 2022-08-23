//!
//! A virtual program environment
//!
//! HanishKVC, 2022
//!

use std::collections::HashMap;
use std::fs;
use std::thread;
use std::time::Duration;

use loggerk::{log_e, log_d};
use crate::iob::IOBridge;
use crate::rtm::RunTimeManager;
use crate::cfgfiles;


struct Context {
    strs: HashMap<String, String>,
    ints: HashMap<String, isize>,
    iobs: HashMap<String, IOBridge>,
    lbls: HashMap<String, usize>,
    bufs: HashMap<String, Vec<u8>>,
    stepu: usize,
    fcrtm: RunTimeManager,
    iptr: usize,
    iptrupdate: bool,
}

impl Context {
    fn new() -> Context {
        Context {
            strs: HashMap::new(),
            ints: HashMap::new(),
            iobs: HashMap::new(),
            lbls: HashMap::new(),
            bufs: HashMap::new(),
            stepu: 0,
            fcrtm: RunTimeManager::new(),
            iptr: 0,
            iptrupdate: true,
        }
    }
}


#[derive(Debug)]
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
    SleepMSec(u64),
    FcGet(String, String),
}

impl Op {

    fn compile(sop: &str) -> Result<Op, String> {
        Ok(Self::None)
    }

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
                let mut val = *ctxt.ints.get(vid).expect(&format!("ERRR:FuzzerK:VM:Op:Inc:{}", vid));
                val += 1;
                ctxt.ints.insert(vid.to_string(), val);
            }
            Self::Dec(vid) => {
                let mut val = *ctxt.ints.get(vid).expect(&format!("ERRR:FuzzerK:VM:Op:Dec:{}", vid));
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
                let buf = ctxt.bufs.get(bufid).expect(&format!("ERRR:FuzzerK:VM:Op:IobWrite:FromBuf:{}", bufid));
                let zenio = ctxt.iobs.get_mut(ioid).expect(&format!("ERRR:FuzzerK:VM:Op:IobWrite:{}", ioid));
                let gotr = zenio.write(buf);
                if gotr.is_err() {
                    log_e(&format!("ERRR:FuzzerK:VM:Op:IobWrite:{}:FromBuf:{}:{}", ioid, bufid, gotr.unwrap_err()));
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
            Self::SleepMSec(msec) => {
                thread::sleep(Duration::from_millis(*msec));
            }
            Self::FcGet(fcid, bufid) => {
                let fci = ctxt.fcrtm.fcimmuts(&fcid).unwrap();
                let gotfuzz = fci.get(ctxt.stepu);
                log_d(&format!("\n\nGot:{}:\n\t{:?}\n\t{}", ctxt.stepu, gotfuzz, String::from_utf8_lossy(&gotfuzz)));
                ctxt.bufs.insert(bufid.to_string(), gotfuzz);
                ctxt.stepu += 1;
            }
            Self::IfLt(sval, vid, sop , oparg) => {
                let chkval = isize::from_str_radix(sval, 10).expect(&format!("ERRR:FuzzerK:VM:Op:IfLt:ChkVal:{}:Conversion", sval));
                let curval = *ctxt.ints.get(vid).expect(&format!("ERRR:FuzzerK:VM:Op:IfLt:Var:{}", vid));
                let mut opdo = false;
                if chkval < curval {
                    opdo = true;
                }
                if opdo {
                    if sop == "goto" {
                        // Translating the label here at runtime, rather than during compile time, allows goto to refer to label
                        // that might not yet have been defined at the point where goto or rather the IfLt is encountered.
                        // Especially when only a single pass parsing of the program is done.
                        ctxt.iptr = *ctxt.lbls.get(oparg).expect(&format!("ERRR:FuzzerK:VM:Op:IfLt:GoTo:Label:{}", oparg));
                        ctxt.iptrupdate = false;
                    }
                }
            }
        }
    }

}


pub struct VM {
    ctxt: Context,
    ops: Vec<Op>,
}

impl VM {

    pub fn new() -> VM {
        VM {
            ctxt: Context::new(),
            ops: Vec::new(),
        }
    }

    fn compile_directive(&mut self, sdirplus: &str) {
        let (sdir, sargs) = sdirplus.split_once(' ').expect(&format!("ERRR:FuzzerK:VM:CompileDirective:{}", sdirplus));
        if sdir == "!label" {
            self.ctxt.lbls.insert(sargs.to_string(), self.ops.len()-1);
        } else {
            panic!("ERRR:FuzzerK:VM:CompileDirective:Unknown:{}", sdirplus);
        }
    }

    pub fn compile(&mut self, ops: Vec<String>) {
        for sop in ops {
            let sop = sop.trim();
            if sop.starts_with("#") {
                continue;
            }
            if sop.starts_with("!") {
                self.compile_directive(sop);
                continue;
            }
            let op = Op::compile(sop).expect(&format!("ERRR:FuzzerK:VM:Compile:Op:{}", sop));
            self.ops.push(op);
        }
    }

    pub fn load_prg(&mut self, prgfile: &str) {
        let mut ops = Vec::<String>::new();
        let prgdata = fs::read_to_string(prgfile).expect("ERRR:FuzzerK:VM:Loading prg");
        let prgdata: Vec<&str> =  prgdata.split("\n").collect();
        for l in prgdata {
            ops.push(l.to_string());
        }
        self.compile(ops);
    }

    pub fn predefined_prg(&mut self, fc: &str, loopcnt: usize) {
        let mut runcmds = Vec::<String>::new();
        runcmds.push("iob new".to_string());
        runcmds.push(format!("fc {}", fc));
        runcmds.push("iob write".to_string());
        runcmds.push("iob flush".to_string());
        runcmds.push("loop inc".to_string());
        runcmds.push(format!("loop iflt {} abspos 1", loopcnt));
        self.compile(runcmds);
    }

    pub fn load_fcrtm(&mut self, cfgfc: &str) {
        cfgfiles::parse_file(cfgfc, &mut self.ctxt.fcrtm);
    }

    pub fn run(&mut self) {
        loop {
            let theop = &self.ops[self.ctxt.iptr];
            log_d(&format!("INFO:FuzzerK:VM:Op:{}:{:?}", self.ctxt.iptr, theop));
            self.ctxt.iptrupdate = true;
            theop.run(&mut self.ctxt);
            if self.ctxt.iptrupdate {
                self.ctxt.iptr += 1;
            }
        }
    }

}
