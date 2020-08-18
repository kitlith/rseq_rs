use rseq_rs::{container, instructions::{OptionalInst, Instruction, U16Parameters}, CookieFile};
use structopt::StructOpt;
use std::path::PathBuf;
use std::fs::File;
use std::error::Error;
use std::io::Read;
use nom::combinator::cut;
use nom::number::Endianness;
use cookie_factory::gen;

#[derive(StructOpt, Debug)]
#[structopt(name = "rseq-tempo")]
struct Options {
    #[structopt(parse(from_os_str))]
    input: PathBuf,
    #[structopt(short = "t", long = "target")]
    target: f64,
    #[structopt(parse(from_os_str))]
    output: Option<PathBuf>,
}

fn tempo_stretch(source: u16, target: f64, value: u64) -> u64 {
    (value as u128 * target as u128 / source as u128) as u64
}

fn main() -> Result<(), Box<dyn Error>> {
    let Options { input, output, target } = Options::from_args();
    let bytes: Result<Vec<u8>, _> = File::open(&input)?.bytes().collect();
    let bytes = bytes?;

    let (_, mut rseq) = cut(container::parse::<nom::error::VerboseError<&[u8]>>)(&bytes).unwrap();

    let mut current = None;

    // TODO: fancy bpm changes might make this incorrect, should really go down each track in turn instead of just a single list of instructions
    for instruction in &mut rseq.instructions {
        match instruction {
            OptionalInst::Instruction(ref mut inst) => match inst {
                Instruction::SetU16Param { param: U16Parameters::Tempo, value: ref mut tempo } => {
                    current = Some(*tempo);
                    *tempo = target as u16;
                },
                Instruction::Note { ref mut len, .. } => *len = tempo_stretch(current.unwrap(), target, *len),
                Instruction::Rest(ref mut len) => *len = tempo_stretch(current.unwrap(), target, *len),
                _ => ()
            },
            _ => ()
        }
    }

    let output = output.unwrap_or_else(|| {
        let mut new_name = input.file_stem().unwrap().to_owned();
        new_name.push(format!("_tempo{}.brseq", target));
        input.with_file_name(new_name)
    });

    let mut file = File::create(output)?;

    gen(container::gen(&rseq, Endianness::Big), CookieFile(&mut file))?;
    Ok(())
}