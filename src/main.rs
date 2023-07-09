use std::{env, fmt::Display, fs, mem::size_of};

const MEM_MAX: usize = 1 << 13;
const REG_NAMES: [&str; 15] = [
    "%rax", "%rcx", "%rdx", "%rbx", "%rsp", "%rbp", "%rsi", "%rdi", "%r08", "%r09", "%r10", "%r11",
    "%r12", "%r13", "%r14",
];

#[derive(Debug)]
enum StepMode {
    NoStep,
    Stage,
    Cycle,
    Debug,
}

fn parse_args() -> (String, StepMode) {
    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.len() < 1 {
        todo!("handle arg error");
    }
    let file = args.remove(0);

    let step_mode = if env::args().any(|e| e == "-c") {
        StepMode::Cycle
    } else if env::args().any(|e| e == "-s") {
        StepMode::Stage
    } else if env::args().any(|e| e == "-d") {
        StepMode::Debug
    } else {
        StepMode::NoStep
    };

    return (file, step_mode);
}

enum Status {
    Halt,
    Aok,
    Err,
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            Status::Halt => write!(f, "STAT: HLT"),
            Status::Aok => write!(f, "STAT: AOK"),
            Status::Err => write!(f, "STAT: ERR"),
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

struct Machine {
    mem: Vec<u8>,
    step_mode: StepMode,
    regs: Vec<u64>,
    flags: Flags,
    status: Status,
    cycle: usize,
    pc: usize,
}

impl Machine {
    fn new(mem_size: usize, step_mode: StepMode) -> Machine {
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

    fn load(&mut self, file: String) -> Result<(), anyhow::Error> {
        for line in file.lines() {
            if !line.starts_with("0x") {
                continue;
            }
            let colon = match line.find(":") {
                Some(i) => i,
                None => {
                    println!("contd on colon");
                    continue;
                }
            };

            let addr_str = line.get(2..colon).unwrap();
            let mut addr = usize::from_str_radix(addr_str, 16)?;

            let pipe = match line.find("|") {
                Some(i) => i,
                None => {
                    println!("contd on pipe");
                    continue;
                }
            };

            let enc = line.get(colon + 1..pipe).unwrap().trim().as_bytes();
            let mut i = 0;
            //let addrs = addr;
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
            //println!("{}:{} {}", addrs, addr, enc.len());
            //println!("{:02x?} ||| {}", self.mem.get(addrs..addr), line);
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
            if *val > 0 {
                str.push_str(&format!("{}: 0x{:016x}\n", REG_NAMES[i], val));
            }
        }

        return str;
    }
}

impl Display for Machine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\nCycle Count: {}\n\n", self.cycle)?;
        write!(f, "{}\n", self.format_mem())?;
        write!(f, "{}\n", self.format_regs())?;
        write!(f, "{}\n", self.flags)?;
        write!(f, "{}\n", self.status)?;
        return write!(f, "PC: {:04x}", self.pc);
    }
}

fn main() -> Result<(), anyhow::Error> {
    let (infile, mode) = parse_args();
    let infile = fs::read_to_string(infile)?;
    let mut machine = Machine::new(MEM_MAX, mode);
    machine.load(infile)?;
    println!("{machine}");
    return Ok(());
}
