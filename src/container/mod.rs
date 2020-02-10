mod parser;

use std::borrow::Cow;
use crate::instructions::OptionalInst;

pub use parser::parse;
use std::collections::HashMap;

#[derive(Debug)]
pub struct RSEQ<'a> {
    pub data: &'a [u8],
    pub instructions: Vec<OptionalInst>,
    pub unused_labels: HashMap<u32, String>
}
