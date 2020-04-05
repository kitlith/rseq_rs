use rseq_rs::{container, instructions::{OptionalInst, Instruction, UserOp, Destination}};
use structopt::StructOpt;
use std::path::PathBuf;
use std::fs::File;
use std::error::Error;
use std::io::{Read, Write};
use nom::Err;
use nom::Offset;
use nom::combinator::cut;

#[derive(StructOpt, Debug)]
#[structopt(name = "rseq-disassembler")]
struct Options {
    #[structopt(parse(from_os_str))]
    input: PathBuf,
    #[structopt(parse(from_os_str))]
    output: PathBuf
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = Options::from_args();
    let bytes: Result<Vec<u8>, _> = File::open(options.input)?.bytes().collect();
    let bytes = bytes?;

    match cut(container::parse::<nom::error::VerboseError<&[u8]>>)(&bytes) {
        Ok((_, rseq)) => {
            // println!("{:?}", rseq.labels);
            let mut output = File::create(options.output)?;
            for inst in rseq.instructions {
                match inst {
                    OptionalInst::Label(l) => writeln!(output, "{}:", l),
                    OptionalInst::Byte(b) => writeln!(output, ".byte 0x{:x}", b),
                    OptionalInst::Instruction(i) => match i {
                        Instruction::Note { note, velocity, len } => writeln!(output, "note {}, {}, {}", note, velocity, len),
                        Instruction::Rest(len) => writeln!(output, "rest {}", len),
                        Instruction::Instrument(value) => writeln!(output, "set Instrument = {}", value),
                        Instruction::Fork { track, dest: Destination::Label(l) } => writeln!(output, "fork {}, {}", track, l),
                        Instruction::Jump(Destination::Label(l)) => writeln!(output, "jump {}", l),
                        Instruction::Call(Destination::Label(l)) => writeln!(output, "call {}", l),
                        Instruction::If => write!(output, "?"),
                        Instruction::LoopStart(unk) => writeln!(output, "start_loop {}", unk),
                        Instruction::PrintVar(var) => writeln!(output, "print _{}", var),
                        Instruction::UserProcess { op, var, imm } => {
                            let op_str = match op {
                                UserOp::Set => "=",
                                UserOp::Add => "+=",
                                UserOp::Sub => "-=",
                                UserOp::Mul => "*=",
                                UserOp::Div => "/=",
                                UserOp::Rand => "rand",
                                UserOp::And => "&=",
                                UserOp::Or => "|=",
                                UserOp::Xor => "^=",
                                UserOp::Not => "~=",
                                UserOp::Mod => "%=",
                                UserOp::CmpEq => "==",
                                UserOp::CmpGe => ">=",
                                UserOp::CmpGt => ">",
                                UserOp::CmpLe => "<=",
                                UserOp::CmpLt => "<",
                                UserOp::CmpNe => "!=",
                                UserOp::Shift => "<<=", //if imm < 0 {">>="} else {"<<="},
                                UserOp::User => "" // special, ends up ignored.
                            };
                            match op {
                                UserOp::User => writeln!(output, "process 0x{:x}", imm as u16),
                                _ => writeln!(output, "process _{} {} {}", var, op_str, imm)
                            }
                        },
                        Instruction::LoopEnd => writeln!(output, "end_loop"),
                        Instruction::Return => writeln!(output, "ret"),
                        Instruction::EndOfTrack => writeln!(output, "end_track"),

                        Instruction::SetU8Param { param, value } => writeln!(output, "set {:?} = {}", param, value),
                        Instruction::SetU16Param { param, value } => writeln!(output, "set {:?} = {}", param, value),
                        //_ => writeln!(output, "{:?}", i)
                    }
                }?;
            }
            // for label in rseq.unused_labels {
            //     println!("Warning: Label '{}' at 0x{:x} was not emitted.", label.1, label.0);
            // }
        },
        // Err(Err::Failure(err)) => {
        //     for (substr, error) in err.errors {
        //         let offset = bytes.offset(substr);
        //         println!("{} at 0x{:x}", error, offset);
        //     }
        // }
        // Err(Err::Incomplete(needed)) => {
        //     println!("{}", needed);
        // }
        Err(err) => println!("{}", err),
        // Err(Err::Error(_)) should not be possible since I used cut to convert Error to Failure.
        //_ => unreachable!()
    }

    Ok(())
}
