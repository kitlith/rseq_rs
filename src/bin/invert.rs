use rseq_rs::{container, instructions::{OptionalInst, Instruction}, CookieFile};
use structopt::StructOpt;
use std::path::PathBuf;
use std::fs::File;
use std::error::Error;
use std::io::{Read, Write, Cursor};
use nom::combinator::cut;
use nom::number::Endianness;
use cookie_factory::gen;

#[derive(StructOpt, Debug)]
#[structopt(name = "rseq-invert")]
struct Options {
    #[structopt(parse(from_os_str))]
    input: PathBuf,
    #[structopt(parse(from_os_str))]
    output: Option<PathBuf>
}

fn main() -> Result<(), Box<dyn Error>> {
    let Options { input, output } = Options::from_args();
    let bytes: Result<Vec<u8>, _> = File::open(&input)?.bytes().collect();
    let bytes = bytes?;

    let (_, mut rseq) = cut(container::parse::<nom::error::VerboseError<&[u8]>>)(&bytes).unwrap();

    for mut instruction in &mut rseq.instructions {
        if let OptionalInst::Instruction(Instruction::Note {ref mut note, ..}) = instruction {
            *note = 0x7F - *note;
        }
    }

    let output = output.unwrap_or_else(|| {
        let mut new_name = input.file_stem().unwrap().to_owned();
        new_name.push("_inverted.brseq");
        input.with_file_name(new_name)
    });

    let mut file = File::create(output)?;

    gen(container::gen(&rseq, Endianness::Big), CookieFile(&mut file))?;
    Ok(())
}
