use std::fmt::{self, Display};

use super::*;
use crate::{
    asm::lower::Constants,
    core::{Identifier, StaticInit, Symbols},
    target::{BinaryFormat, target},
};

impl Display for AssemblyType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Longword => write!(f, "l"),
            Self::Quadword => write!(f, "q"),
            Self::Single => write!(f, "ss"),
            Self::Double => write!(f, "sd"),
        }
    }
}

impl Program {
    pub fn format(&self, symbols: &Symbols, constants: &Constants) -> String {
        let mut output = String::new();

        // so we get the same output each time
        let mut constants: Vec<_> = constants.values().collect();
        constants.sort_unstable();
        for constant in constants {
            output.push_str(&constant.format());
            output.push_str("\n");
        }

        for top_level in &self.0 {
            output.push_str(&top_level.format(symbols));
            output.push_str("\n");
        }

        match target().binary_format {
            BinaryFormat::Elf => {
                format!("    .section .note.GNU-stack,\"\",@progbits\n    .text\n{output}\n")
            }
            BinaryFormat::Mach => output,
        }
    }
}

impl FunctionDefinition {
    pub fn format(&self, symbols: &Symbols) -> String {
        let target = target();
        let name = target.decorate_symbol(&self.name.0, self.global);

        let type_directive = match target.is_elf() {
            true => format!("    .type {name}, @function\n"),
            false => String::new(),
        };

        let mut lines = Vec::new();
        for instruction in &self.instructions {
            instruction.format(symbols, &mut lines);
        }

        let instructions = lines.join("\n");
        let global_directive = target.global_directive(&self.name.0, self.global);

        format!(
            r#"
{type_directive}    {global_directive}
    .text
{name}:
    pushq %rbp
    movq %rsp, %rbp
{instructions}"#
        )
    }
}

impl StaticInit {
    fn directive(&self) -> String {
        match self {
            Self::Int(i) => format!(".long {i}"),
            Self::UInt(i) => format!(".long {i}"),
            Self::Long(i) => format!(".quad {i}"),
            Self::ULong(i) => format!(".quad {i}"),
            Self::Float(f) => format!(".long {} # {f}", f.to_bits()),
            Self::Double(d) => format!(".quad {} # {d}", d.to_bits()),
        }
    }
}

fn static_data(
    name: &Identifier,
    global: bool,
    section: &str,
    alignment_directive: &str,
    body: &str,
) -> String {
    let target = target();
    let global_directive = target.global_directive(&name.0, global);
    let name = target.decorate_symbol(&name.0, global);

    format!(
        r#"
    {global_directive}
    {section}
    {alignment_directive}
{name}:
    {body}
"#
    )
}

impl StaticConstant {
    pub fn format(&self) -> String {
        let padding = if self.alignment == 16 {
            let size_bytes = self.init.size_bytes();
            format!("\n    .zero {}", (16 - (size_bytes % 16)) % 16)
        } else {
            String::new()
        };

        let body = format!("{}{padding}", self.init.directive());
        let align = self.alignment;
        let align_directive = match target().binary_format {
            BinaryFormat::Elf => format!(".align {align}"),
            BinaryFormat::Mach => format!(".literal{align}\n    .balign {align}"),
        };

        static_data(
            &self.name,
            false,
            ".section .rodata",
            &align_directive,
            &body,
        )
    }
}

impl StaticVariable {
    pub fn format(&self) -> String {
        let in_bss = self.init.is_zero() && !self.init.is_float();

        let (section, body) = match in_bss {
            true => (".bss", format!(".zero {}", self.init.size_bytes())),
            false => (".data", self.init.directive()),
        };

        static_data(
            &self.name,
            self.global,
            section,
            &format!(".balign {}", self.alignment),
            &body,
        )
    }
}

impl TopLevel {
    pub fn format(&self, symbols: &Symbols) -> String {
        match self {
            Self::F(func) => func.format(symbols),
            Self::V(var) => var.format(),
            Self::C(c) => c.format(),
        }
    }
}

impl Instruction {
    pub fn format(&self, symbols: &Symbols, lines: &mut Vec<String>) {
        match self {
            Self::Mov { src, dst, ty } => lines.push(format!(
                "    mov{ty} {}, {}",
                src.format(ty.size_bytes()),
                dst.format(ty.size_bytes())
            )),
            Self::Ret => {
                lines.push("    movq %rbp, %rsp".to_string());
                lines.push("    popq %rbp".to_string());
                lines.push("    ret".to_string());
            }
            Self::Unary {
                operator,
                operand,
                ty,
            } => lines.push(format!(
                "    {}{ty} {}",
                operator.op_str(),
                operand.format(ty.size_bytes())
            )),
            Self::Binary {
                ty,
                operator: BinaryOperator::BitwiseXor,
                src,
                dst,
            } if ty.is_float() => {
                let suffix = match ty {
                    AssemblyType::Single => "ps",
                    AssemblyType::Double => "pd",
                    _ => unreachable!("float xor with non-float type"),
                };
                lines.push(format!(
                    "    xor{suffix} {}, {}",
                    src.format(ty.size_bytes()),
                    dst.format(ty.size_bytes())
                ))
            }
            Self::Binary {
                operator,
                src,
                dst,
                ty,
            } => {
                let op_str = operator
                    .float_op_str()
                    .filter(|_| ty.is_float())
                    .unwrap_or_else(|| operator.int_op_str());
                let src_size = operator.src_size().unwrap_or(ty.size_bytes());

                lines.push(format!(
                    "    {op_str}{ty} {}, {}",
                    src.format(src_size),
                    dst.format(ty.size_bytes())
                ))
            }
            Self::ZeroOut(ty, operand) => {
                let o = operand.format(ty.size_bytes());
                let suffix = match ty {
                    AssemblyType::Longword => "l",
                    AssemblyType::Quadword => "q",
                    AssemblyType::Single => "ps",
                    AssemblyType::Double => "pd",
                };
                lines.push(format!("    xor{suffix} {}, {}", &o, o));
            }
            Self::Div(ty, operand) => {
                lines.push(format!("    div{ty} {}", operand.format(ty.size_bytes())));
            }
            Self::Idiv(ty, operand) => {
                lines.push(format!("    idiv{ty} {}", operand.format(ty.size_bytes())));
            }
            Self::Cdq(AssemblyType::Longword) => {
                lines.push("    cdq".to_string());
            }
            Self::Cdq(AssemblyType::Quadword) => {
                lines.push("    cqo".to_string());
            }
            Self::Cdq(ty) => {
                panic!("cdq has no {ty} form");
            }
            Self::Cmp(ty, a, b) => {
                if ty.is_float() {
                    lines.push(format!(
                        "    comi{ty} {}, {}",
                        a.format(ty.size_bytes()),
                        b.format(ty.size_bytes())
                    ));
                } else {
                    lines.push(format!(
                        "    cmp{ty} {}, {}",
                        a.format(ty.size_bytes()),
                        b.format(ty.size_bytes())
                    ));
                }
            }
            Self::Jump(label) => {
                lines.push(format!("    jmp {}", label.0));
            }
            Self::JumpCC(cond_code, label) => {
                lines.push(format!("    j{} {}", cond_code.format(), label.0));
            }
            Self::SetCC(cond_code, operand) => {
                lines.push(format!(
                    "    set{} {}",
                    cond_code.format(),
                    operand.format(1)
                ));
            }
            Self::Label(label) => {
                lines.push(format!("{}:", label.0));
            }
            Self::Comment(c) => lines.push(format!("    # {}", c)),
            Self::Call(name) => {
                let attributes = &symbols
                    .get(name)
                    .expect("call target must have a symbol")
                    .attributes;
                let (is_defined, is_global) = (attributes.defined(), attributes.global());
                let symbol = target().decorate_symbol(&name.0, is_global);

                let s = match (target().binary_format, is_defined) {
                    (BinaryFormat::Elf, false) => format!("    call {symbol}@PLT"),
                    _ => format!("    call {symbol}"),
                };

                lines.push(s);
            }
            Self::Push(op) => {
                lines.push(format!("    pushq {}", op.format(8)));
            }
            Self::Movsx { src, dst } => {
                lines.push(format!("    movslq {}, {}", src.format(4), dst.format(8)));
            }
            // removed during fixups, rewrite_invalid_constant_operands
            Self::Movzx { .. } => {
                unreachable!()
            }
            Self::Convert {
                src,
                src_type,
                dst,
                dst_type,
            } => {
                if src_type == dst_type {
                    Self::Mov {
                        ty: *src_type,
                        src: src.clone(),
                        dst: dst.clone(),
                    }
                    .format(symbols, lines);
                } else {
                    if src_type.is_float() && dst_type.is_float() {
                        lines.push(format!(
                            "    cvt{}2{} {}, {}",
                            src_type,
                            dst_type,
                            src.format(src_type.size_bytes()),
                            dst.format(dst_type.size_bytes())
                        ));
                    } else if src_type.is_float() && !dst_type.is_float() {
                        lines.push(format!(
                            "    cvtt{}2si{} {}, {}",
                            src_type,
                            dst_type,
                            src.format(src_type.size_bytes()),
                            dst.format(dst_type.size_bytes())
                        ));
                    } else if !src_type.is_float() && dst_type.is_float() {
                        lines.push(format!(
                            "    cvtsi2{}{} {}, {}",
                            dst_type,
                            src_type,
                            src.format(src_type.size_bytes()),
                            dst.format(dst_type.size_bytes())
                        ));
                    } else {
                        unreachable!(
                            "invalid convert instruction: {src:?} {src_type:?} -> {dst:?} {dst_type:?}"
                        );
                    }
                }
            }
            Self::TruncateFloat { ty, src, dst } => {
                const TOWARDS_ZERO: u8 = 3;

                let src = src.format(ty.size_bytes());
                let dst = dst.format(ty.size_bytes());

                lines.push(match target().use_avx {
                    true => format!("    vrounds{ty} ${TOWARDS_ZERO}, {src}, {src}, {dst}"),
                    false => format!("    rounds{ty} ${TOWARDS_ZERO}, {src}, {dst}"),
                });
            }
        }
    }
}

impl Operand {
    pub fn format(&self, size: u32) -> String {
        match self {
            Self::Imm(i) => format!("${i}"),
            Self::Reg(register) => {
                if let Some(n) = register.xmm_number() {
                    return format!("%xmm{n}");
                }

                match (register, size) {
                    (Register::Ax, 1) => "%al",
                    (Register::Ax, 2) => "%ax",
                    (Register::Ax, 4) => "%eax",
                    (Register::Ax, 8) => "%rax",

                    (Register::Cx, 1) => "%cl",
                    (Register::Cx, 2) => "%cx",
                    (Register::Cx, 4) => "%ecx",
                    (Register::Cx, 8) => "%rcx",

                    (Register::Dx, 1) => "%dl",
                    (Register::Dx, 2) => "%dx",
                    (Register::Dx, 4) => "%edx",
                    (Register::Dx, 8) => "%rdx",

                    (Register::Di, 1) => "%dil",
                    (Register::Di, 2) => "%di",
                    (Register::Di, 4) => "%edi",
                    (Register::Di, 8) => "%rdi",

                    (Register::Si, 1) => "%sil",
                    (Register::Si, 2) => "%si",
                    (Register::Si, 4) => "%esi",
                    (Register::Si, 8) => "%rsi",

                    (Register::R8, 1) => "%r8b",
                    (Register::R8, 4) => "%r8d",
                    (Register::R8, 8) => "%r8",

                    (Register::R9, 1) => "%r9b",
                    (Register::R9, 4) => "%r9d",
                    (Register::R9, 8) => "%r9",

                    (Register::R10, 1) => "%r10b",
                    (Register::R10, 4) => "%r10d",
                    (Register::R10, 8) => "%r10",

                    (Register::R11, 1) => "%r11b",
                    (Register::R11, 4) => "%r11d",
                    (Register::R11, 8) => "%r11",

                    (Register::R12, 1) => "%r12b",
                    (Register::R12, 4) => "%r12d",
                    (Register::R12, 8) => "%r12",

                    (Register::R13, 1) => "%r13b",
                    (Register::R13, 4) => "%r13d",
                    (Register::R13, 8) => "%r13",

                    (Register::R14, 1) => "%r14b",
                    (Register::R14, 4) => "%r14d",
                    (Register::R14, 8) => "%r14",

                    (Register::R15, 1) => "%r15b",
                    (Register::R15, 4) => "%r15d",
                    (Register::R15, 8) => "%r15",

                    (Register::StackPointer, _) => "%rsp",

                    _ => unimplemented!(),
                }
                .to_string()
            }
            Self::Stack(offset) => format!("{}(%rbp)", offset),
            Self::Data(name) => format!("{name}(%rip)"),
            Self::Pseudo(_) => unimplemented!(),
        }
    }
}
