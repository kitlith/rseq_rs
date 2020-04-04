use nom::{
    IResult,
    number::Endianness,
    u32,
    bytes::complete::{tag, take},
    error::{ParseError, context},
    combinator::{value, verify, map, map_parser},
    sequence::pair,
    branch::alt,
    multi::{count, length_data}
};

use super::RSEQ;
use crate::parse::*;
use crate::instructions;
use std::collections::HashMap;
use std::iter::FromIterator;

fn parse_label<'a, E: ParseError<&'a [u8]>>(endian: Endianness) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], (u32, String), E> {
    move |input| {
        let (input, addr) = u32!(input, endian)?;
        let (input, bytes) = length_data(pu32(endian))(input)?;
        Ok((input, (addr, String::from_utf8_lossy(bytes).into())))
    }
}

fn parse_labl_section<'a, E: ParseError<&'a [u8]>>(input: &'a [u8], endian: Endianness) -> IResult<&'a [u8], HashMap<u32, String>, E> {
    let (input, _) = tag("LABL")(input)?;
    let (input, len) = u32!(input, endian)?;
    let (_rest, relative) = context("Bad section length?", take(len - 0x8))(input)?;
    let (input, cnt) = u32!(input, endian)?;
    let cnt = cnt as usize;


    let res = {
        let label_location = map(pu32(endian), |offset| &relative[offset as usize..]);
        map(
            count(map_parser(label_location, parse_label(endian)), cnt),
            HashMap::from_iter
        )(input)
    };

    res
}

fn parse_data_section<'a, E: ParseError<&'a [u8]>>(input: &'a [u8], endian: Endianness, labels: &HashMap<u32, String>) -> IResult<&'a [u8], Vec<instructions::OptionalInst>, E> {
    let (input, _) = tag("DATA")(input)?;
    let (input, len) = u32!(input, endian)?;
    let (input, hdrlen) = u32!(input, endian)?;

    let (input, _) = take(hdrlen - 0xC)(input)?;
    let (_rest, input) = take(len - hdrlen)(input)?;

    instructions::bin::parse_instructions(input, endian, labels)
}

pub fn parse<'a, E: ParseError<&'a [u8]>>(orig_input: &'a [u8]) -> IResult<&'a [u8], RSEQ<'a>, E> {
    let section_header = |endian| move |input| pair(pu32(endian), pu32(endian))(input);

    let input = orig_input;
    let (input, _) = tag("RSEQ")(input)?;
    let (input, endian) = bom(input)?;
    let (input, _) = context("Bad Version", verify(pu16(endian), |&version| version == 0x100))(input)?;
    let (input, _filesz) = u32!(input, endian)?;

    let (input, _) = context("Unknown header length", verify(pu16(endian), |&hdrlen| hdrlen == 0x20))(input)?;
    let (input, _) = context("Unknown section count", verify(pu16(endian), |&sectcnt| sectcnt == 2))(input)?;
    let (input, data_section) = section_header(endian)(input)?;
    let (_input, labl_section) = section_header(endian)(input)?;

    let data = &orig_input[data_section.0 as usize .. (data_section.0 + data_section.1) as usize];
    let labl = &orig_input[labl_section.0 as usize .. (labl_section.0 + labl_section.1) as usize];
    let (_, labels) = parse_labl_section(labl, endian)?;
    let (_, instructions) = parse_data_section(data, endian, &labels)?;

    Ok((&[][..], RSEQ { data, instructions, labels }))
}
