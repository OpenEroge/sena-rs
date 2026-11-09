use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::error::{PalError, Result};
use crate::opcodes::{ext_opcode, primary_opcode, PrimaryOpcode};
use crate::operand::Operand;
use crate::point::PointTable;
use crate::script::{ScriptImage, SCRIPT_CODE_BASE};

#[derive(Debug, Clone, Copy)]
pub struct DisassembleOptions<'a> {
    pub start: usize,
    pub end: Option<usize>,
    pub point_table: Option<&'a PointTable>,
}

impl<'a> DisassembleOptions<'a> {
    pub fn new(start: usize) -> Self {
        Self {
            start,
            end: None,
            point_table: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Argument {
    Operand(Operand),
    Point {
        id: u32,
        target_pc: Option<u32>,
    },
    PointOperand {
        operand: Operand,
        point_id: Option<u32>,
        target_pc: Option<u32>,
    },
    RawSlot {
        slot: u16,
        raw: u32,
    },
    ExtCall {
        raw: u32,
        category: u16,
        index: u16,
        name: Option<&'static str>,
    },
    DestinationSlot {
        slot: u32,
        raw: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    DataWord {
        pc: u32,
        end_pc: u32,
        word: u32,
    },
    Primary {
        pc: u32,
        end_pc: u32,
        word: u32,
        opcode: u16,
        meta: Option<PrimaryOpcode>,
        args: Vec<Argument>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowEdgeKind {
    Fallthrough,
    Jump,
    BranchFalse,
    BranchTrue,
    Call,
    Return,
    Halt,
    Wait,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowEdge {
    pub kind: FlowEdgeKind,
    pub target_pc: Option<u32>,
    pub point_id: Option<u32>,
}

#[derive(Debug, Clone, Copy)]
pub struct AnnotateOptions {
    pub show_flow: bool,
    pub show_labels: bool,
}

impl Default for AnnotateOptions {
    fn default() -> Self {
        Self {
            show_flow: true,
            show_labels: true,
        }
    }
}

pub fn disassemble_script(
    script: &ScriptImage<'_>,
    options: DisassembleOptions<'_>,
) -> Result<Vec<Instruction>> {
    let len = script.len();
    let end = options.end.unwrap_or(len);
    if options.start > end || end > len {
        return Err(PalError::InvalidRange {
            start: options.start,
            end,
            len,
        });
    }

    let mut pc = options.start;
    let mut instructions = Vec::new();

    while pc.checked_add(4).ok_or(PalError::ArithmeticOverflow)? <= end {
        let insn_pc = pc;
        let word = read_instruction_u32(script, &mut pc, end)?;

        let hi = ((word >> 16) & 0xFFFF) as u16;
        let opcode = (word & 0xFFFF) as u16;

        if hi != 1 {
            instructions.push(Instruction::DataWord {
                pc: insn_pc as u32,
                end_pc: pc as u32,
                word,
            });
            continue;
        }

        let meta = primary_opcode(opcode);
        let mut args = Vec::new();

        match opcode {
            10 => {
                let label = read_instruction_u32(script, &mut pc, end)?;
                let cond = read_instruction_u32(script, &mut pc, end)?;
                let target_pc = resolve_point(options.point_table, label)?;
                args.push(Argument::Point {
                    id: label,
                    target_pc,
                });
                args.push(Argument::Operand(Operand::decode(cond)));
            }
            20 | 29 => {
                let raw = read_instruction_u32(script, &mut pc, end)?;
                args.push(Argument::RawSlot {
                    slot: (raw & 0xFFFF) as u16,
                    raw,
                });
            }
            23 => {
                let raw = read_instruction_u32(script, &mut pc, end)?;
                let dst = read_instruction_u32(script, &mut pc, end)?;
                let category = ((raw >> 16) & 0xFFFF) as u16;
                let index = (raw & 0xFFFF) as u16;
                let ext = ext_opcode(category, index);
                args.push(Argument::ExtCall {
                    raw,
                    category,
                    index,
                    name: ext.and_then(|meta| meta.name),
                });
                args.push(Argument::DestinationSlot {
                    slot: dst,
                    raw: dst,
                });
            }
            9 => {
                let raw = read_instruction_u32(script, &mut pc, end)?;
                let operand = Operand::decode(raw);
                let point_id = immediate_point_id(operand);
                let target_pc = match point_id {
                    Some(id) => resolve_point(options.point_table, id)?,
                    None => None,
                };
                args.push(Argument::PointOperand {
                    operand,
                    point_id,
                    target_pc,
                });
            }
            11 => {
                // The gosub target is an ordinary operand: plain point ids
                // decode as immediates, while flagged words (for example
                // 0x40000001 = var[1]) select the id at run time.
                let raw = read_instruction_u32(script, &mut pc, end)?;
                let operand = Operand::decode(raw);
                match immediate_point_id(operand) {
                    Some(id) => {
                        let target_pc = resolve_point(options.point_table, id)?;
                        args.push(Argument::Point { id, target_pc });
                    }
                    None => args.push(Argument::PointOperand {
                        operand,
                        point_id: None,
                        target_pc: None,
                    }),
                }
            }
            _ => {
                let argc = meta.map(|item| item.argc).unwrap_or(0);
                for _ in 0..argc {
                    let raw = read_instruction_u32(script, &mut pc, end)?;
                    args.push(Argument::Operand(Operand::decode(raw)));
                }
            }
        }

        instructions.push(Instruction::Primary {
            pc: insn_pc as u32,
            end_pc: pc as u32,
            word,
            opcode,
            meta,
            args,
        });
    }

    Ok(instructions)
}

fn read_instruction_u32(script: &ScriptImage<'_>, pc: &mut usize, limit: usize) -> Result<u32> {
    let end = pc.checked_add(4).ok_or(PalError::ArithmeticOverflow)?;
    if end > limit {
        return Err(PalError::InvalidRange {
            start: *pc,
            end: limit,
            len: script.len(),
        });
    }
    let value = crate::util::read_u32_le(script.bytes(), *pc)?;
    *pc = end;
    Ok(value)
}

fn resolve_point(point_table: Option<&PointTable>, id: u32) -> Result<Option<u32>> {
    match point_table {
        Some(table) => table.resolve_target_pc(id),
        None => Ok(None),
    }
}

fn immediate_point_id(operand: Operand) -> Option<u32> {
    match operand.kind {
        crate::operand::OperandKind::Immediate | crate::operand::OperandKind::LiteralSlot => {
            Some(operand.raw)
        }
        _ => None,
    }
}

pub fn format_annotated_script(
    script: &ScriptImage<'_>,
    options: DisassembleOptions<'_>,
    annotate: AnnotateOptions,
) -> Result<String> {
    let instructions = disassemble_script(script, options)?;
    let labels = collect_labels(options.point_table, &instructions);
    let mut out = String::new();

    for instruction in &instructions {
        if annotate.show_labels {
            if let Some(names) = labels.get(&instruction.pc()) {
                for name in names {
                    out.push_str(name);
                    out.push_str(":\n");
                }
            }
        }
        out.push_str("  ");
        out.push_str(&instruction.to_string());
        if annotate.show_flow {
            let flow = format_flow_edges(&instruction.flow_edges(), &labels);
            if !flow.is_empty() {
                out.push_str(" ; flow=");
                out.push_str(&flow);
            }
        }
        out.push('\n');
    }

    Ok(out)
}

pub fn format_cfg(script: &ScriptImage<'_>, options: DisassembleOptions<'_>) -> Result<String> {
    let instructions = disassemble_script(script, options)?;
    let labels = collect_labels(options.point_table, &instructions);
    let mut out = String::new();
    out.push_str("# control-flow edges\n");

    for instruction in &instructions {
        let source = label_for_pc(instruction.pc(), &labels);
        let edges = instruction.flow_edges();
        if edges.is_empty() {
            out.push_str(&format!("{source} -> <none>\n"));
            continue;
        }
        for edge in edges {
            let target = edge
                .target_pc
                .map(|pc| label_for_pc(pc, &labels))
                .unwrap_or_else(|| "<dynamic>".to_owned());
            let point = edge
                .point_id
                .map(|id| format!(" point[{id}]"))
                .unwrap_or_default();
            out.push_str(&format!("{source} -> {target} [{:?}{point}]\n", edge.kind));
        }
    }

    Ok(out)
}

fn collect_labels(
    point_table: Option<&PointTable>,
    instructions: &[Instruction],
) -> BTreeMap<u32, Vec<String>> {
    let mut labels: BTreeMap<u32, Vec<String>> = BTreeMap::new();

    if let Some(table) = point_table {
        let entries = table.len();
        for (reverse_index, relative) in table.raw_offsets().iter().copied().enumerate() {
            let point_id = (entries - reverse_index) as u32;
            if let Some(pc) = SCRIPT_CODE_BASE.checked_add(relative) {
                labels
                    .entry(pc)
                    .or_default()
                    .push(format!("point_{point_id}"));
            }
        }
    }

    for instruction in instructions {
        for edge in instruction.flow_edges() {
            if let Some(pc) = edge.target_pc {
                labels.entry(pc).or_default().push(format!("loc_{pc:08X}"));
            }
        }
    }

    for names in labels.values_mut() {
        let mut seen = BTreeSet::new();
        names.retain(|name| seen.insert(name.clone()));
    }

    labels
}

fn format_flow_edges(edges: &[FlowEdge], labels: &BTreeMap<u32, Vec<String>>) -> String {
    edges
        .iter()
        .map(|edge| {
            let target = edge
                .target_pc
                .map(|pc| label_for_pc(pc, labels))
                .unwrap_or_else(|| "<dynamic>".to_owned());
            let point = edge
                .point_id
                .map(|id| format!(" point[{id}]"))
                .unwrap_or_default();
            format!("{:?}->{target}{point}", edge.kind)
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn label_for_pc(pc: u32, labels: &BTreeMap<u32, Vec<String>>) -> String {
    labels
        .get(&pc)
        .and_then(|names| names.first())
        .cloned()
        .unwrap_or_else(|| format!("0x{pc:08X}"))
}

impl Instruction {
    pub fn pc(&self) -> u32 {
        match self {
            Self::DataWord { pc, .. } | Self::Primary { pc, .. } => *pc,
        }
    }

    pub fn end_pc(&self) -> u32 {
        match self {
            Self::DataWord { end_pc, .. } | Self::Primary { end_pc, .. } => *end_pc,
        }
    }

    pub fn opcode(&self) -> Option<u16> {
        match self {
            Self::Primary { opcode, .. } => Some(*opcode),
            Self::DataWord { .. } => None,
        }
    }

    pub fn flow_edges(&self) -> Vec<FlowEdge> {
        let Self::Primary {
            opcode,
            args,
            end_pc,
            ..
        } = self
        else {
            return vec![FlowEdge {
                kind: FlowEdgeKind::Fallthrough,
                target_pc: Some(self.end_pc()),
                point_id: None,
            }];
        };

        match *opcode {
            9 => {
                let (point_id, target_pc) = point_target_from_arg(args.first());
                vec![FlowEdge {
                    kind: FlowEdgeKind::Jump,
                    target_pc,
                    point_id,
                }]
            }
            10 => {
                let (point_id, target_pc) = point_target_from_arg(args.first());
                vec![
                    FlowEdge {
                        kind: FlowEdgeKind::BranchFalse,
                        target_pc,
                        point_id,
                    },
                    FlowEdge {
                        kind: FlowEdgeKind::BranchTrue,
                        target_pc: Some(*end_pc),
                        point_id: None,
                    },
                ]
            }
            11 => {
                let (point_id, target_pc) = point_target_from_arg(args.first());
                vec![
                    FlowEdge {
                        kind: FlowEdgeKind::Call,
                        target_pc,
                        point_id,
                    },
                    FlowEdge {
                        kind: FlowEdgeKind::Fallthrough,
                        target_pc: Some(*end_pc),
                        point_id: None,
                    },
                ]
            }
            21 => vec![FlowEdge {
                kind: FlowEdgeKind::Halt,
                target_pc: None,
                point_id: None,
            }],
            24 => vec![FlowEdge {
                kind: FlowEdgeKind::Return,
                target_pc: None,
                point_id: None,
            }],
            252 | 253 => vec![FlowEdge {
                kind: FlowEdgeKind::Wait,
                target_pc: Some(*end_pc),
                point_id: None,
            }],
            _ => vec![FlowEdge {
                kind: FlowEdgeKind::Fallthrough,
                target_pc: Some(*end_pc),
                point_id: None,
            }],
        }
    }
}

fn point_target_from_arg(arg: Option<&Argument>) -> (Option<u32>, Option<u32>) {
    match arg {
        Some(Argument::Point { id, target_pc }) => (Some(*id), *target_pc),
        Some(Argument::PointOperand {
            point_id,
            target_pc,
            ..
        }) => (*point_id, *target_pc),
        _ => (None, None),
    }
}

impl fmt::Display for Argument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Operand(operand) => write!(f, "{operand}"),
            Self::Point { id, target_pc } => match target_pc {
                Some(target) => write!(f, "point[{id}] -> 0x{target:08X}"),
                None => write!(f, "point[{id}]"),
            },
            Self::PointOperand {
                operand,
                point_id,
                target_pc,
            } => match (point_id, target_pc) {
                (Some(id), Some(target)) => {
                    write!(f, "point_operand({operand}) point[{id}] -> 0x{target:08X}")
                }
                (Some(id), None) => write!(f, "point_operand({operand}) point[{id}]"),
                (None, _) => write!(f, "point_operand({operand})"),
            },
            Self::RawSlot { slot, raw } => write!(f, "slot[{slot}] raw=0x{raw:08X}"),
            Self::ExtCall {
                category,
                index,
                name,
                ..
            } => match name {
                Some(name) => write!(f, "ext_{category:04X}_{index:04X}.{name}"),
                None => write!(f, "ext_{category:04X}_{index:04X}"),
            },
            Self::DestinationSlot { slot, raw } => write!(f, "dst_slot[{slot}] raw=0x{raw:08X}"),
        }
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DataWord { pc, word, .. } => write!(f, "{pc:08X}: .word 0x{word:08X}"),
            Self::Primary {
                pc,
                word,
                opcode,
                meta,
                args,
                ..
            } => {
                let generated_name;
                let name = match meta {
                    Some(meta) => meta.name,
                    None => {
                        generated_name = format!("op_{opcode:04X}");
                        generated_name.as_str()
                    }
                };

                if args.is_empty() {
                    write!(f, "{pc:08X}: {name} ; word=0x{word:08X}")
                } else {
                    write!(f, "{pc:08X}: {name} ")?;
                    for (idx, arg) in args.iter().enumerate() {
                        if idx != 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{arg}")?;
                    }
                    write!(f, " ; word=0x{word:08X}")
                }
            }
        }
    }
}
