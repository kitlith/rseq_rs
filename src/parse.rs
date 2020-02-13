use nom::error::{ParseError, context};
use nom::number::Endianness;
use nom::IResult;
use nom::u32;
use nom::u16;
use nom::i16;
use nom::number::complete::{be_u24, le_u24};
use nom::branch::alt;
use nom::combinator::value;
use nom::bytes::complete::tag;

// Convenience functions so I can pass runtime endian functions around to combinators.
pub fn pu32<'a, E: ParseError<&'a [u8]>>(endian: Endianness) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], u32, E> {
    move |input| u32!(input, endian)
}

pub fn pu24<'a, E: ParseError<&'a [u8]>>(endian: Endianness) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], u32, E> {
    match endian {
        Endianness::Big => be_u24,
        Endianness::Little => le_u24
    }
}

pub fn pu16<'a, E: ParseError<&'a [u8]>>(endian: Endianness) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], u16, E> {
    move |input| u16!(input, endian)
}

pub fn pi16<'a, E: ParseError<&'a [u8]>>(endian: Endianness) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], i16, E> {
    move |input| i16!(input, endian)
}

pub fn bom<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], Endianness, E> {
    context("Bad BOM Marker", alt((
        value(Endianness::Big, tag(&[0xFE, 0xFF])),
        value(Endianness::Little, tag(&[0xFF, 0xFE]))
    )))(input)
}

#[cfg(test)]
mod test {
    use super::*;
    use nom::error::VerboseError;
    use std::io::Cursor;

    #[test]
    fn test_bom() {
        assert_eq!(bom::<()>(&[0xFF, 0xFE, 0x00]), Ok((&[0x00][..], Endianness::Little)));
        assert_eq!(bom::<()>(&[0xFE, 0xFF, 0x00]), Ok((&[0x00][..], Endianness::Big)));
        assert!(bom::<()>(&[0xFE, 0xFE, 0x00]).is_err());
    }
}
