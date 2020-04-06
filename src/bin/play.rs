use rseq_rs::{container, instructions::{OptionalInst, Instruction, UserOp, U8Parameters, U16Parameters, Destination}};
use structopt::StructOpt;
use std::path::PathBuf;
use std::fs::File;
use std::error::Error;
use std::io::{Read, Write, Cursor};
use std::collections::HashMap;
use nom::combinator::cut;
use nom::number::Endianness;
use nom::Offset;

use midly::{Smf, MidiMessage, MetaMessage, Event as MidiEvent, EventKind as MidiKind};


#[derive(StructOpt, Debug)]
#[structopt(name = "rseq-disassembler")]
struct Options {
    #[structopt(parse(from_os_str))]
    input: PathBuf,
    #[structopt(parse(from_os_str))]
    output: Option<PathBuf>
}

struct Track<'a> {
    pending_notes: Vec<(u8, u64)>,
    messages: Vec<midly::Event<'a>>,
    index: u8,
    last_pos: u64,
    tick_pos: u64,
    instruction_pos: usize,
    stack: Vec<usize>,
    flag: bool,
    variables: [i16; 48]
}

impl<'a> Track<'a> {
    fn push_event(&mut self, kind: MidiKind<'a>) {
        self.messages.push(MidiEvent { delta: ((self.tick_pos - self.last_pos) as u32).into(), kind});
        self.last_pos = self.tick_pos;
    }

    fn push_midi_event(&mut self, message: MidiMessage) {
        self.push_event(MidiKind::Midi { channel: self.index.into(), message })
    }

    fn push_control_event(&mut self, controller: u8, value: u8) {
        self.push_midi_event(MidiMessage::Controller { controller: controller.into(), value: value.into() })
    }

    fn new(index: u8, tick_pos: u64, instruction_pos: usize) -> Track<'a> {
        let mut ret = Track {
            pending_notes: Vec::new(),
            messages: vec![],
            index,
            last_pos: 0,
            tick_pos,
            instruction_pos,
            stack: Vec::new(),
            flag: false,
            variables: [0i16; 48]
        };

        ret.push_event(MidiKind::Meta(MetaMessage::TrackNumber(Some(index as u16))));

        ret
    }

    fn fork(&self, index: u8, instruction_pos: usize) -> Track<'a> {
        Track::new(index, self.tick_pos, instruction_pos)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let Options { input, output } = Options::from_args();
    let bytes: Result<Vec<u8>, _> = File::open(&input)?.bytes().collect();
    let bytes = bytes?;

    let rseq = match cut(container::parse::<nom::error::VerboseError<&[u8]>>)(&bytes) {
        Err(nom::Err::Failure(err)) => {
            for (substr, error) in err.errors {
                let offset = bytes.offset(substr);
                println!("{:?} at 0x{:x}", error, offset);
            }
            return Ok(())
        },
        Err(nom::Err::Incomplete(needed)) => {
            println!("{:?}", needed);
            return Ok(())
        },
        Err(nom::Err::Error(_)) => unreachable!(),
        Ok((_, rseq)) => rseq
    };

    let labels: HashMap<&String, usize> = rseq.instructions.iter().enumerate().filter_map(|(pos, i)| {
        if let OptionalInst::Label(s) = i {
            Some((s, pos))
        } else {
            None
        }
    }).collect();

    let header = midly::Header::new(midly::Format::Parallel, midly::Timing::Metrical(96.into()));
    let mut midi: Smf = Smf::new(header, Vec::new()).unwrap();

    let mut tracks = vec![Track::new(0, 0, 0)];
    while tracks.len() != 0 {
        let mut track = tracks.pop().unwrap();
        println!("Processing track {}", track.index);
        loop {
            match rseq.instructions[track.instruction_pos].clone() {
                OptionalInst::Label(_) => (),
                OptionalInst::Byte(b) => println!("Warning: Tried to execute unknown byte 0x{:x}", b),
                OptionalInst::Instruction(inst) => match inst {
                    Instruction::Note { note, velocity: vel, len} => {
                        track.push_midi_event(MidiMessage::NoteOn { key: note.into(), vel: vel.into() });
                        // println!("new note_pos: {}", track.tick_pos + len);
                        track.pending_notes.push((note, track.tick_pos + len));
                    },
                    Instruction::Rest(len) => {
                        // process notes that should be ending during this rest
                        // NOTE: sorting in reverse order so that pop can efficiently remove items.
                        track.pending_notes.sort_unstable_by(|(_, pos_a), (_, pos_b)| pos_b.cmp(pos_a));
                        let after_rest = track.tick_pos + len;

                        while track.pending_notes.last().map(|(_, pos)| *pos <= after_rest).unwrap_or(false) {
                            let (key, pos) = track.pending_notes.pop().unwrap();

                            // println!("note_pos: {}, tick_pos: {}", pos, track.tick_pos);
                            track.tick_pos += pos - track.tick_pos;
                            track.push_midi_event(MidiMessage::NoteOff { key: key.into(), vel: 0.into() });
                        }

                        track.tick_pos = after_rest;
                    },
                    // TODO: what about instruments higher than 127?
                    Instruction::Instrument(program) => {
                        //track.push_control_event();
                        track.push_midi_event(MidiMessage::ProgramChange { program: (program as u8).into() });
                    },
                    Instruction::Fork { track: index, dest } => {
                        match dest {
                            Destination::Label(name) => {
                                tracks.push(track.fork(index, labels[&name]));
                            },
                            _ => unimplemented!()
                        }
                    },
                    Instruction::Jump(dest) => {
                        match dest {
                            Destination::Label(name) => {
                                track.instruction_pos = labels[&name];
                                continue;
                            },
                            _ => unimplemented!()
                        }
                    },
                    Instruction::Call(dest) => {
                        match dest {
                            Destination::Label(name) => {
                                track.stack.push(track.instruction_pos);
                                track.instruction_pos = labels[&name];
                                continue;
                            },
                            _ => unimplemented!()
                        }
                    },
                    Instruction::If => if !track.flag {
                        // skip the next instructiton.
                        track.instruction_pos += 1;
                    },
                    Instruction::LoopStart(_) => (),

                    Instruction::UserProcess { op, var, imm} => {
                        let var = &mut track.variables[var as usize];
                        match op {
                            UserOp::Set => *var = imm,
                            UserOp::Add => *var += imm,
                            UserOp::Sub => *var -= imm,
                            UserOp::Mul => *var *= imm,
                            UserOp::Div => *var /= imm,
                            // TODO: Verify behavior
                            UserOp::Shift => if imm < 0 {
                                *var >>= -imm;
                            } else {
                                *var <<= imm;
                            },
                            // Chosen by dair dice roll. guaranteed to be random.
                            // TODO: Jokes aside, actually implement this.
                            UserOp::Rand => *var = 4,
                            UserOp::And => *var &= imm,
                            UserOp::Or => *var |= imm,
                            UserOp::Xor => *var ^= imm,
                            // ???? TODO: Verify behavior
                            UserOp::Not => *var = !*var,
                            UserOp::Mod => *var %= imm,
                            UserOp::CmpEq => track.flag = *var == imm,
                            UserOp::CmpGe => track.flag = *var >= imm,
                            UserOp::CmpGt => track.flag = *var < imm,
                            UserOp::CmpLe => track.flag = *var <= imm,
                            UserOp::CmpLt => track.flag = *var < imm,
                            UserOp::CmpNe => track.flag = *var != imm,
                            UserOp::User => println!("User callback(?): 0x{:x}", imm as u16)
                        }
                    },
                    Instruction::LoopEnd => (),
                    Instruction::Return => {
                        track.instruction_pos = track.stack.pop().unwrap();
                    },
                    Instruction::EndOfTrack => break,

                    Instruction::SetU8Param { param, value } => match param {
                        U8Parameters::Pan => track.push_control_event(0xA, value),
                        U8Parameters::Volume => track.push_control_event(7, value),
                        U8Parameters::Expression => track.push_control_event(0xB, value),

                        _ => println!("Unimplemented param {:?} = {}", param, value)
                    },

                    Instruction::SetU16Param { param, value } => match param {
                        U16Parameters::Tempo => track.push_event(MidiKind::Meta(MetaMessage::Tempo( (60 * 1000000 * 2 / value as u32).into() ))),
                        _ => println!("Unimplemented param {:?} = {}", param, value)
                    }

                    _ => println!("Unimplemented command: {:?}", inst),
                }
            }

            // move to next instruction
            track.instruction_pos += 1;
        }

        for (key, _) in std::mem::replace(&mut track.pending_notes, Vec::new()) {
            track.push_midi_event(MidiMessage::NoteOff { key: key.into(), vel: 0.into() });
        }

        track.push_event(MidiKind::Meta(MetaMessage::EndOfTrack));
        midi.tracks.push(track.messages);
    }

    midi.save(output.unwrap_or_else(|| input.with_extension("midi")))?;

    Ok(())
}
