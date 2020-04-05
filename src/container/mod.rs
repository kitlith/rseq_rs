mod parser;
mod gen;

use crate::instructions::OptionalInst;

pub use parser::parse;
pub use gen::gen_rseq as gen;
use std::collections::HashMap;

#[derive(Debug)]
pub struct RSEQ {
    //pub data: &'a [u8],
    pub instructions: Vec<OptionalInst>,
    //pub labels: HashMap<u32, String>
}
