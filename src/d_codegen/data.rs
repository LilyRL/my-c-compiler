use crate::{b_parser::UnaryOperator, parser::Identifier};

#[derive(Debug)]
pub struct Program(pub FunctionDefinition);

#[derive(Debug)]
pub struct FunctionDefinition {
    pub name: Identifier,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mult,
}

impl BinaryOperator {
    pub fn is_add_or_sub(&self) -> bool {
        matches!(self, Self::Add | Self::Sub)
    }
}

#[derive(Debug, Clone)]
pub enum Instruction {
    Mov {
        src: Operand,
        dst: Operand,
    },
    Unary {
        operator: UnaryOperator,
        operand: Operand,
    },
    Binary {
        operator: BinaryOperator,
        src: Operand,
        dst: Operand,
    },
    Idiv(Operand),
    Cdq,
    AllocateStack(u32),
    Ret,
}

#[derive(Debug, Clone)]
pub enum Register {
    Ax,
    Dx,
    R10,
    R11,
}

#[derive(Debug, Clone)]
pub enum Operand {
    Imm(i32),
    Reg(Register),
    Pseudo(Identifier),
    Stack(i32),
}

impl Program {
    pub fn format(&self) -> String {
        #[cfg(not(target_os = "linux"))]
        return self.0.format();
        #[cfg(target_os = "linux")]
        {
            let inner = self.0.format();
            format!("    .section .note.GNU-stack,\"\",@progbits\n    .text\n{inner}\n")
        }
    }
}

impl FunctionDefinition {
    pub fn format(&self) -> String {
        let name = self.name.0.to_string();
        // TODO: this should probably be a flag instead, so you can cross compile
        #[cfg(target_os = "macos")]
        let name = format!("_{name}");

        let mut lines = Vec::new();
        for instruction in &self.instructions {
            instruction.format(&mut lines);
        }

        let instructions = lines
            .iter()
            .map(|l| format!("    {l}"))
            .collect::<Vec<_>>()
            .join("\n");

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
                lines.push(format!("movl {}, {}", src.format(), dst.format()))
            }
            Self::Ret => {
                lines.push("movq %rbp, %rsp".to_string());
                lines.push("popq %rbp".to_string());
                lines.push("ret".to_string());
            }
            Self::AllocateStack(size) => lines.push(format!("subq ${size}, %rsp")),
            Self::Unary { operator, operand } => lines.push(match operator {
                UnaryOperator::Negate => format!("negl {}", operand.format()),
                UnaryOperator::Complement => format!("notl {}", operand.format()),
            }),
            Self::Binary { operator, src, dst } => {
                let op_str = match operator {
                    BinaryOperator::Add => "addl",
                    BinaryOperator::Sub => "subl",
                    BinaryOperator::Mult => "imull",
                };
                lines.push(format!("{op_str} {}, {}", src.format(), dst.format()))
            }
            Self::Idiv(operand) => {
                lines.push(format!("idivl {}", operand.format()));
            }
            Self::Cdq => {
                lines.push("cdq".to_string());
            }
        }
    }
}

impl Operand {
    pub fn format(&self) -> String {
        match self {
            Self::Imm(i) => format!("${i}"),
            Self::Reg(register) => match register {
                Register::Ax => "%eax".to_string(),
                Register::Dx => "%edx".to_string(),
                Register::R10 => "%r10d".to_string(),
                Register::R11 => "%r11d".to_string(),
            },
            Self::Stack(offset) => format!("{}(%rbp)", offset),
            Self::Pseudo(_) => unimplemented!(),
        }
    }

    /// Returns `true` if the operand is [`Stack`].
    ///
    /// [`Stack`]: Operand::Stack
    #[must_use]
    pub fn is_stack(&self) -> bool {
        matches!(self, Self::Stack(..))
    }
}
