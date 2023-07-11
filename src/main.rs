use std::{env, fs};
use y86_rs::{Machine, StepMode};

const MEM_MAX: usize = 1 << 13;

fn parse_args() -> (String, StepMode) {
    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
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

    (file, step_mode)
}

fn main() -> Result<(), anyhow::Error> {
    let (infile, mode) = parse_args();
    let infile = fs::read_to_string(infile)?;
    let mut machine = Machine::new(MEM_MAX, mode);
    machine.load(infile)?;
    match machine.run() {
        Ok(_) =>  print!("{machine}"),
        Err(e) => return Err(e),
    }
    Ok(())
}
