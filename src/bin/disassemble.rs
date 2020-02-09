use rseq_rs::container;
use structopt::StructOpt;
use std::path::PathBuf;
use std::fs::File;
use std::error::Error;
use std::io::Read;
use nom::Err;
use nom::Offset;

#[derive(StructOpt, Debug)]
#[structopt(name = "rseq-disassembler")]
struct Options {
    #[structopt(parse(from_os_str))]
    input: PathBuf
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = Options::from_args();
    let bytes: Result<Vec<u8>, _> = File::open(options.input)?.bytes().collect();
    let bytes = bytes?;

    match container::parse::<nom::error::VerboseError<&[u8]>>(&bytes) {
        Ok((_, rseq)) => {
            // println!("{:?}", rseq.labels);
            for label in rseq.labels {
                println!("{}", label.1);
            }
        },
        Err(Err::Error(err)) => {
            for (substr, error) in err.errors {
                let offset = bytes.offset(substr);
                println!("{:?} at 0x{:x}", error, offset);
            }
        }
        Err(Err::Failure(err)) => {
            for (substr, error) in err.errors {
                let offset = bytes.offset(substr);
                println!("{:?} at 0x{:x}", error, offset);
            }
        }
        Err(Err::Incomplete(err)) => {
            println!("{:?}", err);
            // for (substr, error) in err.errors {
            //     let offset = bytes.offset(substr);
            //     println!("{:?} at 0x{:x}", error, offset);
            // }
        }
        _ => ()
    }


    Ok(())
}
