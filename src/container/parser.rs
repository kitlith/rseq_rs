use nom::{
    IResult,
    number::Endianness,
    u16, u32,
    bytes::complete::{tag, take},
    error::{ParseError, context},
    combinator::{value, verify, map, map_parser},
    sequence::pair,
    branch::alt,
    multi::{count, length_data}
};

use super::{Label, RSEQ};
use crate::parse::*;
use crate::instructions;

fn bom<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], Endianness, E> {
    context("Bad BOM Marker", alt((
        value(Endianness::Big, tag(&[0xFE, 0xFF])),
        value(Endianness::Little, tag(&[0xFF, 0xFE]))
    )))(input)
}

fn parse_label<'a, E: ParseError<&'a [u8]>>(endian: Endianness) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], Label<'a>, E> {
    move |input| {
        let (input, addr) = u32!(input, endian)?;
        let (input, bytes) = length_data(pu32(endian))(input)?;
        Ok((input, Label(addr, String::from_utf8_lossy(bytes))))
    }
}

fn parse_labl_section<'a, E: ParseError<&'a [u8]>>(input: &'a [u8], endian: Endianness) -> IResult<&'a [u8], Vec<Label<'a>>, E> {
    let (input, _) = tag("LABL")(input)?;
    let (input, len) = u32!(input, endian)?;
    let (_rest, relative) = context("Bad section length?", take(len - 0x8))(input)?;
    let (input, cnt) = u32!(input, endian)?;
    let cnt = cnt as usize;


    let res = {
        let label_location = map(pu32(endian), |offset| &relative[offset as usize..]);
        count(map_parser(label_location, parse_label(endian)), cnt)(input)
    };

    res
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
    let (_, instructions) = instructions::parse(data, endian)?;

    Ok((&[][..], RSEQ { data, instructions, labels }))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_varint() {
        assert_eq!(varint(&[0x70, 0x80]), Ok((&[0x80][..], 0x70)));
        assert_eq!(varint(&[0x8F, 0x80, 0x00, 0x14]), Ok((&[0x14][..], 0xF << 14)));
        assert!(varint(&[0x8F, 0x80]).is_err());
    }

    #[test]
    fn test_bom() {
        assert_eq!(bom(&[0xFF, 0xFE, 0x00]), Ok((&[0x00][..], Endianness::Little)));
        assert_eq!(bom(&[0xFE, 0xFF, 0x00]), Ok((&[0x00][..], Endianness::Big)));
        assert!(bom(&[0xFE, 0xFE, 0x00]).is_err());
    }
}
