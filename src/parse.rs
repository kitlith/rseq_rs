use nom::error::ParseError;
use nom::number::Endianness;
use nom::IResult;
use nom::u32;
use nom::u16;
use nom::i16;

// Convenience functions so I can pass runtime endian functions around to combinators.
pub fn pu32<'a, E: ParseError<&'a [u8]>>(endian: Endianness) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], u32, E> {
    move |input| u32!(input, endian)
}

pub fn pu16<'a, E: ParseError<&'a [u8]>>(endian: Endianness) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], u16, E> {
    move |input| u16!(input, endian)
}

pub fn pi16<'a, E: ParseError<&'a [u8]>>(endian: Endianness) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], i16, E> {
    move |input| i16!(input, endian)
}