use rseq_rs::container;
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
            let mut output = File::open(options.output)?;
            for inst in rseq.instructions {
                writeln!(output, "{:?}", inst)?;
            }
            for label in rseq.unused_labels {
                println!("Warning: Label '{}' at 0x{:x} was not emitted.", label.1, label.0);
            }
        },
        Err(Err::Failure(err)) => {
            for (substr, error) in err.errors {
                let offset = bytes.offset(substr);
                println!("{:?} at 0x{:x}", error, offset);
            }
        }
        Err(Err::Incomplete(needed)) => {
            println!("{:?}", needed);
            // for (substr, error) in err.errors {
            //     let offset = bytes.offset(substr);
            //     println!("{:?} at 0x{:x}", error, offset);
            // }
        }
        // Err(Err::Error(_)) should not be possible since I used cut to convert Error to Failure.
        _ => unreachable!()
    }

    Ok(())
}
