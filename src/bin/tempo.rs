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

struct TempoConvert {
    target: f64,
    current: Option<u16>,
    error: f64
}

impl TempoConvert {
    fn stretch(&mut self, value: u64, change_error: bool) -> u64 {
        if self.error > 2.0 {
            eprintln!("Warning: error has gone above 2 ticks.");
        }
        let ticks = (value as f64 * self.target / self.current.unwrap() as f64) + self.error;
        if change_error {
            self.error = ticks.fract();
        }
        ticks.trunc() as u64
    }

    fn set_tempo(&mut self, value: u16) -> Option<u16> {
        // TODO: convert error to the new tempo if needed?
        let old_current = self.current;
        self.current = Some(value);
        old_current
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let Options { input, output, target } = Options::from_args();
    let bytes: Result<Vec<u8>, _> = File::open(&input)?.bytes().collect();
    let bytes = bytes?;

    let (_, mut rseq) = cut(container::parse::<nom::error::VerboseError<&[u8]>>)(&bytes).unwrap();

    let mut convert = TempoConvert { target, current: None, error: 0.0 };

    // TODO: fancy bpm changes might make this incorrect, should really go down each track in turn instead of just a single list of instructions
    for instruction in &mut rseq.instructions {
        match instruction {
            OptionalInst::Instruction(ref mut inst) => match inst {
                Instruction::SetU16Param { param: U16Parameters::Tempo, value: ref mut tempo } => {
                    convert.set_tempo(*tempo);
                    *tempo = convert.target as u16;
                },
                Instruction::Note { ref mut len, .. } => *len = convert.stretch(*len, false),
                Instruction::Rest(ref mut len) => *len = convert.stretch(*len, true),
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