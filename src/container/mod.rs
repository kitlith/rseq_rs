mod parser;

use std::borrow::Cow;

#[derive(Debug)]
pub struct Label<'a>(pub u32, pub Cow<'a, str>);

#[derive(Debug)]
pub struct RSEQ<'a> {
    pub data: &'a [u8],
    pub labels: Vec<Label<'a>>
}

pub use parser::parse;
