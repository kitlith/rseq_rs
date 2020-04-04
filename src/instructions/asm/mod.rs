// mod gen;
//mod parser;

use lalrpop_util::lalrpop_mod;

lalrpop_mod!(asm, "/instructions/asm/asm.rs");
