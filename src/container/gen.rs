use super::RSEQ;
use crate::gen::*;
use crate::instructions::{OptionalInst, self};

use std::io::Write;
use std::collections::HashMap;
use std::iter::IntoIterator;
use nom::number::Endianness;
use cookie_factory::{SerializeFn, BackToTheBuffer, Seek, WriteContext, GenError};
use cookie_factory::combinator::{slice, back_to_the_buffer, string};
use cookie_factory::sequence::{tuple, pair};
use cookie_factory::multi::many_ref;
use cookie_factory::bytes::be_u8;

fn gen_section<W: Write + BackToTheBuffer, F: SerializeFn<W>>(name: [u8; 4], endian: Endianness, func: F) -> impl SerializeFn<W> {
    pair(
        slice(name), // section name
        back_to_the_buffer(4, // section length
            gen_len(pair(func, gen_align(32))),
            move |ctx, len| gu32((len + 8) as u32, endian)(ctx)
        )
    )
}

fn gen_data_section<'a, W: Write + BackToTheBuffer + Seek>(instructions: &'a Vec<OptionalInst>, endian: Endianness) -> impl Fn(WriteContext<W>) -> Result<(WriteContext<W>, Vec<(u32, String)>), GenError> + 'a {
    move |ctx| {
        // Workaround: cookie_factory wants Fn, not FnMut.
        let labels = std::sync::Mutex::new(None);
        let ret = gen_section(*b"DATA", endian, // section name, len
            pair(gu32(0xC, endian), // section header len
                |ctx| {
                    let (ctx, lab) = instructions::bin::gen_instructions(&instructions, endian)(ctx)?;
                    *labels.lock().unwrap() = Some(lab);
                    Ok(ctx)
                }
            )
        )(ctx);
        let ret = ret.map(|ctx| (ctx, labels.into_inner().unwrap().unwrap()));
        ret
    }

}

fn gen_labl_section<W: Write + BackToTheBuffer>(labels: Vec<(u32, String)>, endian: Endianness) -> impl SerializeFn<W> {
    let len = labels.len();
    gen_section(*b"LABL", endian,
        tuple((
            gu32(len as u32, endian), // number of labels
            back_to_the_buffer(
                4 * len, // label offsets
                move |ctx| {
                    labels.iter()
                    .try_fold((ctx, Vec::new()), |(ctx, mut lengths), (addr, label)| {
                        let (ctx, len) = gen_len(tuple((
                            gu32(*addr, endian), // data addr
                            gu32(label.len() as u32, endian), // label len
                            string(label), // label data
                            be_u8(0),
                            gen_align(4),
                        )))(ctx)?;
                        lengths.push(len); // stick the length of all of that into a vec
                        Ok((ctx, lengths))
                    })
                },
                move |ctx, len_list| {
                    // use the vec of lengths from the previous section to generate the offsets
                    len_list.into_iter().try_fold((ctx, 4 + 4*len as u64), |(ctx, offset), len| {
                        gu32(offset as u32, endian)(ctx).map(|ctx| (ctx, offset + len))
                    }).map(|(ctx, _)| ctx)
                }
            ),
            //gen_align(16)
        ))
    )
}

pub fn gen_rseq<'a, W: Write + BackToTheBuffer + Seek + 'a>(rseq: &'a RSEQ, endian: Endianness) -> impl SerializeFn<W> + 'a {
    tuple((
        slice(&b"RSEQ"),
        bom(endian),
        gu16(0x100, endian), // version
        back_to_the_buffer( // file size
            4,
            gen_len(tuple((
                gu16(0x20, endian), // header size
                gu16(2, endian), // section count
                back_to_the_buffer( // section headers
                    16,
                    move |ctx| {
                        let start_pos = ctx.position;
                        let (ctx, mut labels) = gen_data_section(&rseq.instructions, endian)(ctx)?;
                        //let ctx = slice(&rseq.data)(ctx)?;
                        let mid_pos = ctx.position;

                        labels.sort_unstable_by(|(_, a), (_, b)| a.cmp(b));
                        let ctx = gen_labl_section(labels, endian)(ctx)?;

                        let end_pos = ctx.position;
                        Ok((ctx, (mid_pos - start_pos, end_pos - mid_pos)))
                    },
                    move |ctx, (data_len, labl_len)| tuple((
                        gu32(0x20, endian), // data_offset
                        gu32(data_len as u32, endian), // data_len
                        gu32(0x20 + data_len as u32, endian), // labl_offset
                        gu32(labl_len as u32, endian) // labl_len
                    ))(ctx)
                )
            ))),
            move |ctx, pos| gu32((pos + 0xC) as u32, endian)(ctx)
        )
    ))
}
