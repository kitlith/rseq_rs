use num_traits::{FromPrimitive, ToPrimitive};
use num_derive::{FromPrimitive, ToPrimitive};
use std::fmt;

pub mod bin;
pub mod asm;

type VarInt = u64;

// mostly based on rseq2midi.cpp and Atlas' BRSEQ documentation

//struct PrefixedInstruction

#[derive(Debug, Clone)]
pub enum Instruction {
    Note { note: u8, velocity: u8, len: VarInt }, // 0x00 - 0x7F (u8, var)
    Rest(VarInt), // 0x80 (var)
    Instrument(VarInt), // 0x81 (var?)
    // 0x82 ..= 0x87 unused
    // for the following... maybe keep labels instead of addresses?
    Fork { track: u8, dest: Destination}, // 0x88 (u8, u24)
    Jump(Destination), // 0x89 (u24)
    Call(Destination), // 0x8A (u24)
    // 0x8B ..= 0x8F unused
    // 0xA0 ..= 0xA5 command prefixes, leaving these unrepresented for now.
    If, // 0xA2, this is technically a prefix instruction but for now it can just be a regular instruction.
    // 0xA6 ..= 0xAF unused
    // 0xB3 ..= 0xBF unused
    LoopStart(u8), // 0xD4 this may actually take a byte...
    PrintVar(u8), // 0xD6 (u8)
    // 0xE2 unused
    // 0xE4 ..= 0xEF unused
    UserProcess { op: UserOp, var: u8, imm: i16}, // 0xF0
    // 0xF1 ..= 0xFB unused
    LoopEnd, // 0xFC
    Return, // 0xFD
    EndOfTrack, // 0xFF

    SetU8Param { param: U8Parameters, value: u8}, // 0xB0 ..= 0xB2 | 0xC0 ..= 0xD3 | 0xD5 | 0xD7 ..= 0xDF ?
    SetU16Param { param: U16Parameters, value: u16}, // 0xE0 | 0xE1 | 0xE3
}

impl Instruction {
    fn get_tag(&self) -> u8 {
        use Instruction::*;
        match self {
            Note { note, .. } => *note,
            Rest(_) => 0x80,
            Instrument(_) => 0x81,
            Fork { .. } => 0x88,
            Jump(_) => 0x89,
            Call(_) => 0x8A,
            If => 0xA2,
            LoopStart(_) => 0xD4,
            PrintVar(_) => 0xD6,
            UserProcess { .. } => 0xF0,
            LoopEnd => 0xFC,
            Return => 0xFD,
            EndOfTrack => 0xFF,

            SetU8Param { param, .. } => param.to_u8().unwrap(),
            SetU16Param { param, .. } => param.to_u8().unwrap()
        }
    }
}

#[derive(Debug, Clone)]
pub enum Destination {
    Label(String),
    // TODO: phase this out.
    //Address(u32)
}

impl Destination {
    fn is_label(&self) -> bool {
        match self {
            Destination::Label(_) => true,
            _ => false
        }
    }

    // fn is_addr(&self) -> bool {
    //     match self {
    //         Destination::Address(_) => true,
    //         _ => false
    //     }
    // }
}

#[derive(Debug, FromPrimitive, ToPrimitive, Copy, Clone)]
pub enum U8Parameters {
    Timebase = 0xB0,
    EnvHold = 0xB1, // (-1..=127)
    Monophonic = 0xB2, // (?)
    Pan = 0xC0, // (u7)
    Volume = 0xC1, // (u7)
    MasterVolume = 0xC2, // (u7)
    Transpose = 0xC3, // (-64..=63)
    Bend = 0xC4, // (i8)
    BendRange = 0xC5, // (u7)
    Priority = 0xC6, // (u7)
    Polyphony = 0xC7, // (bool?) (NoteWait?)
    Tie = 0xC8, // (bool?)
    PortamentoCnt = 0xC9, // (u7) (what is this? does it relate to 0xC4/0xC5 as brseq.py suggests?)
    ModDepth = 0xCA, // (u7)
    ModSpeed = 0xCB, // (u7)
    ModType = 0xCC, // (0..2)
    ModRange = 0xCD, // (u7)
    Portamento = 0xCE, // (bool?)
    PortamentoTime = 0xCF,
    Attack = 0xD0, // (-1..=127)
    Decay = 0xD1, // (-1..=127)
    Sustain = 0xD2, // (-1..=127)
    Release = 0xD3, // (-1..=127)
    Expression = 0xD5, // (Volume2 ????)
    Span = 0xD7, // (u7)
    Cutoff = 0xD8, // (u7)
    FxSendA = 0xD9, // (u7)
    FxSendB = 0xDA, // (u7)
    MainSend = 0xDB, // (u7)
    InitPan = 0xDC, // (u7)
    Mute = 0xDD, // (0..=3)
    FxSendC = 0xDE, // (u7)
    Damper = 0xDF, // (bool?)
}

#[derive(Debug, FromPrimitive, ToPrimitive, Copy, Clone)]
pub enum U16Parameters {
    ModDelay = 0xE0,
    Tempo = 0xE1,
    // 0xE2?
    Sweep = 0xE3,
    TrackUsage = 0xFE,
}

#[derive(Debug, FromPrimitive, ToPrimitive, Copy, Clone, PartialEq, Eq)]
pub enum UserOp {
    Set = 0x80,
    Add = 0x81,
    Sub = 0x82,
    Mul = 0x83,
    Div = 0x84,
    Shift = 0x85,
    Rand = 0x86,
    And = 0x87,
    Or = 0x88,
    Xor = 0x89,
    Not = 0x8A,
    Mod = 0x8B,
    CmpEq = 0x90,
    CmpGe = 0x91,
    CmpGt = 0x92,
    CmpLe = 0x93,
    CmpLt = 0x94,
    CmpNe = 0x96,
    User = 0xE0, // special, no u8
}

#[derive(Debug, Clone)]
pub enum OptionalInst {
    Instruction(Instruction),
    Byte(u8),
    Label(String)
}
