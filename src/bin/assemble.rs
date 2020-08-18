use rseq_rs::{container, instructions::{OptionalInst, Instruction, asm::AsmParser}, CookieFile};
use structopt::StructOpt;
use std::path::PathBuf;
use std::fs::File;
use std::error::Error;
use std::io::{Read, Write, Cursor, Seek, SeekFrom};
use nom::combinator::cut;
use nom::number::Endianness;
use cookie_factory::gen;

#[derive(StructOpt, Debug)]
#[structopt(name = "rseq-assemble")]
struct Options {
    #[structopt(parse(from_os_str))]
    input: PathBuf,
    #[structopt(parse(from_os_str))]
    output: Option<PathBuf>
}

fn main() -> Result<(), Box<dyn Error>> {
    let Options { input, output } = Options::from_args();
    let mut asm = String::new();
    File::open(&input)?.read_to_string(&mut asm)?;
    let rseq = AsmParser::new().parse(&asm).unwrap();

    let output = File::create(
        output.unwrap_or_else(|| input.with_extension("brseq"))
    )?;
    gen(container::gen(&rseq, Endianness::Big), CookieFile(&mut output))?;
    Ok(())
}
