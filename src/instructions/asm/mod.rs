// mod gen;
//mod parser;

use lalrpop_util::lalrpop_mod;
use crate::instructions::OptionalInst;

lalrpop_mod!(parser, "/instructions/asm/asm.rs");

pub use parser::FileParser as AsmParser;

// #[cfg(test)]
// mod test {
//     use super::*;
//
//     #[test]
//     fn test_number() {
//         let parser = parser::u16Parser::new();
//         assert_eq!(parser.parse("0x1234").unwrap(), 0x1234);
//     }
// }
