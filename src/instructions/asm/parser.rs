use crate::instructions::{OptionalInst, Instruction, U8Parameters, U16Parameters, UserOp, Destination, VarInt};

use nom::error::{ParseError, context, ErrorKind};
use nom::{IResult, Err, Offset, InputTakeAtPosition};

// TODO: represent byte?
struct Line {
    label: Option<String>,
    conditional: bool,
    instruction: Option<Instruction>
}

fn parse_label(input: &str) -> IResult<&str, &str> {
    input.split_at_position1_complete(|item| !(item.is_alphanum() || item == '_' || item == '-'))
}

fn parse_opcode(input: &str) -> IResult<&str, &str> {
    input.split_at_position1_complete(|item| !(item.is_alphanum() || item == '_'))
}

fn parse_instruction(input: &str) -> IResult<&str, Instruction> {
    let (input, op) = parse_opcode(input)?;
    let (input, _) = space1(input)?;

    match op {
        "note" => unimplemented!(),
        "rest" => unimplemented!(),
        "fork" => unimplemented!(),
        "jump" => unimplemented!(),
        "call" => unimplemented!(),
        "start_loop" => unimplemented!(),
        "print" => unimplemented!(),
        "process" => unimplemented!(),
        "end_loop" => Ok((input, Instruction::LoopEnd)),
        "ret" => Ok((input, Instruction::Return)),
        "end_track" => Ok((input, Instruction::EndOfTrack)),
        "set" => unimplemented!(),
        unknown => unimplemented!()
    }
}

fn parse_line(input: &str) -> IResult<&str, Line> {
    let (input, _) = space0(input)?;
    // "label: "
    let (input, (label, _, _)) = opt(tuple((parse_label, tag(':'), space0)))(input)?;
    // '?'
    let (input, is_opt) = map(opt(tag('?')), |mark| mark.is_some())(input)?;

    // ".byte <blah>"
    // TODO

    // "instruction args"
    let (input, instruction) = parse_instruction(input)?;

    let (input, _) = space0(input)?;
    // "#comment"
    let (input, _) = opt(tuple((tag('#'), not_line_ending)))(input)?;
    // eol
    let (input, _) = line_ending(input)?;

    Ok((input, Line { label, conditional: is_opt, instruction }))
}
