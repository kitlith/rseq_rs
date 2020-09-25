use std::collections::{BTreeMap, HashMap, BTreeSet};

use nom::number::Endianness;

use lazy_static::lazy_static;

use arrayvec::ArrayVec;
use std::borrow::Borrow;

#[derive(Clone, Hash, Eq, PartialEq)]
enum InstructionType {
    Note { pitch: u8 },
    Rest,
    Instrument,
    Fork,
    Jump,
    Call,
    LoopStart,
    PrintVar,

    UserProcess(UserOpType),
    // Moved out of UserOp
    UserCall,

    LoopEnd,
    Return,
    EndOfTrack,

    U8Param(U8Parameter),
    U16Param(U16Parameter)
}

#[derive(Clone, Hash, Eq, PartialEq)]
enum UserOpType {
    Set,
    Add,
    Sub,
    Mul,
    Div,
    Shift,
    Rand,
    And,
    Or,
    Xor,
    Not,
    Mod,
    CmpEq,
    CmpGe,
    CmpGt,
    CmpLe,
    CmpLt,
    CmpNe
}

#[derive(Clone, Hash, Eq, PartialEq)]
pub enum U8Parameter {
    Timebase = 0xB0,
    EnvHold = 0xB1,
    Monophonic = 0xB2,
    Pan = 0xC0,
    Volume = 0xC1,
    MasterVolume = 0xC2,
    Transpose = 0xC3,
    Bend = 0xC4,
    BendRange = 0xC5, // (u7)
    Priority = 0xC6, // (u7)
    NoteWait = 0xC7, // (bool?)
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

#[derive(Clone, Hash, Eq, PartialEq)]
pub enum U16Parameter {
    ModDelay = 0xE0,
    Tempo = 0xE1,
    // 0xE2?
    Sweep = 0xE3,
    TrackUsage = 0xFE,
}

#[derive(Clone, Hash, Eq, PartialEq)]
enum PrefixType {
    If,
    // TODO
}

#[derive(Clone, Hash, Eq, PartialEq)]
enum PotentialInstructionType {
    Instruction(InstructionType),
    Prefix(PrefixType)
}

#[derive(Clone, Hash, Eq, PartialEq)]
enum ExtendedType {
    Instruction(PotentialInstructionType),
    Extended
}

pub struct PlatformDef {
    instruction_map: BTreeMap<u8, PotentialInstructionType>,
    extended_ops: BTreeSet<u8>,
    extended_map: BTreeMap<(u8, u8), PotentialInstructionType>,
    rev_instruction_map: HashMap<PotentialInstructionType, ArrayVec<[u8; 2]>>,
    endian: Endianness
}

impl PlatformDef {
    fn parse_type<'a>(&self, input: &'a [u8]) -> Option<(&'a [u8], PotentialInstructionType)> {
        input.get(0).and_then(|op| {
            if self.extended_ops.contains(op) {
                Some((&input[2..], self.extended_map.get(&(*op, *input.get(1)?))?.clone()))
            } else {
                Some((&input[1..], self.instruction_map.get(op)?.clone()))
            }
        })
    }

    fn gen_type(&self, ty: &PotentialInstructionType) -> Option<&[u8]> {
        self.rev_instruction_map.get(ty).map(Borrow::borrow)
    }

    // TODO: parse and gen instructions directly
}

trait SequencePlatform {
    /// default endianess used for writing -- when reading it is auto-detected from file.
    const ENDIAN: Endianness;

    // TODO: container?

    fn match_instruction(opcode: u8) -> Option<ExtendedType>;
    fn match_extended(extension: u8, opcode: u8) -> Option<PotentialInstructionType>;

    fn to_def() -> PlatformDef {
        let mut extended_ops = BTreeSet::new();
        let mut instruction_map = BTreeMap::new();
        for op in 0x00u8..=0xFFu8 {
            match Self::match_instruction(op) {
                Some(ExtendedType::Extended) => {
                    extended_ops.insert(op);
                },
                Some(ExtendedType::Instruction(inst)) => {
                    instruction_map.insert(op, inst);
                },
                None => ()
            }
        }

        let mut extended_map = BTreeMap::new();
        for ext in extended_ops.iter() {
            for op in 0x00u8..=0xFFu8 {
                if let Some(inst) = Self::match_extended(*ext, op) {
                    extended_map.insert((*ext, op), inst);
                }
            }
        }

        let rev_extended_map = extended_map.clone()
            .into_iter()
            .map(|((ext, op), inst)| (inst, ArrayVec::from([ext, op])));

        let rev_instruction_map: HashMap<_, _> = instruction_map.clone()
            .into_iter()
            .map(|(op, inst)| {
                let mut vec = ArrayVec::new();
                vec.push(op);

                (inst, vec)
            })
            .chain(rev_extended_map)
            .collect();

        PlatformDef {
            instruction_map,
            rev_instruction_map,
            extended_ops,
            extended_map,
            endian: Self::ENDIAN
        }
    }
}

struct Revolution;

impl SequencePlatform for Revolution {
    const ENDIAN: Endianness = Endianness::Big;

    fn match_instruction(opcode: u8) -> Option<ExtendedType> {
        Some(ExtendedType::Instruction(PotentialInstructionType::Instruction(match opcode {
            pitch @ 0x00..=0x7F => InstructionType::Note { pitch },

            0x80 => InstructionType::Rest,
            0x81 => InstructionType::Instrument,

            0x88 => InstructionType::Fork,
            0x89 => InstructionType::Jump,
            0x8A => InstructionType::Call,

            0xA2 => return Some(ExtendedType::Instruction(PotentialInstructionType::Prefix(PrefixType::If))),

            0xB0 => InstructionType::U8Param(U8Parameter::Timebase),
            0xB1 => InstructionType::U8Param(U8Parameter::EnvHold),
            0xB2 => InstructionType::U8Param(U8Parameter::Monophonic),

            0xC0 => InstructionType::U8Param(U8Parameter::Pan),
            0xC1 => InstructionType::U8Param(U8Parameter::Volume),
            0xC2 => InstructionType::U8Param(U8Parameter::MasterVolume),
            0xC3 => InstructionType::U8Param(U8Parameter::Transpose),
            0xC4 => InstructionType::U8Param(U8Parameter::Bend),
            0xC5 => InstructionType::U8Param(U8Parameter::BendRange),
            0xC6 => InstructionType::U8Param(U8Parameter::Priority),
            0xC7 => InstructionType::U8Param(U8Parameter::NoteWait),
            0xC8 => InstructionType::U8Param(U8Parameter::Tie),
            0xC9 => InstructionType::U8Param(U8Parameter::PortamentoCnt),
            0xCA => InstructionType::U8Param(U8Parameter::ModDepth),
            0xCB => InstructionType::U8Param(U8Parameter::ModSpeed),
            0xCC => InstructionType::U8Param(U8Parameter::ModType),
            0xCD => InstructionType::U8Param(U8Parameter::ModRange),
            0xCE => InstructionType::U8Param(U8Parameter::Portamento),
            0xCF => InstructionType::U8Param(U8Parameter::PortamentoTime),
            0xD0 => InstructionType::U8Param(U8Parameter::Attack),
            0xD1 => InstructionType::U8Param(U8Parameter::Decay),
            0xD2 => InstructionType::U8Param(U8Parameter::Sustain),
            0xD3 => InstructionType::U8Param(U8Parameter::Release),
            0xD4 => InstructionType::LoopStart,
            0xD5 => InstructionType::U8Param(U8Parameter::Expression),
            0xD6 => InstructionType::PrintVar,
            0xD7 => InstructionType::U8Param(U8Parameter::Span),
            0xD8 => InstructionType::U8Param(U8Parameter::Cutoff),
            0xD9 => InstructionType::U8Param(U8Parameter::FxSendA),
            0xDA => InstructionType::U8Param(U8Parameter::FxSendB),
            0xDB => InstructionType::U8Param(U8Parameter::MainSend),
            0xDC => InstructionType::U8Param(U8Parameter::InitPan),
            0xDD => InstructionType::U8Param(U8Parameter::Mute),
            0xDE => InstructionType::U8Param(U8Parameter::FxSendC),
            0xDF => InstructionType::U8Param(U8Parameter::Damper),
            0xE0 => InstructionType::U16Param(U16Parameter::ModDelay),
            0xE1 => InstructionType::U16Param(U16Parameter::Tempo),

            0xE3 => InstructionType::U16Param(U16Parameter::Sweep),

            0xF0 => return Some(ExtendedType::Extended),

            0xFC => InstructionType::LoopEnd,
            0xFD => InstructionType::Return,
            0xFE => InstructionType::U16Param(U16Parameter::TrackUsage),
            0xFF => InstructionType::EndOfTrack,

            _ => return None
        })))
    }

    fn match_extended(extension: u8, opcode: u8) -> Option<PotentialInstructionType> {
        if extension != 0xF0 {
            return None;
        }

        Some(PotentialInstructionType::Instruction(InstructionType::UserProcess(match opcode {
            0x80 => UserOpType::Set,
            0x81 => UserOpType::Add,
            0x82 => UserOpType::Sub,
            0x83 => UserOpType::Mul,
            0x84 => UserOpType::Div,
            0x85 => UserOpType::Shift,
            0x86 => UserOpType::Rand,
            0x87 => UserOpType::And,
            0x88 => UserOpType::Or,
            0x89 => UserOpType::Xor,
            0x8A => UserOpType::Not,
            0x8B => UserOpType::Mod,

            0x90 => UserOpType::CmpEq,
            0x91 => UserOpType::CmpGe,
            0x92 => UserOpType::CmpGt,
            0x93 => UserOpType::CmpLe,
            0x94 => UserOpType::CmpLt,
            0x95 => UserOpType::CmpNe,

            0xE0 => return Some(PotentialInstructionType::Instruction(InstructionType::UserCall)),

            _ => return None
        })))
    }
}

lazy_static! {
    pub static ref REVOLUTION: PlatformDef = Revolution::to_def();
}
