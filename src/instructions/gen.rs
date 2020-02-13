use super::{OptionalInst, Instruction, U8Parameters, U16Parameters, UserOp, Destination, VarInt};
use crate::gen::*;

use std::io::Write;
use std::collections::HashMap;
use num_traits::ToPrimitive;
//use std::util::num::Integer;
use cookie_factory::{SerializeFn, Seek, WriteContext, GenError};
use cookie_factory::bytes::be_u8;
use cookie_factory::multi::all;
use cookie_factory::gen_simple;
use cookie_factory::sequence::{tuple, pair};
use cookie_factory::combinator::cond;
use nom::number::Endianness;

fn gen_varint<W: Write>(var: VarInt) -> impl SerializeFn<W> {
    let sig_bits = (0 as VarInt).leading_zeros() - var.leading_zeros();
    let mut bytes = (sig_bits + 6) / 7;
    if bytes == 0 { bytes = 1; }
    all((0..bytes).rev().map(move |idx| {
        be_u8(((var >> (idx*7)) as u8) & 0x7F | if idx != 0 { 0x80 } else { 0 })
    }))
}

enum LabelInfo<W: Seek> {
    Label {pos: u64, name: String},
    Placeholder {place: Placeholder<W>, name: String }
}

trait FakeInto<W: Seek> {
    fn conv(self) -> Result<(WriteContext<W>, Option<LabelInfo<W>>), GenError>;
}

impl<W: Seek> FakeInto<W> for Result<WriteContext<W>, GenError> {
    fn conv(self) -> Result<(WriteContext<W>, Option<LabelInfo<W>>), GenError> {
        self.map(|ctx| (ctx, None))
    }
}

pub fn gen_instructions<'a, W: Write + Seek>(instructions: &'a Vec<OptionalInst>, endian: Endianness) -> impl Fn(WriteContext<W>) -> Result<(WriteContext<W>, Vec<(u32, String)>), GenError> + 'a {
    move |mut ctx| {
        let start_pos = ctx.position;
        let mut labels: HashMap<String, (Option<u32>, Vec<Placeholder<W>>)> = HashMap::new();
        for instruction in instructions {
            let (next, label) = gen_optional_inst(instruction, endian)(ctx)?;
            if let Some(info) = label {
                match info {
                    LabelInfo::Label { pos, name } =>
                        labels.entry(name).or_insert_with(|| (None, Vec::new())).0 = Some((pos - start_pos) as u32),
                    LabelInfo::Placeholder { place, name } =>
                        labels.entry(name).or_insert_with(|| (None, Vec::new())).1.push(place)
                }
            }
            ctx = next;
        }

        // Workaround: using mem::replace to trade ctx between iterations of map.
        // This code is kinda meh.
        let mut outer_ctx = Some(ctx);
        let labels: Vec<(u32, String)> = labels.into_iter().map(|(name, (addr, places))| {
            let mut ctx = std::mem::replace(&mut outer_ctx, None).unwrap();
            ctx = places.into_iter().try_fold(ctx, |ctx, p| p.gen(gu24(addr.unwrap(), endian))(ctx))?;
            std::mem::replace(&mut outer_ctx, Some(ctx));

            Ok((addr.unwrap(), name))
        }).collect::<Result<_, GenError>>()?;

        Ok((outer_ctx.unwrap(), labels))
    }
}

fn gen_optional_inst<'a, W: Write + Seek>(inst: &'a OptionalInst, endian: Endianness) -> impl Fn(WriteContext<W>) -> Result<(WriteContext<W>, Option<LabelInfo<W>>), GenError> + 'a {
    move |ctx: WriteContext<W>| match inst {
        OptionalInst::Label(name) => {
            let pos = ctx.position;
            Ok((ctx, Some(LabelInfo::Label {pos, name: name.to_string()})))
        },
        OptionalInst::Byte(b) => be_u8(*b)(ctx).conv(),
        OptionalInst::Instruction(i) => {
            let ctx = be_u8(i.get_tag())(ctx)?;
            match i {
                Instruction::Note { velocity, len, .. } => tuple((be_u8(*velocity), gen_varint(*len)))(ctx).conv(),
                Instruction::Rest(len) | Instruction::Instrument(len) => gen_varint(*len)(ctx).conv(),
                Instruction::Fork { track, dest } => {
                    let ctx = be_u8(*track)(ctx)?;
                    gen_destination(dest.clone(), endian)(ctx)
                },
                Instruction::Jump(dest) | Instruction::Call(dest) =>
                    gen_destination(dest.clone(), endian)(ctx),

                Instruction::LoopStart(b) => be_u8(*b)(ctx).conv(),
                Instruction::PrintVar(b) => be_u8(*b)(ctx).conv(),
                Instruction::UserProcess { op, var, imm } => gen_userop(*op, *var, *imm, endian)(ctx).conv(),

                Instruction::If | Instruction::LoopEnd | Instruction::Return | Instruction::EndOfTrack => Ok((ctx, None)),
                Instruction::SetU8Param { value, .. } => be_u8(*value)(ctx).conv(),
                Instruction::SetU16Param { value, .. } => gu16(*value, endian)(ctx).conv(),
                //_ => unimplemented!()
            }
        }
    }
}

fn gen_userop<W:Write>(op: UserOp, var: u8, imm: i16, endian: Endianness) -> impl SerializeFn<W> {
    tuple((
        be_u8(op.to_u8().unwrap()),
        cond(op != UserOp::User, be_u8(var)),
        gu16(imm as u16, endian)
    ))
}

// This'll get less complicated once I feel confident removing Address from Destination.
fn gen_destination<W: Write + Seek>(dest: Destination, endian: Endianness) -> impl Fn(WriteContext<W>) -> Result<(WriteContext<W>, Option<LabelInfo<W>>), GenError> {
    move |ctx| match &dest {
        Destination::Label(name) => {
            let pos = ctx.position;
            gen_placeholder(3)(ctx).map(|(ctx, place)| (ctx, Some(LabelInfo::Placeholder { place, name: name.clone() })))
        },
        Destination::Address(a) => gu24(*a, endian)(ctx).map(|ctx| (ctx, None))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_varint() {
        let mut target = [0u8; 4];
        gen_simple(gen_varint(0x0FFFFFFF), Cursor::new(&mut target[..]));
        assert_eq!(target, [0xFF, 0xFF, 0xFF, 0x7F]);
        gen_simple(gen_varint(0x08000090), Cursor::new(&mut target[..]));
        assert_eq!(target, [0xC0, 0x80, 0x81, 0x10]);
    }
}
