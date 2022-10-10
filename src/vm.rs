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
    callstack: Vec<usize>,
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
            callstack: Vec::new(),
        }
    }
}


#[derive(Debug)]
enum DataM {
    IntLiteral(isize),
    IntVar(String),
    StringLiteral(String),
    StringVar(String),
    BufData(Vec<u8>),
    AnyVar(String),
    XTimeStamp,
    XRandomBytes(usize)
}


impl DataM {

    ///
    /// * int literals should start with numeric char
    /// * string literals should be in double quotes ""
    /// * buf8 literals should start with $0x
    /// * special literals should start with __
    /// * anything else is treated as a Var name.
    ///   * it needs to start with a alpabhetic char
    ///   * it could either be a IntVar or StringVar or Buf8Var
    ///
    fn compile(mut sdata: &str, stype: &str, smsg: &str) -> DataM {
        sdata = sdata.trim();
        if sdata == "" {
            panic!("ERRR:{}:DataM:Compile:{}:Data token empty", smsg, stype);
        }
        let schar = sdata.chars().nth(0).unwrap();
        let echar = sdata.chars().last().unwrap();

        if schar.is_numeric() || schar == '+' || schar == '-' {
            let idata = datautils::intvalue(sdata, &format!("ERRR:{}:DataM:Compile:IntLiteral:Conversion", smsg));
            return DataM::IntLiteral(idata);
        }

        if sdata.len() >= 2 {

            if schar == '"' || echar == '"' {
                let mut rdata = sdata.clone();
                rdata = rdata.strip_prefix('"').expect(&format!("ERRR:{}:DataM:Compile:StringLiteral:Missing double quote at start of {}", smsg, sdata));
                rdata = rdata.strip_suffix('"').expect(&format!("ERRR:{}:DataM:Compile:StringLiteral:Missing double quote at end of {}", smsg, sdata));
                return DataM::StringLiteral(rdata.to_string());
            }

            if sdata.len() > 2 {
                if sdata.starts_with("$0x") {
                    let bdata = datautils::vu8_from_hex(&sdata[3..]).expect(&format!("ERRR:{}:DataM:Compile:BufHexString:Conversion:{}", smsg, sdata));
                    return DataM::BufData(bdata);
                }
                if sdata.starts_with("__") {
                    if sdata == "__TIME__STAMP__" {
                        return DataM::XTimeStamp;
                    }
                    if sdata.starts_with("__RANDOM__BYTES__") {
                        let (_random, bytelen) = sdata.split_once("__BYTES__").expect(&format!("ERRR:{}:DataM:Compile:RandomBytes:{}", smsg, sdata));
                        let bytelen = usize::from_str_radix(bytelen, 10).expect(&format!("ERRR:{}:DataM:Compile:RandomBytes:{}", smsg, sdata));
                        return DataM::XRandomBytes(bytelen);
                    }
                    panic!("ERRR:{}:DataM:Compile:{}:Unknown Special Tag {}???", smsg, stype, sdata);
                }
            }

        }

        if !schar.is_alphabetic() {
            panic!("ERRR:{}:DataM:{}:Variable name {} should start with a alphabetic char", smsg, stype, sdata);
        }

        match stype {
            "isize" => {
                return DataM::IntVar(sdata.to_string());
            }
            "string" => {
                return DataM::StringVar(sdata.to_string())
            }
            "any" => {
                return DataM::AnyVar(sdata.to_string())
            }
            _ => {
                panic!("ERRR:{}:DataM:{}:Unknown type???", smsg, stype);
            }
        }

    }

    ///
    /// * Int -> Int
    /// * String -> Try interpret the string as a textual literal value of a integer
    /// * Buf -> Try interpret the buf as the underlying raw byte values of a integer
    /// * XTimeStamp -> milliseconds from UnixEpoch truncated
    /// * XRandomBytes -> a randomly generated Int (limited to min(Int size,requested bytes))
    ///
    fn get_isize(&self, ctxt: &mut Context, smsg: &str) -> isize {
        match self {
            Self::IntLiteral(ival) => {
                return *ival;
            },
            Self::IntVar(vid) => {
                let ival  = *ctxt.ints.get(vid).expect(&format!("ERRR:{}:DataM:GetISize:IntVar: Failed to get var", smsg));
                return ival;
            },
            Self::StringLiteral(sval) => {
                return datautils::intvalue(sval, &format!("ERRR:{}:DataM:GetISize:StringLiteral: Conversion failed", smsg));
            },
            Self::StringVar(vid) => {
                let sval  = ctxt.strs.get(vid).expect(&format!("ERRR:{}:DataM:GetISize:StringVar: Failed to get var", smsg));
                return datautils::intvalue(sval, &format!("ERRR:{}:DataM:GetISize:StringVar: Conversion failed", smsg));
            },
            Self::BufData(sval) => {
                //return datautils::intvalue(&String::from_utf8_lossy(sval), &format!("ERRR:{}:DataM:GetISize:BufData: Conversion failed", smsg));
                return isize::from_ne_bytes(sval.as_slice().try_into().expect(&format!("ERRR:{}:DataM:GetISize:BufData: Conversion failed", smsg)));
            },
            Self::AnyVar(vid) => {
                let ival  = ctxt.ints.get(vid);
                if ival.is_some() {
                    return *ival.unwrap();
                }
                let sval = ctxt.strs.get(vid);
                if sval.is_some() {
                    return datautils::intvalue(sval.unwrap(), &format!("ERRR:{}:DataM:GetISize:AnyVarString: Conversion failed", smsg));
                }
                let sval = ctxt.bufs.get(vid);
                if sval.is_some() {
                    //return datautils::intvalue(&String::from_utf8_lossy(sval.unwrap()), &format!("ERRR:{}:DataM:GetISize:AnyVarBuf: Conversion failed", smsg));
                    return isize::from_ne_bytes(sval.unwrap().as_slice().try_into().expect(&format!("ERRR:{}:DataM:GetISize:AnyVarBuf: Conversion failed", smsg)));
                }
                panic!("ERRR:{}:DataM:GetISize:AnyVar:Unknown:{}", smsg, vid);
            },
            Self::XTimeStamp => {
                let ts = time::SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap();
                let uts = ts.as_millis();
                return uts as isize;
            },
            Self::XRandomBytes(bytelen) => {
                let mut rng = rand::thread_rng();
                let mut vdata: Vec<u8> = Vec::new();
                let mut ibytes = isize::BITS/8;
                if (ibytes as usize) > *bytelen {
                    ibytes = *bytelen as u32;
                }
                for _i in 0..ibytes {
                    vdata.push(rng.gen_range(0..=255)); // rusty 0..256
                }
                return isize::from_le_bytes(vdata.as_slice().try_into().unwrap());
            }
        }
    }

    ///
    /// Return a positive interger value, this is built upon get_isize
    /// If the underlying value is negative, then it will panic
    ///
    fn get_usize(&self, ctxt: &mut Context, smsg: &str) -> usize {
        let ival = self.get_isize(ctxt, &format!("{}:DataM:GetUSize",smsg));
        if ival < 0 {
            panic!("ERRR:{}:DataM:GetUSize: Negative int value not supported here", smsg)
        }
        return ival as usize;
    }

    ///
    /// * Returns Int values as equivalent string literal form
    /// * Returns String as is
    /// * Returns Buf8 data as a hex string
    /// * AnyVar follows the order of 1st check IntVars, then StrVars and then finally BufVars
    /// * XTimeStamp returns current System time as a string
    /// * XRandomBytes returns random generated bytes converted to string using utf8_lossy
    ///
    fn get_string(&self, ctxt: &mut Context, smsg: &str) -> String {
        match self {
            DataM::IntLiteral(ival) => ival.to_string(),
            DataM::IntVar(vid) => {
                let ival  = *ctxt.ints.get(vid).expect(&format!("ERRR:{}:DataM:GetString:IntVar: Failed to get var", smsg));
                ival.to_string()
            },
            DataM::StringLiteral(sval) => sval.clone(),
            DataM::StringVar(vid) => {
                let sval  = ctxt.strs.get(vid).expect(&format!("ERRR:{}:DataM:GetString:StringVar: Failed to get var", smsg));
                sval.clone()
            },
            DataM::BufData(bval) => {
                //return String::from_utf8_lossy(bval).to_string();
                //let mut bval = bval.clone();
                //bval.reverse();
                return datautils::hex_from_vu8(&bval);
            },
            DataM::AnyVar(vid) => {
                let ival  = ctxt.ints.get(vid);
                if ival.is_some() {
                    return ival.unwrap().to_string();
                }
                let sval = ctxt.strs.get(vid);
                if sval.is_some() {
                    return sval.unwrap().to_string();
                }
                let sval = ctxt.bufs.get(vid);
                if sval.is_some() {
                    //return String::from_utf8_lossy(sval.unwrap()).to_string();
                    return datautils::hex_from_vu8(sval.unwrap());
                }
                panic!("ERRR:{}:DataM:GetString:AnyVar:Unknown:{}", smsg, vid);
            },
            DataM::XTimeStamp => {
                return format!("{:?}",time::SystemTime::now());
            },
            DataM::XRandomBytes(bytelen) => {
                let mut rng = rand::thread_rng();
                let mut vdata: Vec<u8> = Vec::new();
                for _i in 0..*bytelen {
                    vdata.push(rng.gen_range(0..=255)); // rusty 0..256
                }
                return String::from_utf8_lossy(&vdata).to_string();
            }
        }
    }

    ///
    /// * returns int values as byte buffer in the native endianess format
    /// * Returns String as the underlying byte vector
    /// * Returns Buf8 data as is
    /// * AnyVar follows the order of 1st check IntVars, then StrVars and then finally BufVars
    /// * XTimeStamp -> milliseconds from UnixEpoch, as the underlying byte values of the int
    /// * XRandomBytes returns random generated bytes
    ///
    /// TODO:ThinkAgain: Should I return a fixed endian format like network byte order (BigEndian) or little endian
    /// rather than native byte order (If testing between systems having different endianess, it could help)
    fn get_bufvu8(&self, ctxt: &mut Context, smsg: &str) -> Vec<u8> {
        match self {
            DataM::IntLiteral(ival) => Vec::from(ival.to_ne_bytes()),
            DataM::IntVar(vid) => {
                let ival  = *ctxt.ints.get(vid).expect(&format!("ERRR:{}:DataM:GetBuf:IntVar: Failed to get var", smsg));
                log_d(&format!("DBUG:DataM:GetBufVU8:IntVar:{}:{}", vid, ival));
                Vec::from(ival.to_ne_bytes())
            },
            DataM::StringLiteral(sval) => Vec::from(sval.to_string()),
            DataM::StringVar(vid) => {
                let sval  = ctxt.strs.get(vid).expect(&format!("ERRR:{}:DataM:GetBuf:StringVar: Failed to get var", smsg));
                log_d(&format!("DBUG:DataM:GetBufVU8:StrVar:{}:{}", vid, sval));
                Vec::from(sval.to_string())
            },
            DataM::BufData(bval) => {
                return bval.to_vec();
            },
            DataM::AnyVar(vid) => {
                let ival  = ctxt.ints.get(vid);
                if ival.is_some() {
                    let ival = ival.unwrap().to_ne_bytes();
                    log_d(&format!("DBUG:DataM:GetBufVU8:AnyIntVar:{}:{:?}", vid, ival));
                    return Vec::from(ival)
                }
                let sval = ctxt.strs.get(vid);
                if sval.is_some() {
                    let sval = sval.unwrap().to_string();
                    log_d(&format!("DBUG:DataM:GetBufVU8:AnyStrVar:{}:{}", vid, sval));
                    return Vec::from(sval)
                }
                let sval = ctxt.bufs.get(vid);
                if sval.is_some() {
                    let bval = sval.unwrap().to_vec();
                    log_d(&format!("DBUG:DataM:GetBufVU8:AnyBufVar:{}:{:?}", vid, bval));
                    return bval;
                }
                panic!("ERRR:{}:DataM:GetBuf:AnyVar:Unknown:{}", smsg, vid);
            },
            DataM::XTimeStamp => {
                return time::SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap().as_millis().to_ne_bytes().to_vec();
            },
            DataM::XRandomBytes(bytelen) => {
                let mut rng = rand::thread_rng();
                let mut vdata: Vec<u8> = Vec::new();
                for _i in 0..*bytelen {
                    vdata.push(rng.gen_range(0..=255)); // rusty 0..256
                }
                return vdata;
            }
        }
    }

}


#[derive(Debug)]
enum CondOp {
    IfLtInt,
    IfEqInt,
    IfEqStr,
}

impl CondOp {

    fn check(&self, ctxt: &mut Context, val1: &DataM, val2: &DataM) -> bool {
        match self {
            CondOp::IfLtInt => {
                let val1 = val1.get_isize(ctxt, "FuzzerK:Vm:CondOp:IfLtInt:Val1");
                let val2 = val2.get_isize(ctxt, "FuzzerK:Vm:CondOp:IfLtInt:Val2");
                if val1 < val2 {
                    return true;
                }
                return false;
            },
            CondOp::IfEqInt => {
                let val1 = val1.get_isize(ctxt, "FuzzerK:Vm:CondOp:IfEqInt:Val1");
                let val2 = val2.get_isize(ctxt, "FuzzerK:Vm:CondOp:IfEqInt:Val2");
                if val1 == val2 {
                    return true;
                }
                return false;
            },
            CondOp::IfEqStr => {
                let val1 = val1.get_string(ctxt, "FuzzerK:Vm:CondOp:IfEqStr:Val1");
                let val2 = val2.get_string(ctxt, "FuzzerK:Vm:CondOp:IfEqStr:Val2");
                if val1 == val2 {
                    return true;
                }
                return false;
            }
        }
    }

}

#[derive(Debug)]
enum ALUOP {
    Add,
    Sub,
    Mult,
    Div,
    Mod,
}


#[derive(Debug)]
enum Op {
    Nop,
    LetStr(String, DataM),
    LetInt(String, DataM),
    Inc(String),
    Dec(String),
    Alu(ALUOP, String, DataM, DataM),
    IobNew(String, String, HashMap<String, String>),
    IobWrite(String, String),
    IobFlush(String),
    IobRead(String, String),
    IobClose(String),
    If(CondOp, DataM, DataM, String, String),
    CheckJump(DataM, DataM, String, String, String),
    Jump(String),
    Call(String),
    Ret,
    SleepMSec(DataM),
    FcGet(String, String),
    BufNew(String, DataM),
    LetBuf(String, DataM),
    LetBufStr(String, DataM),
    Buf8Randomize(String, DataM, DataM, DataM, DataM, DataM),
    BufsMerge(String, Vec<String>),
}


impl Op {

    fn compile(opplus: &str) -> Result<Op, String> {
        let msgtag = "FuzzerK:VM:Op:Compile";
        let sop;
        let sargs;
        let op_and_args= opplus.split_once(' ');
        if op_and_args.is_some() {
            (sop, sargs) = op_and_args.unwrap();
        } else {
            sop = opplus;
            sargs = "";
        }
        let sargs = sargs.trim();
        match sop {
            "nop" => {
                return Ok(Op::Nop);
            }

            "letstr" => {
                let (vid, sval) = sargs.split_once(' ').expect(&format!("ERRR:{}:LetStr:{}", msgtag, sargs));
                let dm = DataM::compile(sval, "string", &format!("{}:LetStr:Value:{}", msgtag, sval));
                return Ok(Op::LetStr(vid.to_string(), dm));
            }
            "letint" => {
                let (vid, sval) = sargs.split_once(' ').expect(&format!("ERRR:{}:LetInt:{}", msgtag, sargs));
                let dm = DataM::compile(sval, "isize", &format!("{}:LetInt:Value:{}", msgtag, sval));
                return Ok(Op::LetInt(vid.to_string(), dm));
            }

            "inc" => {
                return Ok(Op::Inc(sargs.to_string()));
            }
            "dec" => {
                return Ok(Op::Dec(sargs.to_string()));
            }

            "add" | "sub" | "mult" | "div" | "mod" => {
                let aluop = match sop {
                    "add" => ALUOP::Add,
                    "sub" => ALUOP::Sub,
                    "mult" => ALUOP::Mult,
                    "div" => ALUOP::Div,
                    "mod" => ALUOP::Mod,
                    _ => todo!(),
                };
                let args: Vec<&str> = sargs.split_whitespace().collect();
                let dmsrc1 = DataM::compile(args[1], "isize", &format!("{}:{}:SrcArg1", msgtag, sop));
                let dmsrc2 = DataM::compile(args[2], "isize", &format!("{}:{}:SrcArg2", msgtag, sop));
                return Ok(Op::Alu(aluop, args[0].to_string(), dmsrc1, dmsrc2));
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

            "iflt.i" | "ifeq.i" | "ifeq.s" => {
                let next = datautils::next_token(sargs).unwrap();
                let arg0 = next.0;
                let next = datautils::next_token(&next.1).unwrap();
                let arg1 = next.0;
                let args: Vec<&str> = next.1.splitn(2, ' ').collect();
                if args.len() != 2 {
                    panic!("ERRR:{}:{}:InsufficientArgs:{}", msgtag, sop, sargs);
                }
                let val1dm = DataM::compile(&arg0, "any", &format!("{}:{}:CheckValue1:{}", msgtag, sop, arg0));
                let val2dm = DataM::compile(&arg1, "any", &format!("{}:{}:CheckValue2:{}", msgtag, sop, arg1));
                let cop = match sop {
                    "iflt.i" => CondOp::IfLtInt,
                    "ifeq.i" => CondOp::IfEqInt,
                    "ifeq.s" => CondOp::IfEqStr,
                    _ => todo!(),
                };
                return Ok(Op::If(cop, val1dm, val2dm, args[0].to_string(), args[1].to_string()));
            }
            "checkjump" => {
                let args: Vec<&str> = sargs.splitn(5, ' ').collect();
                if args.len() != 5 {
                    panic!("ERRR:{}:CheckJump:InsufficientArgs:{}", msgtag, sargs);
                }
                let arg1dm = DataM::compile(args[0], "isize", &format!("{}:CheckJump:Arg1:{}", msgtag, args[0]));
                let arg2dm = DataM::compile(args[1], "isize", &format!("{}:CheckJump:Arg2:{}", msgtag, args[1]));
                return Ok(Op::CheckJump(arg1dm, arg2dm, args[2].to_string(), args[3].to_string(), args[4].to_string()));
            }
            "jump" => {
                return Ok(Op::Jump(sargs.to_string()));
            }
            "call" => {
                return Ok(Op::Call(sargs.to_string()));
            }
            "ret" => {
                return Ok(Op::Ret);
            }

            "sleepmsec" => {
                let msecdm = DataM::compile(sargs, "isize", &format!("{}:SleepMSec:Value:{}", msgtag, sargs));
                return Ok(Op::SleepMSec(msecdm));
            }

            "fcget" => {
                let (fcid, bufid) = sargs.split_once(' ').expect(&format!("ERRR:{}:FcGet:{}", msgtag, sargs));
                return Ok(Op::FcGet(fcid.to_string(), bufid.to_string()));
            }

            "bufnew" => {
                let (bufid, bufsize) = sargs.split_once(' ').expect(&format!("ERRR:{}:BufNew:{}", msgtag, sargs));
                let dmbufsize = DataM::compile(bufsize, "any", &format!("{}:BufNew:Size:{}", msgtag, bufsize));
                return Ok(Op::BufNew(bufid.to_string(), dmbufsize));
            }
            "letbuf" | "letbuf.s" => {
                let (bufid, bufdata) = sargs.split_once(' ').expect(&format!("ERRR:{}:LetBuf+:{}", msgtag, sargs));
                let dm = DataM::compile(bufdata, "any", &format!("{}:LetBuf+:Value:{}", msgtag, bufdata));
                if sop == "letbuf" {
                    return Ok(Op::LetBuf(bufid.to_string(), dm));
                } else if sop == "letbuf.s" {
                    return Ok(Op::LetBufStr(bufid.to_string(), dm));
                } else {
                    return Err(format!("ERRR:{}:LetBuf+:Unknown Variant:{}", msgtag, sop))
                }
            }
            "buf8randomize" => {
                let parts: Vec<&str> = sargs.split(" ").collect();
                let bufid = parts[0].to_string();

                let dmrandcount;
                let dmstartoffset;
                let dmendoffset;
                let dmstartval;
                let dmendval;

                let mut thepart;

                if parts.len() >= 2 {
                    thepart = parts[1].to_string();
                } else {
                    thepart = String::from("-1");
                }
                dmrandcount = DataM::compile(&thepart, "isize", &format!("{}:Buf8Randomize:RandCount:{}", msgtag, thepart));

                if parts.len() >= 3 {
                    thepart = parts[2].to_string();
                } else {
                    thepart = String::from("-1");
                }
                dmstartoffset = DataM::compile(&thepart, "isize", &format!("{}:Buf8Randomize:StartOffset:{}", msgtag, thepart));

                if parts.len() >= 4 {
                    thepart = parts[3].to_string();
                } else {
                    thepart = String::from("-1");
                }
                dmendoffset = DataM::compile(&thepart, "isize", &format!("{}:Buf8Randomize:EndOffset:{}", msgtag, thepart));

                if parts.len() >= 5 {
                    thepart = parts[4].to_string();
                } else {
                    thepart = String::from("0");
                }
                dmstartval = DataM::compile(&thepart, "isize", &format!("{}:Buf8Randomize:StartVal:{}", msgtag, thepart));

                if parts.len() == 6 {
                    thepart = parts[5].to_string();
                } else {
                    thepart = String::from("255");
                }
                dmendval = DataM::compile(&thepart, "isize", &format!("{}:Buf8Randomize:EndVal:{}", msgtag, thepart));

                if parts.len() > 6 {
                    panic!("ERRR:{}:Buf8Randomize:Too many args:{}", msgtag, sargs);
                }
                return Ok(Op::Buf8Randomize(bufid, dmrandcount, dmstartoffset, dmendoffset, dmstartval, dmendval))
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
            _ => panic!("ERRR:{}:UnknownOp:{}", msgtag, sop)
        }
    }

}

impl Op {

    fn run(&self, ctxt: &mut Context) {
        match self {
            Self::Nop => (),
            Self::LetStr(vid, vdm) => {
                let sval = vdm.get_string(ctxt, &format!("FuzzerK:VM:Op:LetStr:{} {:?}", vid, vdm));
                ctxt.strs.insert(vid.to_string(), sval);
            },
            Self::LetInt(vid, vval) => {
                let ival = vval.get_isize(ctxt, &format!("FuzzerK:VM:Op:LetInt:{} {:?}", vid, vval));
                ctxt.ints.insert(vid.to_string(), ival);
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
            },
            Self::Alu(aluop, destvid, dmsrc1, dmsrc2) => {
                let src1 = dmsrc1.get_isize(ctxt, "FuzzerK:VM:Op:Alu:Src1");
                let src2 = dmsrc2.get_isize(ctxt, "FuzzerK:VM:Op:Alu:Src2");
                let res = match aluop {
                    ALUOP::Add => src1 + src2,
                    ALUOP::Sub => src1 - src2,
                    ALUOP::Mult => src1 * src2,
                    ALUOP::Div => src1 / src2,
                    ALUOP::Mod => src1 % src2,
                };
                ctxt.ints.insert(destvid.to_string(), res);
            },
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
            Self::SleepMSec(msecdm) => {
                let msec = msecdm.get_usize(ctxt, &format!("FuzzerK:VM:Op:SleepMSec:Value:{:?}", msecdm));
                thread::sleep(Duration::from_millis(msec as u64));
            }
            Self::FcGet(fcid, bufid) => {
                let fci = ctxt.fcrtm.fcimmuts(&fcid).expect(&format!("ERRR:FuzzerK:VM:Op:FcGet:UnknownFC???:{}", fcid));
                let gotfuzz = fci.get(ctxt.stepu);
                log_d(&format!("\n\nGot:{}:\n\t{:?}\n\t{}", ctxt.stepu, gotfuzz, String::from_utf8_lossy(&gotfuzz)));
                ctxt.bufs.insert(bufid.to_string(), gotfuzz);
                ctxt.stepu += 1;
            }
            Self::If(cop, val1dm, val2dm, sop , oparg) => {
                let mut opdo = false;
                //log_d(&format!("DBUG:FuzzerK:VM:Op:IfLt:{},{},{},{}", val1, val2, sop, oparg));
                if cop.check(ctxt, val1dm, val2dm) {
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
                let varg1 = arg1.get_isize(ctxt, &format!("FuzzerK:VM:Op:CheckJump:GetArg1:{:?}", arg1));
                let varg2 = arg2.get_isize(ctxt, &format!("FuzzerK:VM:Op:CheckJump:GetArg2:{:?}", arg2));
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
            Self::Call(label) => {
                ctxt.callstack.push(ctxt.iptr);
                ctxt.iptr = *ctxt.lbls.get(label).expect(&format!("ERRR:FuzzerK:VM:Op:Call:Label:{}", label));
                ctxt.iptr_commonupdate = false;
            }
            Self::Ret => {
                ctxt.iptr = ctxt.callstack.pop().expect("ERRR:FuzzerK:VM:Op:Ret:CallStack");
            }

            Self::BufNew(bufid, dmbufsize) => {
                let mut buf = Vec::<u8>::new();
                let bufsize = dmbufsize.get_usize(ctxt, "FuzzerK:VM:Op:BufNew:BufSize");
                buf.resize(bufsize, 0);
                ctxt.bufs.insert(bufid.to_string(), buf);
            }
            Self::LetBuf(bufid, bufdm) => {
                let vdata = bufdm.get_bufvu8(ctxt, "FuzzerK:VM:Op:LetBuf:GetSrcData");
                log_d(&format!("DBUG:VM:Op:LetBuf:{}:{:?}", bufid, vdata));
                ctxt.bufs.insert(bufid.to_string(), vdata);
            }
            Self::LetBufStr(bufid, bufdm) => {
                let vdata = bufdm.get_string(ctxt, "FuzzerK:VM:Op:LetBufStr:GetSrcData");
                log_d(&format!("DBUG:VM:Op:LetBufStr:{}:{:?}", bufid, vdata));
                ctxt.bufs.insert(bufid.to_string(), Vec::from(vdata));
            }
            Self::Buf8Randomize(bufid, dmrandcount, dmstartoffset, dmendoffset, dmstartval, dmendval) => {
                let b8rmsg = "FuzzerK:VM:Op:Buf8Randomize";
                let mut buf = ctxt.bufs.get(bufid).expect(&format!("ERRR:{}:Buf:{}", b8rmsg, bufid)).clone();

                let randcount = dmrandcount.get_isize(ctxt, &format!("{}:RandCount", b8rmsg));
                let trandcount;
                if randcount < 0 {
                    trandcount = rand::random::<usize>() % buf.len();
                } else {
                    trandcount = randcount as usize;
                }

                let startoffset = dmstartoffset.get_isize(ctxt, &format!("{}:StartOffset", b8rmsg));
                let tstartoffset;
                if startoffset < 0 {
                    tstartoffset = 0;
                } else {
                    tstartoffset = startoffset as usize;
                }

                let endoffset = dmendoffset.get_isize(ctxt, &format!("{}:EndOffset", b8rmsg));
                let tendoffset;
                if endoffset < 0 {
                    tendoffset = buf.len()-1;
                } else {
                    tendoffset = endoffset as usize;
                }

                // TOTHINK: Should I truncate silently or should I panic if truncation required.
                let startval = dmstartval.get_isize(ctxt, &format!("{}:StartVal", b8rmsg)) as u8;
                let endval = dmendval.get_isize(ctxt, &format!("{}:EndVal", b8rmsg)) as u8;

                let mut rng = rand::thread_rng();
                let offsetwidth = tendoffset - tstartoffset + 1;
                let valwidth: u16 = endval as u16 - startval as u16 + 1;
                for _i in 0..trandcount {
                    let curind = tstartoffset + (rng.gen::<usize>() % offsetwidth);
                    let curval = startval + (rng.gen::<u16>() % valwidth) as u8;
                    buf[curind] = curval;
                }
                ctxt.bufs.insert(bufid.to_string(), buf);
            }
            Self::BufsMerge(destbufid, srcbufids) => {
                //let destbuf = ctxt.bufs.get_mut(destbufid).expect(&format!("ERRR:FuzzerK:VM:Op:BufsMerge:Dest:{}", destbufid));
                let mut destbuf = Vec::new();
                for srcbufid in srcbufids {
                    let srcbuf = ctxt.bufs.get_mut(srcbufid).expect(&format!("ERRR:FuzzerK:VM:Op:BufsMerge:SrcBuf:{}", srcbufid));
                    let mut dupbuf = srcbuf.clone();
                    destbuf.append(&mut dupbuf);
                }
                log_d(&format!("DBUG:VM:Op:BufsMerge:{}:{:?}", destbufid, destbuf));
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
            log_d(&format!("IN :{}\n", l));
            let nl = datautils::remove_extra_whitespaces(l);
            log_d(&format!("OUT:{}\n", nl));
            let mut tl = nl.clone();
            while tl.len() > 0 {
                let (tok, tlnext) = datautils::next_token(&tl).unwrap();
                log_d(&format!("[{}]=>\n\t[{}],\n\t[{}]", tl, tok, tlnext));
                tl = tlnext;
            }
            ops.push(nl.to_string());
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
        runcmds.push("letint loopcnt $0".to_string());
        runcmds.push("!label freshstart".to_string());
        runcmds.push(format!("iobnew srvX {} {}", ioaddr, ioargs));
        runcmds.push(format!("fcget {} fuzzgot", fc));
        runcmds.push("iobwrite srvX fuzzgot".to_string());
        runcmds.push("iobflush srvX".to_string());
        runcmds.push("inc loopcnt".to_string());
        runcmds.push(format!("iflt.i loopcnt ${} goto freshstart", loopcnt));
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
