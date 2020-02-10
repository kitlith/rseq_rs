use super::{Instruction, U8Parameters, U16Parameters, VarInt, OptionalInst, UserOp};
use crate::parse::*;

use nom::error::{ParseError, context, ErrorKind};
use nom::{IResult, Err, Offset};
use nom::sequence::{pair, tuple, preceded};
use nom::bytes::complete::{take_till, tag};
use nom::number::complete::{be_u8, be_u16};
use nom::number::Endianness;
use nom::combinator::{map, verify, map_opt};
use num_traits::FromPrimitive;
use nom::branch::alt;
use nom::multi::many0;
use std::collections::HashMap;
use nom::lib::std::fmt::Error;
use std::sync::Mutex;

fn varint<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], VarInt, E> {
    let (rest, (list, last)) = pair(take_till(|c| c & 0x80 == 0), be_u8)(input)?;

    let mut result: VarInt = 0;
    for c in list {
        result = (result << 7) | (c & 0x7F) as VarInt;
    }
    return Ok((rest, (result << 7) | last as VarInt));
}

fn parse_userproc<'a, E: ParseError<&'a [u8]>>(input: &'a [u8], endian: Endianness) -> IResult<&'a [u8], Instruction, E> {
    let (input, op) = map_opt(be_u8, UserOp::from_u8)(input)?;

    match op {
        UserOp::User => map(
            pi16(endian),
            |imm| Instruction::UserProcess { op, var: 0xFF, imm}
        )(input),
        _ => map(
            pair(be_u8, pi16(endian)),
            |(var, imm)| Instruction::UserProcess {op, var, imm}
        )(input)
    }
}

fn parse_instr<'a, E: ParseError<&'a [u8]>>(input: &'a [u8], endian: Endianness) -> IResult<&'a [u8], Instruction, E> {
    let (rest, tag) = be_u8(input)?;
    match tag {
        note @ 0..=0x7F => map(
            pair(be_u8, varint),
            |(velocity, len)| Instruction::Note {note, velocity, len}
        )(rest),

        0x80 => map(varint, |len| Instruction::Rest(len))(rest),
        0x81 => map(varint, |inst| Instruction::Instrument(inst))(rest),

        0x88 => map(pair(be_u8, pu32(endian)), |(track, address)| Instruction::Fork {track, address})(rest),
        0x89 => map(pu32(endian), |address| Instruction::Jump(address))(rest),
        0x8A => map(pu32(endian), |address| Instruction::Call(address))(rest),

        0xD4 => map(be_u8, |byte| Instruction::LoopStart(byte))(rest),

        0xD6 => map(be_u8, |byte| Instruction::PrintVar(byte))(rest),

        0xF0 => parse_userproc(rest, endian),

        0xFC => Ok((rest, Instruction::LoopEnd)),
        0xFD => Ok((rest, Instruction::Return)),

        0xFF => Ok((rest, Instruction::EndOfTrack)),

        0xB0 | 0xC0 ..= 0xD3 | 0xD5 | 0xD7 ..= 0xDF => map(
            be_u8,
            |value| Instruction::SetU8Param {param: U8Parameters::from_u8(tag).unwrap(), value}
        )(rest),
        0xE0 | 0xE1 | 0xE3 | 0xFE => map(
            pu16(endian),
            |value| Instruction::SetU16Param {param: U16Parameters::from_u8(tag).unwrap(), value}
        )(rest),

        _ => context("Unknown Instruction", |input: &[u8]| Err(Err::Error(ParseError::from_error_kind(&input[..1], ErrorKind::Switch))))(input)
    }
}

//fn parse_instr_alt<'a, E: ParseError<&'a [u8]>>(input: &'a [u8], endian: Endianness) -> IResult<&'a [u8], Instruction, E> {
//    let note_instr = map(
//        tuple((verify(be_u8, |&c| c <= 0x7F), be_u8, varint)),
//        |(note, velocity, len)| Instruction::Note {note, velocity, len}
//    );
//
//    let u8_params = map(
//        tuple((map_opt(be_u8, U8Parameters::from_u8), be_u8)),
//        |(param, value)| Instruction::SetU8Param {param, value}
//    );
//
//    let u16_params = map(
//        tuple((map_opt(be_u8, U16Parameters::from_u8), be_u16)),
//        |(param, value)| Instruction::SetU16Param {param, value}
//    );
//
//    alt((
//        note_instr,
//        u8_params,
//        u16_params,
//
//        map(preceded(tag(0x80), varint), |len| Instruction::Rest(len)),
//        map(preceded(tag(0x81), varint), |inst|Instruction::Instrument(inst)),
//        map(preceded(tag(0x88), pair(be_u8, p32(endian))), |(track, address)| Instruction::Fork {track, address})
//
//    ))
//}

pub fn parse_instructions<'a, E: ParseError<&'a [u8]>>(input: &'a [u8], endian: Endianness, labels: &mut HashMap<u32, String>) -> IResult<&'a [u8], Vec<OptionalInst>, E> {
    let begin = input;

    // HACK: nom requires Fn instead of FnMut, so I'm wrapping the mutable state in a mutex >_>
    // TODO: this entire method of label parsing is a hack.
    let mutex = Mutex::new(labels);
    // TODO: what if the label isn't at the beginning of an instruction for some reason?
    let label_parser = |i| {
        let mut labels = mutex.lock().unwrap();
        Ok((i, OptionalInst::Label(
            labels.remove(&(begin.offset(i) as u32))
                .ok_or(Err::Error(ParseError::from_error_kind(i, ErrorKind::Verify)))?
        )))
    };

    let instruction = |endian| move |input| parse_instr(input, endian);
    let res = many0(alt((
        label_parser,
        map(instruction(endian), OptionalInst::Instruction),
        map(be_u8, OptionalInst::Byte)
    )))(input);

    res
}