use cookie_factory::bytes::*;
use cookie_factory::{SerializeFn, WriteContext, GenError, Skip, Seek, GenResult, gen_at_offset};
use cookie_factory::combinator::{slice, skip};
use cookie_factory::sequence::tuple;
use nom::number::Endianness;
use std::io::{Write, SeekFrom, Seek as IoSeek};

pub fn gu32<W: Write>(i: u32, endian: Endianness) -> impl SerializeFn<W> {
    move |out: WriteContext<W>| match endian {
        Endianness::Big => be_u32(i)(out),
        Endianness::Little => le_u32(i)(out)
    }
}

pub fn gu24<W: Write>(i: u32, endian: Endianness) -> impl SerializeFn<W> {
    move |out: WriteContext<W>| match endian {
        Endianness::Big => be_u24(i)(out),
        Endianness::Little => le_u24(i)(out)
    }
}

pub fn gu16<W: Write>(i: u16, endian: Endianness) -> impl SerializeFn<W> {
    move |out: WriteContext<W>| match endian {
        Endianness::Big => be_u16(i)(out),
        Endianness::Little => le_u16(i)(out)
    }
}

pub fn bom<W: Write>(endian: Endianness) -> impl SerializeFn<W> {
    slice(match endian {
        Endianness::Big => [0xFE, 0xFF],
        Endianness::Little => [0xFF, 0xFE]
    })
}

pub fn gen_len<W: Write>(f: impl SerializeFn<W>) -> impl Fn(WriteContext<W>) -> Result<(WriteContext<W>, u64), GenError> {
    move |ctx| {
        let start_pos = ctx.position;
        f(ctx).map(|ctx| { let end_pos = ctx.position; (ctx, end_pos - start_pos)})
    }
}

// pub fn gen_relative_pos<W: Write>(pos: u64) -> impl Fn(WriteContext<W>) -> Result<(WriteContext<W>, u64), GenError> {
//     move |ctx| Ok((ctx, ctx.position - pos))
// }

#[must_use]
pub struct Placeholder<W: Write + Seek> {
    pos: u64,
    phantom: std::marker::PhantomData<WriteContext<W>>
}

impl<W: Write + Seek> Placeholder<W> {
    fn new(pos: u64) -> Placeholder<W> {
        Placeholder {
            pos,
            phantom: std::marker::PhantomData
        }
    }

    pub fn gen(self, func: impl SerializeFn<W>) -> impl SerializeFn<W> {
        // gen_at_offset!(self.pos, func); ?
        move |mut ctx: WriteContext<W>| {
            let current = ctx.seek(SeekFrom::Current(0))?;
            ctx.seek(SeekFrom::Start(self.pos))?;
            // TODO: only allow writing in the reserved space?
            let mut ctx = func(ctx)?;
            ctx.seek(SeekFrom::Start(current))?;
            Ok(ctx)
        }
    }
}

pub fn gen_placeholder<W: Write + Seek>(reserved: usize) -> impl Fn(WriteContext<W>) -> Result<(WriteContext<W>, Placeholder<W>), GenError> {
    move |mut ctx| {
        let pos = ctx.seek(SeekFrom::Current(0))?;
        ctx.seek(SeekFrom::Current(reserved as i64))?;

        Ok((ctx, Placeholder::new(pos)))
    }
}

pub fn gen_align<W: Write>(amount: u64) -> impl SerializeFn<W> {
    // assuming power of 2
    move |ctx: WriteContext<W>| {
        let padding = amount as usize - (ctx.position % amount) as usize;
        let padding = if padding as u64 == amount { 0 } else { padding };
        slice(vec![0; padding])(ctx)
    }
}
