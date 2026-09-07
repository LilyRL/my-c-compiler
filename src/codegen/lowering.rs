use crate::analysis::get_identifiers;

use super::*;

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

        format!(
            r#"
    .type main, @function
    .globl {name}
{name}:
    pushq %rbp
    movq %rsp, %rbp
{instructions}"#
        )
    }
}

impl Instruction {
    pub fn format(&self, lines: &mut Vec<String>) {
        match self {
            Self::Mov { src, dst } => {
                lines.push(format!("    movl {}, {}", src.format(4), dst.format(4)))
            }
            Self::Ret => {
                lines.push("    movq %rbp, %rsp".to_string());
                lines.push("    popq %rbp".to_string());
                lines.push("    ret".to_string());
            }
            Self::AllocateStack(size) => lines.push(format!("    subq ${size}, %rsp")),
            Self::DeallocateStack(size) => lines.push(format!("    addq ${size}, %rsp")),
            Self::Unary { operator, operand } => {
                lines.push(format!("{} {}", operator.op_str(), operand.format(4)))
            }
            Self::Binary { operator, src, dst } => {
                let op_str = operator.op_str();
                let src_size = operator.src_size();
                let dst_size = operator.dst_size();

                lines.push(format!(
                    "    {op_str} {}, {}",
                    src.format(src_size),
                    dst.format(dst_size)
                ))
            }
            Self::Idiv(operand) => {
                lines.push(format!("    idivl {}", operand.format(4)));
            }
            Self::Cdq => {
                lines.push("    cdq".to_string());
            }
            Self::Cmp(a, b) => {
                lines.push(format!("    cmpl {}, {}", a.format(4), b.format(4)));
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
        }
    }
}

impl Operand {
    pub fn format(&self, size: u32) -> String {
        match self {
            Self::Imm(i) => format!("${i}"),
            Self::Reg(register) => match (register, size) {
                (Register::Ax, 1) => "%al".to_string(),
                (Register::Ax, 2) => "%ax".to_string(),
                (Register::Ax, 4) => "%eax".to_string(),
                (Register::Ax, 8) => "%rax".to_string(),

                (Register::Cx, 1) => "%cl".to_string(),
                (Register::Cx, 2) => "%cx".to_string(),
                (Register::Cx, 4) => "%ecx".to_string(),
                (Register::Cx, 8) => "%rcx".to_string(),

                (Register::Dx, 1) => "%dl".to_string(),
                (Register::Dx, 2) => "%dx".to_string(),
                (Register::Dx, 4) => "%edx".to_string(),
                (Register::Dx, 8) => "%rdx".to_string(),

                (Register::Di, 1) => "%dil".to_string(),
                (Register::Di, 2) => "%di".to_string(),
                (Register::Di, 4) => "%edi".to_string(),
                (Register::Di, 8) => "%rdi".to_string(),

                (Register::Si, 1) => "%sil".to_string(),
                (Register::Si, 2) => "%si".to_string(),
                (Register::Si, 4) => "%esi".to_string(),
                (Register::Si, 8) => "%rsi".to_string(),

                (Register::R8, 1) => "%r8b".to_string(),
                (Register::R8, 4) => "%r8d".to_string(),
                (Register::R8, 8) => "%r8".to_string(),

                (Register::R9, 1) => "%r9b".to_string(),
                (Register::R9, 4) => "%r9d".to_string(),
                (Register::R9, 8) => "%r9".to_string(),

                (Register::R10, 1) => "%r10b".to_string(),
                (Register::R10, 4) => "%r10d".to_string(),
                (Register::R10, 8) => "%r10".to_string(),

                (Register::R11, 1) => "%r11b".to_string(),
                (Register::R11, 4) => "%r11d".to_string(),
                (Register::R11, 8) => "%r11".to_string(),

                _ => unimplemented!(),
            },
            Self::Stack(offset) => format!("{}(%rbp)", offset),
            Self::Pseudo(_) => unimplemented!(),
        }
    }
}
