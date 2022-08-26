//!
//! A virtual program environment
//!
//! HanishKVC, 2022
//!

use std::collections::HashMap;
use std::collections::VecDeque;
use std::fs;
use std::thread;
use std::time;
use std::time::Duration;

use loggerk::log_w;
use loggerk::{log_e, log_d};
use rand::Rng;
use crate::datautils;
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
    iptr_commonupdate: bool,
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
            iptr_commonupdate: true,
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
    IobRead(String, String),
    IobClose(String),
    IfLt(String, String, String, String),
    CheckJump(String, String, String, String, String),
    Jump(String),
    SleepMSec(u64),
    FcGet(String, String),
    BufNew(String, usize),
    LetBuf(String, String),
    Buf8Randomize(String, isize, isize, isize, u8, u8),
    BufsMerge(String, Vec<String>),
}


impl Op {

    fn compile(opplus: &str) -> Result<Op, String> {
        let msgtag = "FuzzerK:VM:Op:Compile";
        let (sop, sargs) = opplus.split_once(' ').expect(&format!("ERRR:{}:ExtractingOp+:{}", msgtag, opplus));
        let sargs = sargs.trim();
        match sop {
            "none" => {
                // Wont reach here bcas opplus split_once will fail, just to keep Compiler from warning about None not used.
                // May remove None
                return Ok(Op::None);
            }

            "letstr" => {
                let (vid, vval) = sargs.split_once(' ').expect(&format!("ERRR:{}:LetStr:{}", msgtag, sargs));
                return Ok(Op::LetStr(vid.to_string(), vval.to_string()));
            }
            "letint" => {
                let (vid, sval) = sargs.split_once(' ').expect(&format!("ERRR:{}:LetInt:{}", msgtag, sargs));
                let ival = isize::from_str_radix(sval, 10).expect(&format!("ERRR:{}:LetInt:Value:{}", msgtag, sval));
                return Ok(Op::LetInt(vid.to_string(), ival));
            }

            "inc" => {
                return Ok(Op::Inc(sargs.to_string()));
            }
            "dec" => {
                return Ok(Op::Dec(sargs.to_string()));
            }

            "iobnew" => {
                let args: Vec<&str> = sargs.splitn(3, ' ').collect();
                if args.len() < 2 {
                    panic!("ERRR:{}:IobNew:InsufficientArgs:{}:[{:?}]", msgtag, sargs, args);
                }
                let ioid = args[0].to_string();
                let ioaddr = args[1].to_string();
                let mut sioargs = "";
                if args.len() == 3 {
                    sioargs = args[2];
                }
                let mut ioargs = HashMap::new();
                let lioargs = sioargs.split(" ").collect::<Vec<&str>>();
                for sioarg in lioargs {
                    if sioarg.len() == 0 {
                        continue;
                    }
                    let (k, v) = sioarg.split_once("=").expect(&format!("ERRR:{}:IobNew:IoArgs:{}", msgtag, sioargs));
                    ioargs.insert(k.to_string(), v.to_string());
                }
                return Ok(Op::IobNew(ioid, ioaddr, ioargs));
            }
            "iobwrite" => {
                let (ioid, bufid) = sargs.split_once(' ').expect(&format!("ERRR:{}:IobWrite:{}", msgtag, sargs));
                return Ok(Op::IobWrite(ioid.to_string(), bufid.to_string()));
            }
            "iobflush" => {
                return Ok(Op::IobFlush(sargs.to_string()));
            }
            "iobread" => {
                let (ioid, bufid) = sargs.split_once(' ').expect(&format!("ERRR:{}:IobRead:{}", msgtag, sargs));
                return Ok(Op::IobRead(ioid.to_string(), bufid.to_string()));
            }
            "iobclose" => {
                return Ok(Op::IobClose(sargs.to_string()));
            }

            "iflt" => {
                let args: Vec<&str> = sargs.splitn(4, ' ').collect();
                if args.len() != 4 {
                    panic!("ERRR:{}:IfLt:InsufficientArgs:{}", msgtag, sargs);
                }
                return Ok(Op::IfLt(args[0].to_string(), args[1].to_string(), args[2].to_string(), args[3].to_string()));
            }
            "checkjump" => {
                let args: Vec<&str> = sargs.splitn(5, ' ').collect();
                if args.len() != 5 {
                    panic!("ERRR:{}:CheckJump:InsufficientArgs:{}", msgtag, sargs);
                }
                return Ok(Op::CheckJump(args[0].to_string(), args[1].to_string(), args[2].to_string(), args[3].to_string(), args[4].to_string()));
            }
            "jump" => {
                return Ok(Op::Jump(sargs.to_string()));
            }

            "sleepmsec" => {
                let uval = u64::from_str_radix(sargs, 10).expect(&format!("ERRR:{}:SleepMSec:Value:{}", msgtag, sargs));
                return Ok(Op::SleepMSec(uval));
            }

            "fcget" => {
                let (fcid, bufid) = sargs.split_once(' ').expect(&format!("ERRR:{}:FcGet:{}", msgtag, sargs));
                return Ok(Op::FcGet(fcid.to_string(), bufid.to_string()));
            }

            "bufnew" => {
                let (bufid, bufsize) = sargs.split_once(' ').expect(&format!("ERRR:{}:BufNew:{}", msgtag, sargs));
                let bufsize = usize::from_str_radix(bufsize, 10).expect(&format!("ERRR:{}:BufNew:Size:{}", msgtag, bufsize));
                return Ok(Op::BufNew(bufid.to_string(), bufsize));
            }
            "letbuf" => {
                let (bufid, bufdata) = sargs.split_once(' ').expect(&format!("ERRR:{}:LetBuf:{}", msgtag, sargs));
                return Ok(Op::LetBuf(bufid.to_string(), bufdata.to_string()));
            }
            "buf8randomize" => {
                let parts: Vec<&str> = sargs.split(" ").collect();
                let bufid = parts[0].to_string();

                let randcount;
                let startoffset;
                let endoffset;
                let startval;
                let endval;

                if parts.len() >= 2 {
                    randcount = isize::from_str_radix(parts[1], 10).expect(&format!("ERRR:{}:Buf8Randomize:RandCount:{}", msgtag, parts[1]));
                } else {
                    randcount = -1;
                }
                if parts.len() >= 3 {
                    startoffset = isize::from_str_radix(parts[2], 10).expect(&format!("ERRR:{}:Buf8Randomize:StartOffset:{}", msgtag, parts[2]));
                } else {
                    startoffset = -1;
                }
                if parts.len() >= 4 {
                    //endoffset = isize::from_str_radix(parts[3], 10).expect(&format!("ERRR:{}:Buf8Randomize:EndOffset:{}", msgtag, parts[3]));
                    endoffset = datautils::intvalue(parts[3], &format!("ERRR:{}:Buf8Randomize:EndOffset:{}", msgtag, parts[3]));
                } else {
                    endoffset = -1;
                }
                if parts.len() >= 5 {
                    //startval = u8::from_str_radix(parts[4], 10).expect(&format!("ERRR:{}:Buf8Randomize:StartVal:{}", msgtag, parts[4]));
                    let tstartval: datautils::U8X = datautils::intvalue(parts[4], &format!("ERRR:{}:Buf8Randomize:StartVal:{}", msgtag, parts[4]));
                    startval = tstartval.try_into().unwrap();
                } else {
                    startval = 0;
                }
                if parts.len() == 6 {
                    //endval = u8::from_str_radix(parts[5], 10).expect(&format!("ERRR:{}:Buf8Randomize:EndVal:{}", msgtag, parts[5]));
                    endval = datautils::intvalue::<datautils::U8X>(parts[5], &format!("ERRR:{}:Buf8Randomize:EndVal:{}", msgtag, parts[5])).into();
                } else {
                    endval = 255;
                }
                if parts.len() > 6 {
                    panic!("ERRR:{}:Buf8Randomize:Too many args:{}", msgtag, sargs);
                }
                return Ok(Op::Buf8Randomize(bufid, randcount, startoffset, endoffset, startval, endval))
            }
            "bufsmerge" => {
                let mut parts: VecDeque<&str> = sargs.split_whitespace().collect();
                let numparts = parts.len();
                if numparts < 2 {
                    panic!("ERRR:{}:BufsMerge:Too few bufs:{}", msgtag, sargs);
                }
                if numparts == 2 {
                    log_w(&format!("WARN:{}:BufsMerge:Only a copy will occur, specify more buffers to concat:{}", msgtag, sargs));
                }
                let bufid = parts.pop_front().unwrap().to_string();
                let mut vbufs = Vec::new();
                for sbuf in parts {
                    vbufs.push(sbuf.to_string());
                }
                //log_d(&format!("DBUG:{}:BufsMerge:{} <- {:?}", msgtag, bufid, vbufs));
                return Ok(Op::BufsMerge(bufid, vbufs));
            }
            _ => todo!()
        }
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
            Self::IobRead(ioid, bufid) => {
                let buf = ctxt.bufs.get_mut(bufid).expect(&format!("ERRR:FuzzerK:VM:Op:IobRead:ToBuf:{}", bufid));
                let zenio = ctxt.iobs.get_mut(ioid).expect(&format!("ERRR:FuzzerK:VM:Op:IobRead:{}", ioid));
                let gotr = zenio.read(buf);
                if gotr.is_err() {
                    let errmsg = gotr.as_ref().unwrap_err();
                    log_e(&format!("ERRR:FuzzerK:VM:Op:IobRead:{}:ToBuf:{}:{}", ioid, bufid, errmsg));
                }
                let readsize = gotr.unwrap();
                buf.resize(readsize, 0);
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
                let fci = ctxt.fcrtm.fcimmuts(&fcid).expect(&format!("ERRR:FuzzerK:VM:Op:FcGet:UnknownFC???:{}", fcid));
                let gotfuzz = fci.get(ctxt.stepu);
                log_d(&format!("\n\nGot:{}:\n\t{:?}\n\t{}", ctxt.stepu, gotfuzz, String::from_utf8_lossy(&gotfuzz)));
                ctxt.bufs.insert(bufid.to_string(), gotfuzz);
                ctxt.stepu += 1;
            }
            Self::IfLt(sval, vid, sop , oparg) => {
                let chkval = isize::from_str_radix(sval, 10).expect(&format!("ERRR:FuzzerK:VM:Op:IfLt:ChkVal:{}:Conversion", sval));
                let curval = *ctxt.ints.get(vid).expect(&format!("ERRR:FuzzerK:VM:Op:IfLt:Var:{}", vid));
                let mut opdo = false;
                //log_d(&format!("DBUG:FuzzerK:VM:Op:IfLt:{},{},{},{}", chkval, curval, sop, oparg));
                if curval < chkval {
                    opdo = true;
                }
                if opdo {
                    if sop == "goto" {
                        // Translating the label here at runtime, rather than during compile time, allows goto to refer to label
                        // that might not yet have been defined at the point where goto or rather the IfLt is encountered.
                        // Especially when only a single pass parsing of the program is done.
                        ctxt.iptr = *ctxt.lbls.get(oparg).expect(&format!("ERRR:FuzzerK:VM:Op:IfLt:GoTo:Label:{}", oparg));
                        ctxt.iptr_commonupdate = false;
                        //log_d(&format!("DBUG:FuzzerK:VM:Op:IfLt:Goto:{}:{}", oparg, ctxt.iptr));
                    }
                }
            }
            Self::CheckJump(arg1, arg2, ltlabel, eqlabel, gtlabel) => {
                let varg1;
                if arg1.starts_with("$") {
                    varg1 = isize::from_str_radix(&arg1[1..], 10).expect("ERRR:FuzzerK:VM:Op:CheckJump:Arg1Val");
                } else {
                    varg1 = *ctxt.ints.get(arg1).expect("ERRR:FuzzerK:VM:Op:CheckJump:Arg1Var");
                }
                let varg2;
                if arg2.starts_with("$") {
                    varg2 = isize::from_str_radix(&arg2[1..], 10).expect("ERRR:FuzzerK:VM:Op:CheckJump:Arg2Val");
                } else {
                    varg2 = *ctxt.ints.get(arg2).expect("ERRR:FuzzerK:VM:Op:CheckJump:Arg2Var");
                }
                let label;
                if varg1 < varg2 {
                    label = ltlabel;
                } else if varg1 == varg2 {
                    label = eqlabel;
                } else {
                    label = gtlabel;
                }
                if label != "__NEXT__" {
                    ctxt.iptr = *ctxt.lbls.get(label).expect(&format!("ERRR:FuzzerK:VM:Op:CheckJump:Label:{}", label));
                    ctxt.iptr_commonupdate = false;
                }
            }
            Self::Jump(label) => {
                if label != "__NEXT__" {
                    ctxt.iptr = *ctxt.lbls.get(label).expect(&format!("ERRR:FuzzerK:VM:Op:Jump:Label:{}", label));
                    ctxt.iptr_commonupdate = false;
                }
            }
            Self::BufNew(bufid, bufsize) => {
                let mut buf = Vec::<u8>::new();
                buf.resize(*bufsize, 0);
                ctxt.bufs.insert(bufid.to_string(), buf);
            }
            Self::LetBuf(bufid, bufdata) => {
                let mut vdata;
                if bufdata == "__TIME__STAMP__" {
                    let ts = format!("{:?}",time::SystemTime::now());
                    vdata = Vec::from(ts);
                } else if bufdata.starts_with("__RANDOM__BYTES__") {
                    let (_random, bytelen) = bufdata.split_once("__BYTES__").expect(&format!("ERRR:FuzzerK:VM:Op:LetBuf:RandomBytes:{}", bufdata));
                    let bytelen = usize::from_str_radix(bytelen, 10).expect(&format!("ERRR:FuzzerK:VM:Op:LetBuf:RandomBytes:{}", bufdata));
                    let mut rng = rand::thread_rng();
                    vdata = Vec::new();
                    for _i in 0..bytelen {
                        vdata.push(rng.gen_range(0..=255)); // rusty 0..256
                    }
                } else if bufdata.starts_with("0x") {
                    vdata = datautils::vu8_from_hex(&bufdata[2..]).expect(&format!("ERRR:FuzzerK:VM:Op:LetBuf:HexData:{}", bufdata));
                } else {
                    vdata = Vec::from(bufdata.clone());
                }
                ctxt.bufs.insert(bufid.to_string(), vdata);
            }
            Self::Buf8Randomize(bufid, randcount, startoffset, endoffset, startval, endval) => {
                let buf = ctxt.bufs.get_mut(bufid).expect(&format!("ERRR:FuzzerK:VM:Op:Buf8Randomize:Buf:{}", bufid));
                let trandcount;
                if *randcount < 0 {
                    trandcount = rand::random::<usize>() % buf.len();
                } else {
                    trandcount = *randcount as usize;
                }
                let tstartoffset;
                if *startoffset < 0 {
                    tstartoffset = 0;
                } else {
                    tstartoffset = *startoffset as usize;
                }
                let tendoffset;
                if *endoffset < 0 {
                    tendoffset = buf.len()-1;
                } else {
                    tendoffset = *endoffset as usize;
                }
                let mut rng = rand::thread_rng();
                let offsetwidth = tendoffset - tstartoffset + 1;
                let valwidth: u16 = *endval as u16 - *startval as u16 + 1;
                for _i in 0..trandcount {
                    let curind = tstartoffset + (rng.gen::<usize>() % offsetwidth);
                    let curval = *startval + (rng.gen::<u16>() % valwidth) as u8;
                    buf[curind] = curval;
                }
            }
            Self::BufsMerge(destbufid, srcbufids) => {
                //let destbuf = ctxt.bufs.get_mut(destbufid).expect(&format!("ERRR:FuzzerK:VM:Op:BufsMerge:Dest:{}", destbufid));
                let mut destbuf = Vec::new();
                for srcbufid in srcbufids {
                    let srcbuf = ctxt.bufs.get_mut(srcbufid).expect(&format!("ERRR:FuzzerK:VM:Op:BufsMerge:SrcBuf:{}", srcbufid));
                    let mut dupbuf = srcbuf.clone();
                    destbuf.append(&mut dupbuf);
                }
                ctxt.bufs.insert(destbufid.to_string(), destbuf);
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
            self.ctxt.lbls.insert(sargs.to_string(), self.ops.len());
        } else {
            panic!("ERRR:FuzzerK:VM:CompileDirective:Unknown:{}", sdirplus);
        }
    }

    pub fn compile(&mut self, ops: Vec<String>) {
        let mut linenum = -1;
        for sop in ops {
            linenum += 1;
            let sop = sop.trim();
            if sop.starts_with("#") || (sop.len() == 0) {
                continue;
            }
            log_d(&format!("DBUG:FuzzerK:VM:Compile:Op:{}:{}", linenum, sop));
            if sop.starts_with("!") {
                self.compile_directive(sop);
                continue;
            }
            let op = Op::compile(sop).expect(&format!("ERRR:FuzzerK:VM:Compile:Op:{}", sop));
            self.ops.push(op);
        }
    }

    pub fn load_prg(&mut self, prgfile: &str) {
        if prgfile.len() == 0 {
            log_w("WARN:FuzzerK:VM:LoadPRG:Empty filename passed, skipping...");
            return;
        }
        let mut ops = Vec::<String>::new();
        let prgdata = fs::read_to_string(prgfile).expect("ERRR:FuzzerK:VM:Loading prg");
        let prgdata: Vec<&str> =  prgdata.split("\n").collect();
        for l in prgdata {
            ops.push(l.to_string());
        }
        self.compile(ops);
    }

    pub fn predefined_prg(&mut self, fc: &str, loopcnt: usize, ioaddr: &str, ioargshm: &HashMap<String, String>) {
        let mut ioargs = String::new();
        for ioarg in ioargshm {
            let sioarg = format!("{}={} ", ioarg.0, ioarg.1);
            ioargs.push_str(&sioarg);
        }
        let mut runcmds = Vec::<String>::new();
        runcmds.push("letint loopcnt 0".to_string());
        runcmds.push("!label freshstart".to_string());
        runcmds.push(format!("iobnew srvX {} {}", ioaddr, ioargs));
        runcmds.push(format!("fcget {} fuzzgot", fc));
        runcmds.push("iobwrite srvX fuzzgot".to_string());
        runcmds.push("iobflush srvX".to_string());
        runcmds.push("inc loopcnt".to_string());
        runcmds.push(format!("iflt {} loopcnt goto freshstart", loopcnt));
        self.compile(runcmds);
    }

    pub fn load_fcrtm(&mut self, cfgfc: &str) {
        if cfgfc.len() == 0 {
            log_w("WARN:FuzzerK:VM:LoadFCRTM:Empty filename passed, skipping...");
            return;
        }
        cfgfiles::parse_file(cfgfc, &mut self.ctxt.fcrtm);
    }

    pub fn run(&mut self) {
        loop {
            if self.ctxt.iptr >= self.ops.len() {
                break;
            }
            let theop = &self.ops[self.ctxt.iptr];
            log_d(&format!("INFO:FuzzerK:VM:Op:{}:{:?}", self.ctxt.iptr, theop));
            self.ctxt.iptr_commonupdate = true;
            theop.run(&mut self.ctxt);
            if self.ctxt.iptr_commonupdate {
                self.ctxt.iptr += 1;
            }
        }
    }

}
