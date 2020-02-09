mod parser;

use std::borrow::Cow;
use crate::instructions::OptionalInst;

pub use parser::parse;

#[derive(Debug)]
pub struct Label<'a>(pub u32, pub Cow<'a, str>);

#[derive(Debug)]
pub struct RSEQ<'a> {
    pub data: &'a [u8],
    pub instructions: Vec<OptionalInst>,
    pub labels: Vec<Label<'a>>
}
