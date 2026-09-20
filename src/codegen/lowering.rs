use std::fmt::{self, Display};

use crate::analysis::{StaticInit, get_identifiers};

use super::*;

impl Display for AssemblyType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Longword => write!(f, "l"),
            Self::Quadword => write!(f, "q"),
        }
    }
}

impl crate::codegen::data::Program {
    pub fn format(&self) -> String {
        let inner = self
            .0
            .iter()
            .map(|f| f.format())
            .collect::<Vec<_>>()
            .join("\n\n");

        #[cfg(not(target_os = "linux"))]
        return inner;
        #[cfg(target_os = "linux")]
        return format!("    .section .note.GNU-stack,\"\",@progbits\n    .text\n{inner}\n");
    }
}

impl FunctionDefinition {
    pub fn format(&self) -> String {
        let name = self.name.0.to_string();
        // TODO: this should probably be a flag instead, so you can cross compile
        // there's some more stuff, grep for target_os
        #[cfg(target_os = "macos")]
        let name = format!("_{name}");

        let mut lines = Vec::new();
        for instruction in &self.instructions {
            instruction.format(&mut lines);
        }

        let instructions = lines.join("\n");
        let global_directive = if self.global {
            &format!(".globl {name}")
        } else {
            ""
        };

        format!(
            r#"
    .type {name}, @function
    {global_directive}
    .text
{name}:
    pushq %rbp
    movq %rsp, %rbp
{instructions}"#
        )
    }
}

impl StaticVariable {
    pub fn format(&self) -> String {
        let global_directive = if self.global {
            &format!(".globl {}", self.name.0)
        } else {
            ""
        };
        let name = &self.name;

        let size_bytes = self.init.size_bytes();
        let alignment = self.alignment;
        if self.init.is_zero() {
            format!(
                r#"
    {global_directive}
    .bss
    .balign {alignment}
{name}:
    .zero {size_bytes}
"#
            )
        } else {
            let decl = match self.init {
                StaticInit::Int(i) => format!(".long {i}"),
                StaticInit::Long(i) => format!(".quad {i}"),
            };

            format!(
                r#"
    {global_directive}
    .data
    .balign {alignment}
{name}:
    {decl}
"#
            )
        }
    }
}

impl TopLevel {
    pub fn format(&self) -> String {
        match self {
            Self::F(func) => func.format(),
            Self::V(var) => var.format(),
        }
    }
}

impl Instruction {
    pub fn format(&self, lines: &mut Vec<String>) {
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
                operator,
                src,
                dst,
                ty,
            } => {
                let op_str = operator.op_str();
                let src_size = operator.src_size().unwrap_or(ty.size_bytes());

                lines.push(format!(
                    "    {op_str}{ty} {}, {}",
                    src.format(src_size),
                    dst.format(ty.size_bytes())
                ))
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
            Self::Cmp(ty, a, b) => {
                lines.push(format!(
                    "    cmp{ty} {}, {}",
                    a.format(ty.size_bytes()),
                    b.format(ty.size_bytes())
                ));
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
                if get_identifiers().get(name).unwrap().defined {
                    #[cfg(target_os = "macos")]
                    lines.push(format!("    call _{}", name));
                    #[cfg(target_os = "linux")]
                    lines.push(format!("    call {}", name));
                } else {
                    #[cfg(target_os = "macos")]
                    lines.push(format!("    call _{}", name));
                    #[cfg(target_os = "linux")]
                    lines.push(format!("    call {}@PLT", name));
                }
            }
            Self::Push(op) => {
                lines.push(format!("    pushq {}", op.format(8)));
            }
            Self::Movsx { src, dst } => {
                lines.push(format!("    movslq {}, {}", src.format(4), dst.format(8)));
            }
        }
    }
}

impl Operand {
    pub fn format(&self, size: u32) -> String {
        match self {
            Self::Imm(i) => format!("${i}"),
            Self::Reg(register) => match (register, size) {
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

                (Register::StackPointer, _) => "%rsp",

                _ => unimplemented!(),
            }
            .to_string(),
            Self::Stack(offset) => format!("{}(%rbp)", offset),
            Self::Data(name) => format!("{}(%rip)", name.0),
            Self::Pseudo(_) => unimplemented!(),
        }
    }
}
