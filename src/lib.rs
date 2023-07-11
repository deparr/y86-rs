use core::mem::size_of;
use std::{
    fmt::Display,
    io::{self, Read},
};

const REG_NAMES: [&str; 15] = [
    "%rax", "%rcx", "%rdx", "%rbx", "%rsp", "%rbp", "%rsi", "%rdi", "%r08", "%r09", "%r10", "%r11",
    "%r12", "%r13", "%r14",
];
const RSP: usize = 4;

fn wait_until_key(target: u8) {
    // this is so bad
    for byte in io::stdin().lock().bytes() {
        if let Ok(key) = byte {
            if key == target {
                break;
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum StepMode {
    NoStep,
    Stage,
    Cycle,
    Debug,
}

pub enum Stage {
    Fetch,
    Decode,
    Execute,
    Memory,
    Writeback,
    PcUpdate,
}

impl Display for Stage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            Stage::Fetch => write!(f, "Fetch"),
            Stage::Decode => write!(f, "Decode"),
            Stage::Execute => write!(f, "Execute"),
            Stage::Memory => write!(f, "Memory"),
            Stage::Writeback => write!(f, "Writeback"),
            Stage::PcUpdate => write!(f, "PC Update"),
        };
    }
}

#[derive(PartialEq)]
enum Status {
    Halt,
    Aok,
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            Status::Halt => write!(f, "STAT: HLT"),
            Status::Aok => write!(f, "STAT: AOK"),
        };
    }
}

struct Flags(bool, bool, bool);

impl Display for Flags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sf = if self.0 { 1 } else { 0 };
        let zf = if self.1 { 1 } else { 0 };
        let of = if self.2 { 1 } else { 0 };
        return write!(f, "SF: {}\tZF: {}\tOF: {}", sf, zf, of);
    }
}

pub struct Machine {
    mem: Vec<u8>,
    step_mode: StepMode,
    regs: Vec<isize>,
    flags: Flags,
    status: Status,
    cycle: usize,
    pc: usize,
}

#[derive(PartialEq)]
enum OpCode {
    Halt,
    Nop,
    Cmov,
    Irmov,
    Rmmov,
    Mrmov,
    Opx,
    Jxx,
    Call,
    Ret,
    Push,
    Pop,
}

impl Display for OpCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            OpCode::Halt => write!(f, "halt"),
            OpCode::Nop => write!(f, "nop"),
            OpCode::Cmov => write!(f, "cmov"),
            OpCode::Irmov => write!(f, "irmov"),
            OpCode::Rmmov => write!(f, "rmmov"),
            OpCode::Mrmov => write!(f, "mrmov"),
            OpCode::Opx => write!(f, "op"),
            OpCode::Jxx => write!(f, "jmp"),
            OpCode::Call => write!(f, "call"),
            OpCode::Ret => write!(f, "ret"),
            OpCode::Push => write!(f, "push"),
            OpCode::Pop => write!(f, "pop"),
        };
    }
}

#[derive(Copy, Clone)]
enum FunCode {
    Add,
    Sub,
    And,
    Xor,
    Ucnd,
    Lte,
    Lt,
    Eq,
    Neq,
    Gte,
    Gt,
    None,
}

impl Display for FunCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            FunCode::Add => write!(f, "add"),
            FunCode::Sub => write!(f, "sub"),
            FunCode::And => write!(f, "and"),
            FunCode::Xor => write!(f, "xor"),
            FunCode::Ucnd => write!(f, "ucnd"),
            FunCode::Lte => write!(f, "lte"),
            FunCode::Lt => write!(f, "lt"),
            FunCode::Eq => write!(f, "eq"),
            FunCode::Neq => write!(f, "neq"),
            FunCode::Gte => write!(f, "gte"),
            FunCode::Gt => write!(f, "gt"),
            FunCode::None => write!(f, "none"),
        };
    }
}

struct CycleState {
    op: OpCode,
    fun: FunCode,
    r_a: usize,
    r_b: usize,
    val_c: isize,
    val_p: usize,
    val_a: isize,
    val_b: isize,
    val_e: isize,
    val_m: isize,
    cnd: bool,
}

impl Machine {
    pub fn new(mem_size: usize, step_mode: StepMode) -> Machine {
        let mem = vec![0; mem_size];
        let regs = vec![0; 15];
        let status = Status::Aok;
        let flags = Flags(false, false, false);
        let cycle = 0;
        let pc = 0;

        Machine {
            mem,
            step_mode,
            regs,
            flags,
            status,
            cycle,
            pc,
        }
    }

    pub fn load(&mut self, file: String) -> Result<(), anyhow::Error> {
        for line in file.lines() {
            if !line.starts_with("0x") {
                continue;
            }
            let colon = match line.find(":") {
                Some(i) => i,
                None => {
                    continue;
                }
            };

            let addr_str = line.get(2..colon).unwrap();
            let mut addr = usize::from_str_radix(addr_str, 16)?;

            let pipe = match line.find("|") {
                Some(i) => i,
                None => {
                    continue;
                }
            };

            let enc = line.get(colon + 1..pipe).unwrap().trim().as_bytes();
            let mut i = 0;
            while i < enc.len() {
                let mut byte = enc[i];
                if byte > b'9' {
                    byte = (byte & 0x0f) + 9;
                } else {
                    byte &= 0x0f;
                }
                byte <<= 4;

                byte |= if enc[i + 1] > b'9' {
                    (enc[i + 1] & 0x0f) + 9
                } else {
                    enc[i + 1] & 0x0f
                };

                if let Some(x) = self.mem.get_mut(addr) {
                    *x = byte;
                } else {
                    break;
                }
                i += 2;
                addr += 1;
            }
            //println!("{:02x?} ||| {}", self.mem.get(addrs..addr), line);
        }
        return Ok(());
    }

    fn get_mem_word(&self, addr: usize) -> Result<isize, anyhow::Error> {
        let mut word = 0;
        let wordsize = size_of::<usize>();
        let bytes = match self.mem.get(addr..addr + wordsize) {
            Some(bytes) => bytes,
            None => anyhow::bail!("get word: bad addr"),
        };

        for (i, byte) in bytes.iter().enumerate() {
            word |= (*byte as isize) << (i << 3);
        }

        return Ok(word);
    }

    fn set_mem_word(&mut self, addr: usize, word: isize) -> Result<(), anyhow::Error> {
        let wordsize = size_of::<usize>();
        let bytes = match self.mem.get_mut(addr..addr + wordsize) {
            Some(bytes) => bytes,
            None => anyhow::bail!("set word: bad addr"),
        };

        for (wbyte, mbyte) in word.to_le_bytes().iter().zip(bytes.iter_mut()) {
            *mbyte = *wbyte
        }

        Ok(())
    }

    // Todo maybe fetch returns the CycleState Object??? would make more sense
    fn fetch(&self, state: &mut CycleState) -> Result<(), anyhow::Error> {
        let (code, fun) = match self.mem.get(self.pc) {
            Some(byte) => (byte / 16, byte & 0x0f),
            None => anyhow::bail!("bad addr"),
        };

        match code {
            0 => {
                state.op = OpCode::Halt;
                state.val_p = self.pc + 1;
            }
            1 => {
                state.op = OpCode::Nop;
                state.val_p = self.pc + 1;
            }
            2 => {
                state.op = OpCode::Cmov;
                state.val_p = self.pc + 2;
                state.fun = match fun {
                    0 => FunCode::Ucnd,
                    1 => FunCode::Lte,
                    2 => FunCode::Lt,
                    3 => FunCode::Eq,
                    4 => FunCode::Neq,
                    5 => FunCode::Gte,
                    6 => FunCode::Gt,
                    _ => anyhow::bail!("bad ifun for cmov"),
                };
                (state.r_a, state.r_b) = match self.mem.get(self.pc + 1) {
                    Some(byte) => ((byte / 16) as usize, (byte & 0x0f) as usize),
                    None => anyhow::bail!("bad addr"),
                };
            }
            3 => {
                state.op = OpCode::Irmov;
                state.val_p = self.pc + 10;
                (state.r_a, state.r_b) = match self.mem.get(self.pc + 1) {
                    Some(byte) => ((byte / 16) as usize, (byte & 0x0f) as usize),
                    None => anyhow::bail!("bad addr"),
                };
                state.val_c = self.get_mem_word(self.pc + 2)?;
            }
            4 | 5 => {
                state.op = if code == 4 {
                    OpCode::Rmmov
                } else {
                    OpCode::Mrmov
                };
                state.val_p = self.pc + 10;
                (state.r_a, state.r_b) = match self.mem.get(self.pc + 1) {
                    Some(byte) => ((byte / 16) as usize, (byte & 0x0f) as usize),
                    None => anyhow::bail!("bad addr"),
                };
                state.val_c = self.get_mem_word(self.pc + 2)?;
            }
            6 => {
                state.op = OpCode::Opx;
                state.fun = match fun {
                    0 => FunCode::Add,
                    1 => FunCode::Sub,
                    2 => FunCode::And,
                    3 => FunCode::Xor,
                    _ => anyhow::bail!("bad ifun for opx"),
                };
                (state.r_a, state.r_b) = match self.mem.get(self.pc + 1) {
                    Some(byte) => ((byte / 16) as usize, (byte & 0x0f) as usize),
                    None => anyhow::bail!("bad addr"),
                };
                state.val_p = self.pc + 2;
            }
            7 => {
                state.op = OpCode::Jxx;
                state.fun = match fun {
                    0 => FunCode::Ucnd,
                    1 => FunCode::Lte,
                    2 => FunCode::Lt,
                    3 => FunCode::Eq,
                    4 => FunCode::Neq,
                    5 => FunCode::Gte,
                    6 => FunCode::Gt,
                    _ => anyhow::bail!("bad ifun for jxx"),
                };
                state.val_c = self.get_mem_word(self.pc + 1)?;
                state.val_p = self.pc + 9;
            }
            8 => {
                state.op = OpCode::Call;
                state.val_c = self.get_mem_word(self.pc + 1)?;
                state.val_p = self.pc + 9;
            }
            9 => {
                state.op = OpCode::Ret;
                state.val_p = self.pc + 1;
            }
            0xa => {
                state.op = OpCode::Push;
                (state.r_a, state.r_b) = match self.mem.get(self.pc + 1) {
                    Some(byte) => ((byte / 16) as usize, (byte & 0x0f) as usize),
                    None => anyhow::bail!("bad addr"),
                };
                state.val_p = self.pc + 2;
            }
            0xb => {
                state.op = OpCode::Pop;
                (state.r_a, state.r_b) = match self.mem.get(self.pc + 1) {
                    Some(byte) => ((byte / 16) as usize, (byte & 0x0f) as usize),
                    None => anyhow::bail!("bad addr"),
                };
                state.val_p = self.pc + 2;
            }
            _ => anyhow::bail!("bad icode"),
        }

        Ok(())
    }

    fn decode(&self, state: &mut CycleState) -> Result<(), anyhow::Error> {
        match state.op {
            OpCode::Rmmov | OpCode::Opx | OpCode::Cmov => {
                state.val_a = match self.regs.get(state.r_a) {
                    Some(&val) => val,
                    None => anyhow::bail!("bad reg in rmmov/opx"),
                };
                state.val_b = match self.regs.get(state.r_b) {
                    Some(&val) => val,
                    None => anyhow::bail!("bad reg in rmmov/opx"),
                }
            }
            OpCode::Mrmov => {
                let (idx, val) = (state.r_b, &mut state.val_b);
                *val = match self.regs.get(idx) {
                    Some(&val) => val,
                    None => anyhow::bail!("bad reg in cmov/mrmov"),
                };
            }
            OpCode::Call => {
                state.val_b = match self.regs.get(RSP) {
                    Some(&val) => val,
                    None => anyhow::bail!("bad reg in call"),
                };
            }
            OpCode::Ret | OpCode::Pop => {
                let rsp = match self.regs.get(RSP) {
                    Some(&val) => val,
                    None => anyhow::bail!("bad reg in ret/pop"),
                };
                state.val_b = rsp;
                state.val_a = rsp;
            }
            OpCode::Push => {
                state.val_a = match self.regs.get(state.r_a) {
                    Some(&val) => val,
                    None => anyhow::bail!("bad reg in push"),
                };
                state.val_b = match self.regs.get(RSP) {
                    Some(&val) => val,
                    None => anyhow::bail!("bad reg in push"),
                };
            }

            _ => (),
        }

        Ok(())
    }

    fn execute(&mut self, state: &mut CycleState) -> Result<(), anyhow::Error> {
        state.val_e = match state.op {
            OpCode::Irmov => state.val_c,
            OpCode::Cmov => {
                state.cnd = self.cond(state.fun);
                if state.cnd {
                    state.val_a
                } else {
                    state.val_b
                }
            }
            OpCode::Rmmov | OpCode::Mrmov => state.val_b + state.val_c,
            OpCode::Opx => match state.fun {
                FunCode::Add => {
                    let (res, of) = state.val_b.overflowing_add(state.val_a);
                    self.flags = Flags(res < 0, res == 0, of);
                    res
                }
                FunCode::Sub => {
                    let (res, of) = state.val_b.overflowing_sub(state.val_a);
                    self.flags = Flags(res < 0, res == 0, of);
                    res
                }
                FunCode::And => {
                    let res = state.val_b & state.val_a;
                    self.flags = Flags(res < 0, res == 0, false);
                    res
                }
                FunCode::Xor => {
                    let res = state.val_b ^ state.val_a;
                    self.flags = Flags(res < 0, res == 0, false);
                    res
                }
                _ => anyhow::bail!("bad fun for Opx"),
            },
            OpCode::Jxx => {
                state.cnd = self.cond(state.fun);
                if state.cnd {
                    state.val_c
                } else {
                    state.val_p as isize
                }
            }
            OpCode::Call | OpCode::Push => state.val_b - 8,
            OpCode::Ret | OpCode::Pop => state.val_b + 8,
            _ => 0,
        };

        Ok(())
    }

    fn memory(&mut self, state: &mut CycleState) -> Result<(), anyhow::Error> {
        match state.op {
            OpCode::Rmmov => self.set_mem_word(state.val_e as usize, state.val_a)?,
            OpCode::Mrmov => {
                state.val_m = self.get_mem_word(state.val_e as usize)?;
            }
            // LEFT OFF HERE FOR CALL/RET/PUSH/POP
            OpCode::Call => self.set_mem_word(state.val_e as usize, state.val_p as isize)?,
            OpCode::Push => self.set_mem_word(state.val_e as usize, state.val_a)?,
            OpCode::Ret | OpCode::Pop => state.val_m = self.get_mem_word(state.val_a as usize)?,
            _ => (),
        };

        Ok(())
    }

    fn writeback(&mut self, state: &mut CycleState) -> Result<(), anyhow::Error> {
        match state.op {
            OpCode::Irmov | OpCode::Cmov | OpCode::Opx => match self.regs.get_mut(state.r_b) {
                Some(reg) => *reg = state.val_e,
                None => anyhow::bail!("bad reg for irmov"),
            },
            OpCode::Mrmov => match self.regs.get_mut(state.r_a) {
                Some(reg) => *reg = state.val_m,
                None => anyhow::bail!("bad reg for mrmov"),
            },
            OpCode::Call | OpCode::Ret | OpCode::Push => match self.regs.get_mut(RSP) {
                Some(reg) => *reg = state.val_e,
                None => anyhow::bail!("bad reg for call/ret/push"),
            },
            OpCode::Pop => {
                match self.regs.get_mut(state.r_a) {
                    Some(reg) => *reg = state.val_a,
                    None => anyhow::bail!("bad reg for pop"),
                };
                match self.regs.get_mut(RSP) {
                    Some(reg) => *reg = state.val_e,
                    None => anyhow::bail!("bad reg for mrmov"),
                };
            }
            _ => (),
        };

        Ok(())
    }

    fn pc_update(&mut self, state: &mut CycleState) -> Result<(), anyhow::Error> {
        if state.op == OpCode::Halt {
            self.status = Status::Halt;
        }
        self.pc = match state.op {
            OpCode::Jxx => state.val_e as usize,
            OpCode::Ret => state.val_m as usize,
            OpCode::Call => state.val_c as usize,
            _ => state.val_p,
        };

        Ok(())
    }

    pub fn run(&mut self) -> Result<(), anyhow::Error> {
        // use match and loop?
        while self.status == Status::Aok {
            match self.step_mode {
                StepMode::Stage | StepMode::Cycle => {
                    println!("{}", self);
                    wait_until_key(0x0a);
                }
                _ => (),
            }

            let mut cycle_state = CycleState {
                op: OpCode::Halt,
                fun: FunCode::None,
                r_a: 0,
                r_b: 0,
                val_c: 0,
                val_p: 0,
                val_a: 0,
                val_b: 0,
                val_e: 0,
                val_m: 0,
                cnd: false,
            };

            self.fetch(&mut cycle_state)?;
            self.do_step(Stage::Fetch, &cycle_state);

            self.decode(&mut cycle_state)?;
            self.do_step(Stage::Decode, &cycle_state);

            self.execute(&mut cycle_state)?;
            self.do_step(Stage::Execute, &cycle_state);

            self.memory(&mut cycle_state)?;
            self.do_step(Stage::Memory, &cycle_state);

            self.writeback(&mut cycle_state)?;
            self.do_step(Stage::Writeback, &cycle_state);

            self.pc_update(&mut cycle_state)?;
            self.do_step(Stage::PcUpdate, &cycle_state);

            self.cycle += 1;
        }

        return Ok(());
    }

    fn format_mem(&self) -> String {
        let mut i = 0;
        let mut str = String::new();
        let wordsize = size_of::<usize>();
        while i < self.mem.len() {
            let bytes = match self.mem.get(i..i + wordsize) {
                Some(val) => val,
                None => panic!("badaddr"),
            };
            i += wordsize;

            if bytes.iter().all(|&e| e == 0) {
                continue;
            }

            str.push_str(&format!(
                "0x{:04x}: {:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}\n",
                i - wordsize,
                bytes[0],
                bytes[1],
                bytes[2],
                bytes[3],
                bytes[4],
                bytes[5],
                bytes[6],
                bytes[7]
            ));
        }

        return str;
    }

    fn format_regs(&self) -> String {
        let mut str = String::new();
        for (i, val) in self.regs.iter().enumerate() {
            if *val != 0 {
                str.push_str(&format!("{}: 0x{:016x}\n", REG_NAMES[i], val));
            }
        }

        return str;
    }

    fn cond(&self, fun: FunCode) -> bool {
        let sf = self.flags.0;
        let zf = self.flags.1;
        let of = self.flags.2;
        return match fun {
            FunCode::Ucnd => true,
            FunCode::Lte => (sf ^ of) || zf,
            FunCode::Lt => sf ^ of,
            FunCode::Eq => zf,
            FunCode::Neq => !zf,
            FunCode::Gte => !(sf ^ of) && zf,
            FunCode::Gt => !(sf ^ of),
            _ => false,
        };
    }

    fn do_step(&self, stage: Stage, state: &CycleState) {
        if self.step_mode == StepMode::Stage {
            println!(
                r#"{}:
icode:ifun = {}:{} rA:rB = {:x}:{:x}
valC = 0x{:016x} valP = 0x{:016x}
valA = 0x{:016x} valB = 0x{:016x}
valE = 0x{:016x} valM = 0x{:016x}
Cnd = {}"#,
                stage,
                state.op,
                state.fun,
                state.r_a,
                state.r_b,
                state.val_c,
                state.val_p,
                state.val_a,
                state.val_b,
                state.val_e,
                state.val_m,
                state.cnd
            );
            wait_until_key(0x0a);
        }
    }
}

impl Display for Machine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\nCycle Count: {}\n", self.cycle)?;
        writeln!(f, "{}", self.format_mem())?;
        writeln!(f, "{}", self.format_regs())?;
        writeln!(f, "{}", self.flags)?;
        writeln!(f, "{}", self.status)?;
        return writeln!(f, "PC: 0x{:04x}", self.pc);
    }
}
