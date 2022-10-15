//!
//! A virtual program environment
//!
//! HanishKVC, 2022
//!

use std::collections::HashMap;
use std::fs;
use std::thread;
use std::time::Duration;

use loggerk::log_w;
use loggerk::{log_e, log_d};
use rand::Rng;
use crate::datautils;
use crate::iob::IOBridge;
use crate::rtm::RunTimeManager;
use crate::cfgfiles;

mod datas;
use datas::{Variant, VDataType};

struct Context {
    globals: HashMap<String, Variant>,
    iobs: HashMap<String, IOBridge>,
    lbls: HashMap<String, usize>,
    stepu: usize,
    fcrtm: RunTimeManager,
    iptr: usize,
    iptr_commonupdate: bool,
    callretstack: Vec<usize>,
    funcs: HashMap<String, (usize, Vec<String>)>,
    fargsmapstack: Vec<HashMap<String, (isize, String)>>,
    localsstack: Vec<HashMap<String, Variant>>,
    bcompilingfunc: bool,
    compilingfunc: String,
    compilingline: u32,
}

impl Context {
    fn new() -> Context {
        Context {
            globals: HashMap::new(),
            iobs: HashMap::new(),
            lbls: HashMap::new(),
            stepu: 0,
            fcrtm: RunTimeManager::new(),
            iptr: 0,
            iptr_commonupdate: true,
            callretstack: Vec::new(),
            funcs: HashMap::new(),
            fargsmapstack: Vec::new(),
            localsstack: Vec::new(),
            bcompilingfunc: false,
            compilingfunc: String::new(),
            compilingline: 0,
        }
    }
}

impl Context {

    pub fn var_islocal(&self, vname: &str) -> bool {
        let locals = self.localsstack.last().unwrap();
        if locals.contains_key(vname) {
            return true;
        }
        return false;
    }

    ///
    /// This function gets the data variant corresponding to the passed name.
    ///
    /// In order to achieve its goal, it does the below steps.
    ///
    /// If the passed name is a func arg, get name of the original variable
    /// * Func arg can refer to
    ///   * a global variable or
    ///   * a local variable from any of the parent functions
    ///   * or the func arg recieved by the current function, which inturn refers to
    ///     either global or local variables as mentioned before
    ///
    /// Else Check
    /// * if the name corresponds to a local variable
    /// * if the name corresponds to a global variable
    ///
    pub fn var_get(&self, datakind: &DataKind, vname: &str) -> Option<&Variant> {
        let (vnameindex, vname) = self.var_farg2real_ifreqd(datakind, vname);

        let ovhm = match vnameindex {
            -2 => self.localsstack.last(),
            -1 => Some(&self.globals),
            0.. => Some(&self.localsstack[vnameindex as usize]),
            _ => todo!(),
        };
        if ovhm.is_some() {
            let vhm = ovhm.unwrap();
            let vval = vhm.get(&vname);
            if vval.is_some() {
                return vval;
            }
        }
        if vnameindex == -2 {
            let vval = self.globals.get(&vname);
            if vval.is_some() {
                return vval;
            }
        }

        None
    }

    ///
    /// Check if specified name in current local vars set, if so set it there, else set in global var set.
    /// Set bforcelocal to true, if you want to set a new local variable
    ///
    /// FuncArgs are readonly currently and thus cant be set.
    ///
    /// Setting a new variable creates it. bforcelocal helps control whether it is created has a local variable
    /// or a global variable.
    ///
    pub fn var_set(&mut self, datakind: &DataKind, vname: &str, vvalue: Variant, bforcelocal: bool, smsg: &str) {
        if let DataKind::FuncArg = datakind  {
            panic!("ERRR:{}:Ctxt:VarSet:Cant set a funcarg, they are readonly currently", smsg);
        }
        let olocals = self.localsstack.last_mut();
        if olocals.is_some() {
            let locals = olocals.unwrap();
            if locals.contains_key(vname) || bforcelocal {
                locals.insert(vname.to_string(), vvalue);
                return;
            }
        }
        if bforcelocal {
            panic!("ERRR:{}:Ctxt:VarSet:Cant set a local variable, outside a function", smsg);
        }
        self.globals.insert(vname.to_string(), vvalue);
    }

    ///
    /// The returned isize could be
    /// -2   => if the passed name is not a func arg
    /// -1   => if the func arg maps to a global var
    /// >= 0 => if the func arg maps to a local var (then the stack function index)
    ///
    pub fn var_farg2real_ifreqd(&self, datakind: &DataKind, vname: &str) -> (isize, String) {
        if let DataKind::FuncArg = datakind {
            let fargs = self.fargsmapstack.last().expect("ERRR:FuzzerK:VM:Ctxt:FArg2Real:Can be called only from run phase");
            let rname = fargs.get(vname);
            if rname.is_none() {
                panic!("DBUG:FuzzerK:VM:Ctxt:Farg2Real:FArg:{:?}:{}:not in fargsmapstack", datakind, vname);
            }
            log_d(&format!("DBUG:FuzzerK:VM:Ctxt:FArg2Real:{:?}:{}=>{:?}", datakind, vname, rname.unwrap()));
            let rname = rname.unwrap();
            return (rname.0, rname.1.to_string());
        }
        return (-2, vname.to_string());
    }

}

impl Context {

    ///
    /// Map function arguments to the passed arguments in the call.
    ///
    /// Check if the passed arg, is any of the below in this order
    /// * is it a func arg of the current function
    /// * is it a local variable of the current function
    /// * is it a global variable
    ///
    fn func_helper(&mut self, fname: &str, passedargs: &Vec<String>, msgtag: &str) -> (usize, HashMap<String, (isize, String)>) {
        let (fptr, fargs) = self.funcs.get(fname).expect(&format!("ERRR:{}:Ctxt:FuncHelper:{}:Missing???", msgtag, fname));
        // Map farg names of the func to be called to actual var names.
        if fargs.len() != passedargs.len() {
            panic!("ERRR:{}:Ctxt:FuncHelper:Num of required and passed args dont match", msgtag);
        }
        let ocurfargsmap = self.fargsmapstack.last();
        let mut curfargsmap: &HashMap<String, (isize, String)> = &HashMap::new();
        if ocurfargsmap.is_some() {
            curfargsmap = ocurfargsmap.unwrap();
        }
        let mut newfargsmap: HashMap<String, (isize, String)> = HashMap::new();
        for i in 0..passedargs.len() {
            let fargname = &fargs[i];
            let mut baseloc = -2isize;
            let mut basename= &passedargs[i];
            if ocurfargsmap.is_some() {
                let obaseinfo = curfargsmap.get(basename);
                if obaseinfo.is_some() {
                    let baseinfo = obaseinfo.unwrap();
                    baseloc = baseinfo.0;
                    basename = &baseinfo.1;
                }
            }
            // Not a passed func arg, so setup as local variable, if it is the case
            if baseloc == -2 {
                let olvars = self.localsstack.last();
                if olvars.is_some() {
                    let lvars = olvars.unwrap();
                    let olvar = lvars.get(basename);
                    if olvar.is_some() {
                        baseloc = (self.localsstack.len() - 1) as isize;
                    }
                }
            }
            // Not a local var also, so setup as global variable
            if baseloc == -2 {
                baseloc = -1;
            }
            newfargsmap.insert(fargname.to_string(), (baseloc, basename.to_string()));
        }
        log_d(&format!("DBUG:{}:Ctxt:FuncHelper:{}:{:?}:{:?}", msgtag, fptr, fargs, newfargsmap));
        return (*fptr, newfargsmap);
    }

}


#[derive(Debug, PartialEq)]
enum DataKind {
    Variable,
    FuncArg,
}


///
/// NOTE: The program logic currently implements a simple one pass compilation, which inturn
/// only does a partial/quasi ahead of time compilation.
///
/// As a part and result of this mechanism, the DataM::compile
/// * identifies the type of literal value and inturn creates a appropriate variant entity
/// * identifies a possible var name and inturn sets it up as a variable.
///   However do keep in mind that this acts more like a pointer/alias to the actual underlying variable.
///   * the underlying var could be
///     * a function argument (is read only)
///     * a local variable OR
///     * a global variable.
///
#[derive(Debug)]
enum DataM {
    Value(Variant),
    Variable(DataKind, String),
}


impl DataM {

    ///
    /// * int literals should start with numeric char
    /// * string literals should be in double quotes ""
    /// * buf8 literals should start with $0x
    /// * special literals should start with __
    /// * anything else is treated as a Var name.
    ///   * it needs to start with a alpabhetic char
    ///   * it could either be a func arg or local variable or a global variable.
    ///
    fn compile(ctxt: &Context, mut sdata: &str, stype: &str, smsg: &str) -> DataM {
        sdata = sdata.trim();
        if sdata == "" {
            panic!("ERRR:{}:DataM:Compile:{}:Data token empty", smsg, stype);
        }
        let schar = sdata.chars().nth(0).unwrap();
        let echar = sdata.chars().last().unwrap();

        if schar.is_numeric() || schar == '+' || schar == '-' {
            let idata = datautils::intvalue(sdata, &format!("ERRR:{}:DataM:Compile:IntLiteral:Conversion", smsg));
            return DataM::Value(Variant::IntValue(idata));
        }

        if sdata.len() >= 2 {

            if schar == '"' || echar == '"' {
                if schar != echar {
                    panic!("ERRR:{}:DataM:Compile:StringLiteral:Mising double quote at one of the ends:[{}]", smsg, sdata);
                }
                let tdata = datautils::next_token(sdata).expect(&format!("ERRR:{}:DataM:Compile:StringLiteral:Processing...", smsg));
                if tdata.1.len() > 0 {
                    panic!("ERRR:{}:DataM:Compile:StringLiteral:Extra data [{}] beyond end of the string[{}]???", smsg, tdata.1, tdata.0);
                }
                let mut rdata = tdata.0.as_str();
                rdata = rdata.strip_prefix('"').expect(&format!("ERRR:{}:DataM:Compile:StringLiteral:Missing double quote at start of {}", smsg, sdata));
                rdata = rdata.strip_suffix('"').expect(&format!("ERRR:{}:DataM:Compile:StringLiteral:Missing double quote at end of {}", smsg, sdata));
                return DataM::Value(Variant::StrValue(rdata.to_string()));
            }

            if sdata.len() > 2 {
                if sdata.starts_with("$0x") {
                    let bdata = datautils::vu8_from_hex(&sdata[3..]).expect(&format!("ERRR:{}:DataM:Compile:BufHexString:Conversion:{}", smsg, sdata));
                    return DataM::Value(Variant::BufValue(bdata));
                }
                if sdata.starts_with("__") {
                    if sdata == "__TIME__STAMP__" {
                        return DataM::Value(Variant::XTimeStamp);
                    }
                    if sdata.starts_with("__RANDOM__BYTES__") {
                        let (_random, bytelen) = sdata.split_once("__BYTES__").expect(&format!("ERRR:{}:DataM:Compile:RandomBytes:{}", smsg, sdata));
                        let bytelen = usize::from_str_radix(bytelen, 10).expect(&format!("ERRR:{}:DataM:Compile:RandomBytes:{}", smsg, sdata));
                        return DataM::Value(Variant::XRandomBytes(bytelen));
                    }
                    panic!("ERRR:{}:DataM:Compile:{}:Unknown Special Tag {}???", smsg, stype, sdata);
                }
            }

        }

        if !schar.is_alphabetic() {
            panic!("ERRR:{}:DataM:{}:Variable name {} should start with a alphabetic char", smsg, stype, sdata);
        }

        let mut datakind = DataKind::Variable;
        if ctxt.bcompilingfunc {
            let fi = ctxt.funcs.get(&ctxt.compilingfunc).unwrap();
            if fi.1.contains(&sdata.to_string()){
                datakind = DataKind::FuncArg;
            }
        }
        return DataM::Variable(datakind, sdata.to_string());

    }

    ///
    /// Check if I am a value variant or not
    ///
    fn is_value(&self) -> bool {
        match self {
            DataM::Value(_) => true,
            DataM::Variable(_, _) => false,
        }
    }

    ///
    /// Check if I am a variable variant or not
    ///
    #[allow(dead_code)]
    fn is_variable(&self) -> bool {
        match self {
            DataM::Value(_) => false,
            DataM::Variable(_, _) => true,
        }
    }

    ///
    /// This supports infering
    /// * Value's type in both AheadOfTimeCompilation as well as Run phase
    /// * Variable's type only during Run phase (So be careful)
    ///
    fn get_type(&self, ctxt: &Context) -> VDataType {
        match self {
            Self::Value(valv) => {
                return valv.get_type();
            }
            Self::Variable(datakind,vid) => {
                let ovalue = ctxt.var_get(datakind, vid);
                if ovalue.is_some() {
                    return ovalue.unwrap().get_type();
                }
            }
        }
        return VDataType::Unknown;
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
            Self::Value(oval) => {
                return oval.get_isize(smsg);
            }
            Self::Variable(datakind, vid) => {
                let ovalue = ctxt.var_get(datakind, vid);
                if ovalue.is_some() {
                    return ovalue.unwrap().get_isize(&format!("{}:DataM:GetISize:", smsg));
                }
                panic!("ERRR:{}:DataM:GetISize:Var:Unknown:{}", smsg, vid);
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
    /// * XTimeStamp returns milliseconds from UnixEpoch
    /// * XRandomBytes returns random generated bytes converted to string using utf8_lossy
    ///
    fn get_string(&self, ctxt: &mut Context, smsg: &str) -> String {
        match self {
            Self::Value(oval) => {
                return oval.get_string();
            }
            DataM::Variable(datakind, vid) => {
                let ovalue = ctxt.var_get(datakind, vid);
                if ovalue.is_some() {
                    return ovalue.unwrap().get_string();
                }
                panic!("ERRR:{}:DataM:GetString:Var:Unknown:{}", smsg, vid);
            }
        }
    }

    ///
    /// * returns int values as underlying byte values based vector in the native endianess format
    /// * Returns String as the underlying byte values based vector
    /// * Returns Buf8 data as is
    /// * AnyVar follows the order of 1st check IntVars, then StrVars and then finally BufVars
    /// * XTimeStamp -> milliseconds from UnixEpoch, as the underlying byte values of the int
    /// * XRandomBytes returns random generated bytes
    ///
    fn get_bufvu8(&self, ctxt: &mut Context, smsg: &str) -> Vec<u8> {
        match self {
            Self::Value(oval) => {
                return oval.get_bufvu8();
            }
            DataM::Variable(datakind, vid) => {
                let ovalue = ctxt.var_get(datakind, vid);
                if ovalue.is_some() {
                    return ovalue.unwrap().get_bufvu8();
                }
                panic!("ERRR:{}:DataM:GetBuf:Var:Unknown:{}", smsg, vid);
            }
        }
    }

    #[allow(dead_code)]
    fn get_bufvu8_mut<'a>(&'a self, ctxt: &'a mut Context, smsg: &str) -> &'a mut Vec<u8> {
        match self {
            Self::Value(_) => panic!("ERRR:{}:GetBufVu8Mut:Cant return mutable ref to values", smsg),
            Self::Variable(datakind, vid) => {
                if let DataKind::FuncArg = datakind {
                    panic!("ERRR:{}:GetBufVu8Mut:FuncArg cant be used mutably/to-write-to its existing buf, currently", smsg);
                }
                if ctxt.var_islocal(vid) {
                    panic!("ERRR:{}:GetBufVu8Mut:Localvar cant be used mutably/to-write-to its existing buf, currently", smsg);
                }
                let vvalue = ctxt.globals.get_mut(vid).unwrap();
                let gotr = vvalue.get_bufvu8_mut();
                if gotr.is_none() {
                    panic!("ERRR:{}:GetBufVu8Mut:Some issue with getting mut buf wrt:{}", smsg, vid);
                }
                return gotr.unwrap();
            }
        }
    }

    fn set_isize(&self, ctxt: &mut Context, vvalue: isize, smsg: &str) {
        match  self {
            DataM::Value(_) => panic!("ERRR:{}:DataM:SetISize:Cant set a value!", smsg),
            DataM::Variable(datakind, vname) => {
                let vvalue = Variant::IntValue(vvalue);
                ctxt.var_set(datakind, vname, vvalue, false, &format!("{}:DataM:SetISize", smsg));
            }
        }
    }

    #[allow(dead_code)]
    fn set_string(&self, ctxt: &mut Context, vvalue: String, smsg: &str) {
        match  self {
            DataM::Value(_) => panic!("ERRR:{}:DataM:SetString:Cant set a value!", smsg),
            DataM::Variable(datakind, vname) => {
                let vvalue = Variant::StrValue(vvalue);
                ctxt.var_set(datakind, vname, vvalue, false, &format!("{}:DataM:SetString", smsg));
            }
        }
    }

    fn set_bufvu8(&self, ctxt: &mut Context, vvalue: Vec<u8>, smsg: &str) {
        match  self {
            DataM::Value(_) => panic!("ERRR:{}:DataM:SetBuf:Cant set a value!", smsg),
            DataM::Variable(datakind, vname) => {
                let vvalue = Variant::BufValue(vvalue);
                ctxt.var_set(datakind, vname, vvalue, false, &format!("{}:DataM:SetBuf", smsg));
            }
        }
    }

    fn set_value(&self, ctxt: &mut Context, vvalue: Variant, bforcelocal: bool, smsg: &str) {
        match  self {
            DataM::Value(_) => panic!("ERRR:{}:DataM:SetValue:Cant set a value! to a value", smsg),
            DataM::Variable(datakind, vname) => {
                ctxt.var_set(datakind, vname, vvalue, bforcelocal, &format!("{}:DataM:SetValue", smsg));
            }
        }
    }

}


///
/// Support a bunch of condition checks
/// * Uses Lt-Int and Eq-Buf to construct other condition checks
///
#[derive(Debug)]
enum CondOp {
    IfLtInt,
    IfGtInt,
    IfLeInt,
    IfGeInt,
    IfEqBuf,
    IfNeBuf,
}

impl CondOp {

    ///
    /// The following conditions will get transformed into iflt check, as follows
    /// a >  b  ==>  b < a
    /// a <= b  ==>  a < (b+1)
    /// a >= b  ==>  b < (a+1)
    ///
    /// The Ne check gets replaced as follows
    /// a != b  ==>  !(a == b)
    ///
    fn check(&self, ctxt: &mut Context, val1: &DataM, val2: &DataM) -> bool {
        match self {
            CondOp::IfLtInt => {
                let val1 = val1.get_isize(ctxt, "FuzzerK:Vm:CondOp:IfLtInt:Val1");
                let val2 = val2.get_isize(ctxt, "FuzzerK:Vm:CondOp:IfLtInt:Val2");
                log_d(&format!("DBUG:CondOp:IfLtInt:[{}] vs [{}]", val1, val2));
                if val1 < val2 {
                    return true;
                }
                return false;
            },
            CondOp::IfGtInt => {
                return CondOp::IfLtInt.check(ctxt, val2, val1);
            },
            CondOp::IfLeInt => {
                let adjval2 = val2.get_isize(ctxt, "FuzzerK:Vm:CondOp:IfLeInt:Val2") + 1;
                return CondOp::IfLtInt.check(ctxt, val1, &DataM::Value(Variant::IntValue(adjval2)));
            },
            CondOp::IfGeInt => {
                let adjval1 = val1.get_isize(ctxt, "FuzzerK:Vm:CondOp:IfGeInt:Val1") + 1;
                return CondOp::IfLtInt.check(ctxt, val2, &DataM::Value(Variant::IntValue(adjval1)));
            },
            CondOp::IfEqBuf => {
                let val1 = val1.get_bufvu8(ctxt, "FuzzerK:Vm:CondOp:IfEqBuf:Val1");
                let val2 = val2.get_bufvu8(ctxt, "FuzzerK:Vm:CondOp:IfEqBuf:Val2");
                log_d(&format!("DBUG:CondOp:IfEqBuf:[{:?}] vs [{:?}]", val1, val2));
                if val1 == val2 {
                    return true;
                }
                return false;
            },
            CondOp::IfNeBuf => {
                return !CondOp::IfEqBuf.check(ctxt, val1, val2);
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
    LetGlobal(char, DataM, DataM),
    LetLocal(char, DataM, DataM),
    Inc(DataM),
    Dec(DataM),
    Alu(ALUOP, DataM, DataM, DataM),
    IobNew(String, String, HashMap<String, String>),
    IobWrite(String, DataM),
    IobFlush(String),
    IobRead(String, DataM),
    IobClose(String),
    If(CondOp, DataM, DataM, String, String, Vec<String>),
    CheckJump(DataM, DataM, String, String, String),
    Jump(String),
    Call(String, Vec<String>),
    Ret,
    SleepMSec(DataM),
    FcGet(String, DataM),
    BufNew(DataM, DataM),
    Buf8Randomize(DataM, DataM, DataM, DataM, DataM, DataM),
    BufMerged(char, DataM, Vec<DataM>),
}


impl Op {

    fn name_args(ins: &str) -> Result<(String, Vec<String>), String> {
        let parts: Vec<&str> = ins.split_whitespace().collect();
        if parts.len() == 0 {
            return Err(format!("NameArgs:name missing {}", ins));
        }
        let mut vargs: Vec<String> = Vec::new();
        for i in 1..parts.len() {
            vargs.push(parts[i].to_string());
        }
        return Ok((parts[0].to_string(), vargs));
    }

    fn opcompile_opdatatype_autoinfer_ifreqd(sop: &str, ctxt: &Context, srcdm: &DataM, smsg: &str) -> char {
        let (_op, tm) = sop.split_once('.').unwrap_or(("DUMMY", "?")); // extract explicit typemarker if any
        let srctype = match tm {
            "?" => {
                if srcdm.is_value() {
                    match srcdm.get_type(ctxt) {
                        VDataType::Unknown => '?',
                        VDataType::Integer => 'i',
                        VDataType::String => 's',
                        VDataType::Buffer => 'b',
                        VDataType::Special => 'b',
                    }
                } else { // a variable's associated data value type cant be resolved at compile time.
                    '?'
                }
            }
            "b" => 'b',
            "s" => 's',
            "i" => 'i',
            _ => panic!("ERRR:{}:OpDestDataType:Unknown Variant:{}", smsg, sop)
        };
        return srctype;
    }

    fn compile(opplus: &str, ctxt: &mut Context) -> Result<Op, String> {
        let msgtag = &format!("FuzzerK:VM:Op:Compile:{}:", ctxt.compilingline);
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

            "letint" | "letstr" | "letbuf" | "letglobal.i" | "letglobal.s" | "letglobal.b" | "letglobal" => {
                let (vid, sval) = sargs.split_once(' ').expect(&format!("ERRR:{}:LetGlobal:{}:{}", msgtag, sop, sargs));
                let viddm = DataM::compile(ctxt, vid, "any", &format!("{}:LetGlobal:{}:Var:{}", msgtag, sop, vid));
                let valdm = DataM::compile(ctxt, sval, "any", &format!("{}:LetGlobal:{}:Value:{}", msgtag, sop, sval));
                let opdatatype = match sop {
                    "letint" | "letglobal.i" => 'i',
                    "letstr" | "letglobal.s" => 's',
                    "letbuf" | "letglobal.b" => 'b',
                    "letglobal" => Op::opcompile_opdatatype_autoinfer_ifreqd(sop, ctxt, &valdm, &format!("{}:LetGlobal", msgtag)),
                    _ => todo!(),
                };
                return Ok(Op::LetGlobal(opdatatype, viddm, valdm));
            }

            "inc" => {
                let viddm = DataM::compile(ctxt, sargs, "any", &format!("{}:Inc:Var:{}", msgtag, sargs));
                return Ok(Op::Inc(viddm));
            }
            "dec" => {
                let viddm = DataM::compile(ctxt, sargs, "any", &format!("{}:Dec:Var:{}", msgtag, sargs));
                return Ok(Op::Dec(viddm));
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
                let dmdst = DataM::compile(ctxt, args[0], "any", &format!("{}:{}:Dest:{}", msgtag, sop, args[0]));
                let dmsrc1 = DataM::compile(ctxt, args[1], "any", &format!("{}:{}:SrcArg1:{}", msgtag, sop, args[1]));
                let dmsrc2 = DataM::compile(ctxt, args[2], "any", &format!("{}:{}:SrcArg2:{}", msgtag, sop, args[1]));
                return Ok(Op::Alu(aluop, dmdst, dmsrc1, dmsrc2));
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
                let dmsrc = DataM::compile(ctxt, bufid, "any", &format!("{}:{}:Src", msgtag, sop));
                return Ok(Op::IobWrite(ioid.to_string(), dmsrc));
            }
            "iobflush" => {
                return Ok(Op::IobFlush(sargs.to_string()));
            }
            "iobread" => {
                let (ioid, bufid) = sargs.split_once(' ').expect(&format!("ERRR:{}:IobRead:{}", msgtag, sargs));
                let dmdst = DataM::compile(ctxt, bufid, "any", &format!("{}:IobRead:Dst:{}", msgtag, bufid));
                return Ok(Op::IobRead(ioid.to_string(), dmdst));
            }
            "iobclose" => {
                return Ok(Op::IobClose(sargs.to_string()));
            }

            "iflt" | "iflt.i" | "ifgt" | "ifgt.i" | "ifeq" | "ifeq.b" | "ifeq.i" | "ifeq.s" | "ifne" | "ifne.b" | "ifne.i" | "ifne.s" | "ifle" | "ifle.i" | "ifge" | "ifge.i" => {
                let next = datautils::next_token(sargs).unwrap();
                let arg0 = next.0;
                let next = datautils::next_token(&next.1).unwrap();
                let arg1 = next.0;
                let args: Vec<&str> = next.1.splitn(2, ' ').collect();
                let desttype;
                let destdata;
                if args.len() != 2 {
                    panic!("ERRR:{}:{}:InsufficientArgs:{}", msgtag, sop, sargs);
                } else {
                    desttype = args[0];
                    destdata = args[1];
                }
                let val1dm = DataM::compile(ctxt, &arg0, "any", &format!("{}:{}:CheckValue1:{}", msgtag, sop, arg0));
                let val2dm = DataM::compile(ctxt, &arg1, "any", &format!("{}:{}:CheckValue2:{}", msgtag, sop, arg1));
                let cop = match sop {
                    "iflt" | "iflt.i" => CondOp::IfLtInt,
                    "ifgt" | "ifgt.i" => CondOp::IfGtInt,
                    "ifle" | "ifle.i" => CondOp::IfLeInt,
                    "ifge" | "ifge.i" => CondOp::IfGeInt,
                    "ifeq" | "ifeq.b" | "ifeq.i" | "ifeq.s" => CondOp::IfEqBuf,
                    "ifne" | "ifne.b" | "ifne.i" | "ifne.s" => CondOp::IfNeBuf,
                    _ => todo!(),
                };
                let destname;
                let destargs;
                match desttype {
                    "goto" => {
                        destname = destdata.to_string();
                        destargs = Vec::new();
                    }
                    "call" => {
                        let na = Op::name_args(destdata).expect(&format!("ERRR:{}:IfCall", msgtag));
                        destname = na.0;
                        destargs = na.1;
                    }
                    _ => todo!()
                }
                return Ok(Op::If(cop, val1dm, val2dm, desttype.to_string(), destname, destargs));
            }
            "checkjump" => {
                let args: Vec<&str> = sargs.splitn(5, ' ').collect();
                if args.len() != 5 {
                    panic!("ERRR:{}:CheckJump:InsufficientArgs:{}", msgtag, sargs);
                }
                let arg1dm = DataM::compile(ctxt, args[0], "isize", &format!("{}:CheckJump:Arg1:{}", msgtag, args[0]));
                let arg2dm = DataM::compile(ctxt, args[1], "isize", &format!("{}:CheckJump:Arg2:{}", msgtag, args[1]));
                return Ok(Op::CheckJump(arg1dm, arg2dm, args[2].to_string(), args[3].to_string(), args[4].to_string()));
            }
            "jump" | "goto" => {
                return Ok(Op::Jump(sargs.to_string()));
            }
            "call" => {
                let na = Op::name_args(sargs).expect(&format!("ERRR:{}:Call", msgtag));
                return Ok(Op::Call(na.0, na.1));
            }
            "ret" => {
                ctxt.bcompilingfunc = false;
                ctxt.compilingfunc = String::new();
                return Ok(Op::Ret);
            }

            "sleepmsec" => {
                let msecdm = DataM::compile(ctxt, sargs, "isize", &format!("{}:SleepMSec:Value:{}", msgtag, sargs));
                return Ok(Op::SleepMSec(msecdm));
            }

            "fcget" => {
                let (fcid, destvid) = sargs.split_once(' ').expect(&format!("ERRR:{}:FcGet:{}", msgtag, sargs));
                let dm = DataM::compile(ctxt, destvid, "any", &format!("{}:FCGet:Dest:{}", msgtag, destvid));
                return Ok(Op::FcGet(fcid.to_string(), dm));
            }

            "bufnew" => {
                let (bufid, bufsize) = sargs.split_once(' ').expect(&format!("ERRR:{}:BufNew:{}", msgtag, sargs));
                let bufid = DataM::compile(ctxt, bufid, "any", &format!("{}:BufNew:Dest:{}", msgtag, bufid));
                let dmbufsize = DataM::compile(ctxt, bufsize, "any", &format!("{}:BufNew:Size:{}", msgtag, bufsize));
                return Ok(Op::BufNew(bufid, dmbufsize));
            }
            "buf8randomize" => {
                let parts: Vec<&str> = sargs.split(" ").collect();
                let bufid = parts[0].to_string();
                let bufid = DataM::compile(ctxt, &bufid, "any", &format!("{}:Buf8Randomize:TheBuf:{}", msgtag, bufid));

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
                dmrandcount = DataM::compile(ctxt, &thepart, "isize", &format!("{}:Buf8Randomize:RandCount:{}", msgtag, thepart));

                if parts.len() >= 3 {
                    thepart = parts[2].to_string();
                } else {
                    thepart = String::from("-1");
                }
                dmstartoffset = DataM::compile(ctxt, &thepart, "isize", &format!("{}:Buf8Randomize:StartOffset:{}", msgtag, thepart));

                if parts.len() >= 4 {
                    thepart = parts[3].to_string();
                } else {
                    thepart = String::from("-1");
                }
                dmendoffset = DataM::compile(ctxt, &thepart, "isize", &format!("{}:Buf8Randomize:EndOffset:{}", msgtag, thepart));

                if parts.len() >= 5 {
                    thepart = parts[4].to_string();
                } else {
                    thepart = String::from("0");
                }
                dmstartval = DataM::compile(ctxt, &thepart, "isize", &format!("{}:Buf8Randomize:StartVal:{}", msgtag, thepart));

                if parts.len() == 6 {
                    thepart = parts[5].to_string();
                } else {
                    thepart = String::from("255");
                }
                dmendval = DataM::compile(ctxt, &thepart, "isize", &format!("{}:Buf8Randomize:EndVal:{}", msgtag, thepart));

                if parts.len() > 6 {
                    panic!("ERRR:{}:Buf8Randomize:Too many args:{}", msgtag, sargs);
                }
                return Ok(Op::Buf8Randomize(bufid, dmrandcount, dmstartoffset, dmendoffset, dmstartval, dmendval))
            }
            "bufmerged" | "bufmerged.s" | "bufmerged.b" => {
                let (bufid, srcs) = sargs.split_once(' ').expect(&format!("ERRR:{}:BufMerged:Extracting dest from {}", msgtag, sargs));
                let bufid = DataM::compile(ctxt, bufid, "any", &format!("{}:BufMerged:Dest:{}", msgtag, bufid));
                let mut vdm = Vec::new();
                let mut tnext = srcs.to_string();
                while tnext.len() > 0 {
                    let tplus = datautils::next_token(&tnext).expect(&format!("ERRR:{}:BufMerged:Extracting data sources at {}", msgtag, tnext));
                    let dm = DataM::compile(ctxt, &tplus.0, "any", &format!("{}:BufMerged:ProcessingSrc:{}", msgtag, tplus.0));
                    vdm.push(dm);
                    tnext = tplus.1;
                }
                let op_type = sop.split_once('.');
                let mtype;
                if op_type.is_none() {
                    mtype = 'b';
                } else {
                    let op_type = op_type.unwrap();
                    if op_type.1 == "s" {
                        mtype = 's';
                    } else {
                        mtype = 'b';
                    }
                }
                return Ok(Op::BufMerged(mtype, bufid, vdm));
            }
            "letlocal" | "letlocal.b" | "letlocal.s" | "letlocal.i" => {
                let (vid, vdata) = sargs.split_once(' ').expect(&format!("ERRR:{}:LetLocal+:{}", msgtag, sargs));
                let viddm = DataM::compile(ctxt, vid, "any", &format!("{}:LetLocal+:Var:{}", msgtag, vid));
                let datadm = DataM::compile(ctxt, vdata, "any", &format!("{}:LetLocal+:Value:{}", msgtag, vdata));
                let opdatatype = Op::opcompile_opdatatype_autoinfer_ifreqd(sop, ctxt, &datadm, &format!("{}:LetLocal", msgtag));
                return Ok(Op::LetLocal(opdatatype, viddm, datadm));
            }
            _ => panic!("ERRR:{}:UnknownOp:{}", msgtag, sop)
        }
    }

}

impl Op {

    fn oprun_opdatatype_infer(curtypeinfo: char, ctxt: &Context, srcdm: &DataM) -> char {
        let dtype;
        if curtypeinfo == '?' {
            dtype = match srcdm.get_type(ctxt) {
                VDataType::Unknown => '?',
                VDataType::Integer => 'i',
                VDataType::String => 's',
                VDataType::Buffer => 'b',
                VDataType::Special => 'b',
            }
        } else {
            dtype = curtypeinfo;
        }
        return dtype;
    }

    fn run(&self, ctxt: &mut Context, linenum: u32) {
        let msgtag = &format!("FuzzerK:VM:Op:Run:{}", linenum);
        match self {
            Self::Nop => (),

            Self::Inc(vid) => {
                let mut val = vid.get_isize(ctxt, &format!("{}:Inc:{:?}", msgtag, vid));
                val += 1;
                vid.set_isize(ctxt, val, &format!("{}:Inc:{:?}", msgtag, vid));
            }
            Self::Dec(vid) => {
                let mut val = vid.get_isize(ctxt, &format!("{}:Dec:{:?}", msgtag, vid));
                val -= 1;
                vid.set_isize(ctxt, val, &format!("{}:Dec:{:?}", msgtag, vid));
            },
            Self::Alu(aluop, destvid, dmsrc1, dmsrc2) => {
                let src1 = dmsrc1.get_isize(ctxt, &format!("{}:Alu:Src1", msgtag));
                let src2 = dmsrc2.get_isize(ctxt, &format!("{}:Alu:Src2", msgtag));
                let res = match aluop {
                    ALUOP::Add => src1 + src2,
                    ALUOP::Sub => src1 - src2,
                    ALUOP::Mult => src1 * src2,
                    ALUOP::Div => src1 / src2,
                    ALUOP::Mod => src1 % src2,
                };
                destvid.set_isize(ctxt, res, &format!("{}:Alu:{:?}:{:?}", msgtag, aluop, destvid));
            },

            Self::IobNew(ioid, ioaddr, ioargs) => {
                let zenio = ctxt.iobs.get_mut(ioid);
                if zenio.is_some() {
                    let zenio = zenio.unwrap();
                    if let IOBridge::None = zenio {
                    } else {
                        let gotr = zenio.close();
                        if gotr.is_err() {
                            log_e(&format!("ERRR:{}:IobNew:Close4New:{}:{}", msgtag, ioid, gotr.unwrap_err()));
                        }
                    }
                }
                let zenio = IOBridge::new(&ioaddr, &ioargs);
                ctxt.iobs.insert(ioid.to_string(), zenio);
            }
            Self::IobWrite(ioid, srcdm) => {
                let buf = srcdm.get_bufvu8(ctxt, &format!("{}:IobWrite:Getting SrcBuf:{:?}", msgtag, srcdm));
                let zenio = ctxt.iobs.get_mut(ioid).expect(&format!("ERRR:{}:IobWrite:Getting IOB:{}", msgtag, ioid));
                let gotr = zenio.write(&buf);
                if gotr.is_err() {
                    log_e(&format!("ERRR:{}:IobWrite:{}:Writing src:{:?}:{}", msgtag, ioid, srcdm, gotr.unwrap_err()));
                }
            }
            Self::IobFlush(ioid) => {
                let zenio = ctxt.iobs.get_mut(ioid).unwrap();
                let gotr = zenio.flush();
                if gotr.is_err() {
                    log_e(&format!("ERRR:{}:IobFlush:{}:{}", msgtag, ioid, gotr.unwrap_err()));
                }
            }
            Self::IobRead(ioid, bufid) => {
                let buf = &mut bufid.get_bufvu8(ctxt, &format!("{}:IobRead:Getting ToBuf:{:?}", msgtag, bufid));
                let zenio = ctxt.iobs.get_mut(ioid).expect(&format!("ERRR:{}:IobRead:Getting IOB:{}", msgtag, ioid));
                let gotr = zenio.read(buf);
                let readsize;
                if gotr.is_err() {
                    let errmsg = gotr.as_ref().unwrap_err();
                    log_e(&format!("ERRR:{}:IobRead:{}:Reading ToBuf:{:?}:{}", msgtag, ioid, bufid, errmsg));
                    readsize = 0;
                } else {
                    readsize = gotr.unwrap();
                }
                buf.resize(readsize, 0);
                bufid.set_bufvu8(ctxt, buf.to_vec(), &format!("{}:IobRead:Updating ToBuf:{:?}", msgtag, bufid));
            }
            Self::IobClose(ioid) => {
                let zenio = ctxt.iobs.get_mut(ioid).unwrap();
                let gotr = zenio.close();
                if gotr.is_err() {
                    log_e(&format!("ERRR:{}:IobClose:{}:{}", msgtag, ioid, gotr.unwrap_err()));
                }
                ctxt.iobs.remove(ioid);
            }
            Self::SleepMSec(msecdm) => {
                let msec = msecdm.get_usize(ctxt, &format!("{}:SleepMSec:Value:{:?}", msgtag, msecdm));
                thread::sleep(Duration::from_millis(msec as u64));
            }
            Self::FcGet(fcid, vid) => {
                let fci = ctxt.fcrtm.fcimmuts(&fcid).expect(&format!("ERRR:{}:FcGet:UnknownFC???:{}", msgtag, fcid));
                let gotfuzz = fci.get(ctxt.stepu);
                log_d(&format!("\n\nGot:{}:\n\t{:?}\n\t{}", ctxt.stepu, gotfuzz, String::from_utf8_lossy(&gotfuzz)));
                vid.set_bufvu8(ctxt, gotfuzz, &format!("{}:FcGet:SetDest:{:?}", msgtag, vid));
                ctxt.stepu += 1;
            }
            Self::If(cop, val1dm, val2dm, sop , destname, destargs) => {
                let mut opdo = false;
                //log_d(&format!("DBUG:{}:IfLt:{},{},{},{}", msgtag, val1, val2, sop, oparg));
                if cop.check(ctxt, val1dm, val2dm) {
                    opdo = true;
                }
                if opdo {
                    match sop.as_str() {
                        // Translating the label here at runtime, rather than during compile time, allows goto to refer to label
                        // that might not yet have been defined at the point where goto or rather the If condition is encountered.
                        // Especially when only a single pass parsing of the program is done.
                        "goto" | "jump" => {
                            Op::Jump(destname.to_string()).run(ctxt, linenum);
                        }
                        "call" => {
                            Op::Call(destname.to_string(), destargs.clone()).run(ctxt, linenum);
                        }
                        _ => todo!()
                    }
                }
            }
            Self::CheckJump(arg1, arg2, ltlabel, eqlabel, gtlabel) => {
                let varg1 = arg1.get_isize(ctxt, &format!("{}:CheckJump:GetArg1:{:?}", msgtag, arg1));
                let varg2 = arg2.get_isize(ctxt, &format!("{}:CheckJump:GetArg2:{:?}", msgtag, arg2));
                let label;
                if varg1 < varg2 {
                    label = ltlabel;
                } else if varg1 == varg2 {
                    label = eqlabel;
                } else {
                    label = gtlabel;
                }
                if label != "__NEXT__" {
                    ctxt.iptr = *ctxt.lbls.get(label).expect(&format!("ERRR:{}:CheckJump:Label:{}", msgtag, label));
                    ctxt.iptr_commonupdate = false;
                }
            }
            Self::Jump(label) => {
                if label != "__NEXT__" {
                    ctxt.iptr = *ctxt.lbls.get(label).expect(&format!("ERRR:{}:Jump:Label:{}", msgtag, label));
                    ctxt.iptr_commonupdate = false;
                    //log_d(&format!("DBUG:{}:Jump:{}:{}", msgtag, label, ctxt.iptr));
                }
            }
            Self::Call(fname, passedargs) => {
                let (fptr, fargsmap) = ctxt.func_helper(fname, passedargs, &format!("{}:Call:{}", msgtag, fname));
                // Setup the call
                ctxt.callretstack.push(ctxt.iptr);
                ctxt.iptr = fptr;
                ctxt.fargsmapstack.push(fargsmap);
                ctxt.localsstack.push(HashMap::new());
                ctxt.iptr_commonupdate = false;
            }
            Self::Ret => {
                ctxt.iptr = ctxt.callretstack.pop().expect(&format!("ERRR:{}:Ret:CallRetStack", msgtag));
                ctxt.fargsmapstack.pop().expect(&format!("ERRR:{}:Ret:FArgsMapStack", msgtag));
                ctxt.localsstack.pop();
            }

            Self::BufNew(bufid, dmbufsize) => {
                let mut buf = Vec::<u8>::new();
                let bufsize = dmbufsize.get_usize(ctxt, &format!("{}:BufNew:BufSize", msgtag));
                buf.resize(bufsize, 0);
                bufid.set_bufvu8(ctxt, buf, &format!("{}:BufNew:{:?}", msgtag, bufid));
            }
            Self::Buf8Randomize(bufid, dmrandcount, dmstartoffset, dmendoffset, dmstartval, dmendval) => {
                let b8rmsg = &format!("{}:Buf8Randomize", msgtag);
                let mut buf = bufid.get_bufvu8(ctxt, &format!("{}:Getting TheBuf:{:?}", b8rmsg, bufid));

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
                bufid.set_bufvu8(ctxt, buf, &format!("{}:Buf:{:?}:SettingResult", b8rmsg, bufid));
            }
            Self::BufMerged(mtype, destbufdm, srcdms) => {
                let mut destbuf = Vec::new();
                for srcdm in srcdms {
                    let mut sbuf;
                    if *mtype == 'b' {
                        sbuf = srcdm.get_bufvu8(ctxt, &format!("{}:BufMerged.B:Src:{:?}", msgtag, srcdm));
                    } else {
                        let tbuf = srcdm.get_string(ctxt, &format!("{}:BufMerged.S:Src:{:?}", msgtag, srcdm));
                        sbuf = Vec::from(tbuf);
                    }
                    destbuf.append(&mut sbuf);
                }
                log_d(&format!("DBUG:{}:BufMerged:{:?}:{:?}", msgtag, destbufdm, destbuf));
                destbufdm.set_bufvu8(ctxt, destbuf, &format!("{}:BufMerged.{}:{:?}", msgtag, mtype, destbufdm));
            }

            Self::LetGlobal(ltype, vardm, datadm) => {
                // Resolve src data type at runtime, if src was a variable rather than a value
                let dtype = Op::oprun_opdatatype_infer(*ltype, ctxt, datadm);
                let vdata;
                match dtype {
                    'b' => {
                        let tdata = datadm.get_bufvu8(ctxt, &format!("{}:LetGlobal.b:GetSrcData", msgtag));
                        vdata = Variant::BufValue(tdata);
                    }
                    's' => {
                        let tdata = datadm.get_string(ctxt, &format!("{}:LetGlobal.s:GetSrcData", msgtag));
                        vdata = Variant::StrValue(tdata);
                    }
                    'i' => {
                        let tdata = datadm.get_isize(ctxt, &format!("{}:LetGlobal.i:GetSrcData", msgtag));
                        vdata = Variant::IntValue(tdata);
                    }
                    _ => panic!("{}:LetGlobal:GetSrcData:Unknown type:{}", msgtag, ltype),
                }
                log_d(&format!("DBUG:{}:LetGlobal.{}:{:?}:{:?}", msgtag, ltype, vardm, vdata));
                vardm.set_value(ctxt, vdata, false, &format!("{}:LetGlobal:Set the value", msgtag));
            }

            Self::LetLocal(ltype, vardm, datadm) => {
                let vdata;
                // Resolve src data type at runtime, if src was a variable rather than a value
                let dtype = Op::oprun_opdatatype_infer(*ltype, ctxt, datadm);
                match dtype {
                    'b' => {
                        let tdata = datadm.get_bufvu8(ctxt, &format!("{}:LetLocal.b:GetSrcData", msgtag));
                        vdata = Variant::BufValue(tdata);
                    }
                    's' => {
                        let tdata = datadm.get_string(ctxt, &format!("{}:LetLocal.s:GetSrcData", msgtag));
                        vdata = Variant::StrValue(tdata);
                    }
                    'i' => {
                        let tdata = datadm.get_isize(ctxt, &format!("{}:LetLocal.i:GetSrcData", msgtag));
                        vdata = Variant::IntValue(tdata);
                    }
                    _ => panic!("{}:LetLocal:GetSrcData:Unknown type:{}", msgtag, ltype),
                }
                log_d(&format!("DBUG:{}:LetLocal.{}:{:?}:{:?}", msgtag, ltype, vardm, vdata));
                vardm.set_value(ctxt, vdata, true, &format!("{}:LetLocal:Set the value", msgtag));
            }

        }
    }

}


pub struct VM {
    ctxt: Context,
    ops: Vec<(Op,u32)>,
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
        match sdir {
            "!label" => {
                self.ctxt.lbls.insert(sargs.to_string(), self.ops.len());
            }
            "!func" => {
                let parts: Vec<&str> = sargs.split_whitespace().collect();
                if parts.len() == 0 {
                    panic!("ERRR:FuzzerK:VM:CompileDirective:!func:function name missing {}", sdirplus);
                }
                let mut vargs: Vec<String> = Vec::new();
                for i in 1..parts.len() {
                    vargs.push(parts[i].to_string());
                }
                self.ctxt.funcs.insert(parts[0].to_string(), (self.ops.len(),vargs));
                if self.ctxt.bcompilingfunc {
                    panic!("ERRR:FuzzerK:VM:CompileDirective:!func:{}, prev func may be missing ret", sdirplus);
                }
                self.ctxt.bcompilingfunc = true;
                self.ctxt.compilingfunc = parts[0].to_string();
            }
            _ => panic!("ERRR:FuzzerK:VM:CompileDirective:Unknown:{}", sdirplus),
        }
    }

    pub fn compile(&mut self, ops: Vec<String>) {
        self.ctxt.compilingline = 0;
        for sop in ops {
            self.ctxt.compilingline += 1;
            let sop = sop.trim();
            if sop.starts_with("#") || (sop.len() == 0) {
                continue;
            }
            //log_d(&format!("DBUG:FuzzerK:VM:Compile:Op:{}:{}", self.ctxt.compilingline, sop));
            if sop.starts_with("!") {
                self.compile_directive(sop);
                continue;
            }
            let op = Op::compile(sop, &mut self.ctxt).expect(&format!("ERRR:FuzzerK:VM:Compile:Op:{}", sop));
            log_d(&format!("DBUG:FuzzerK:VM:Compiled:Op:{}:{:?}", self.ctxt.compilingline, op));
            self.ops.push((op,self.ctxt.compilingline));
        }
    }

    #[allow(dead_code)]
    fn test_bruteforce_nexttoken(nl: &str) {
        let mut tl = nl.to_string();
        while tl.len() > 0 {
            let (tok, tlnext) = datautils::next_token(&tl).unwrap();
            log_d(&format!("[{}]=>\n\t[{}],\n\t[{}]", tl, tok, tlnext));
            tl = tlnext;
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
            //log_d(&format!("IN :{}\n", l));
            let nl = datautils::remove_extra_whitespaces(l);
            //log_d(&format!("OUT:{}\n", nl));
            //Self::test_bruteforce_nexttoken(&nl);
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
            log_d(&format!("INFO:FuzzerK:VM:Op:ToRun:{}:{}:{:?}", theop.1, self.ctxt.iptr, theop.0));
            self.ctxt.iptr_commonupdate = true;
            theop.0.run(&mut self.ctxt, theop.1);
            if self.ctxt.iptr_commonupdate {
                self.ctxt.iptr += 1;
            }
        }
    }

}
