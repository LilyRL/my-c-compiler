use crate::parser::Identifier;

#[derive(Debug)]
pub struct Program(pub FunctionDefinition);

#[derive(Debug)]
pub struct FunctionDefinition {
    pub name: Identifier,
    pub instructions: Vec<Instruction>,
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
    Cmp(Operand, Operand),
    Idiv(Operand),
    Cdq,
    Jump(Identifier),
    JumpCC(CondCode, Identifier),
    SetCC(CondCode, Operand),
    Label(Identifier),
    AllocateStack(u32),
    Ret,
}

#[derive(Debug, Clone, Copy)]
pub enum CondCode {
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
}

impl CondCode {
    fn format(self) -> &'static str {
        match self {
            Self::Eq => "e",
            Self::Ne => "ne",
            Self::Lt => "l",
            Self::Gt => "g",
            Self::Le => "le",
            Self::Ge => "ge",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Register {
    Ax,
    Dx,
    Cx,
    R10,
    R11,
}

#[derive(Debug, Clone)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mul,
    LeftShift,
    RightShift,
    LogicalLeftShift,
    LogicalRightShift,
    BitwiseAnd,
    BitwiseXor,
    BitwiseOr,
    Equal,
    NotEqual,
    GreaterThan,
    GreaterEqual,
    LessThan,
    LessEqual,
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOperator {
    BitwiseNot,
    Negate,
    Not,
}

#[derive(Debug, Clone)]
pub enum Operand {
    Imm(i32),
    Reg(Register),
    Pseudo(Identifier),
    Stack(i32),
}

impl BinaryOperator {
    pub fn op_str(&self) -> &'static str {
        match self {
            Self::Add => "addl",
            Self::Sub => "subl",
            Self::Mul => "imull",
            Self::LeftShift => "sall",
            Self::RightShift => "sarl",
            Self::LogicalLeftShift => "shll",
            Self::LogicalRightShift => "shrl",
            Self::BitwiseAnd => "andl",
            Self::BitwiseXor => "xorl",
            Self::BitwiseOr => "orl",
            _ => todo!(),
        }
    }

    pub fn src_size(&self) -> u32 {
        match self {
            Self::Add
            | Self::Sub
            | Self::Mul
            | Self::BitwiseAnd
            | Self::BitwiseXor
            | Self::BitwiseOr => 4,
            Self::LeftShift
            | Self::RightShift
            | Self::LogicalLeftShift
            | Self::LogicalRightShift => 1,
            _ => todo!(),
        }
    }

    pub fn dst_size(&self) -> u32 {
        4
    }

    pub fn cant_have_double_memory(&self) -> bool {
        matches!(
            self,
            Self::Add | Self::Sub | Self::BitwiseAnd | Self::BitwiseXor | Self::BitwiseOr
        )
    }

    pub fn is_shift(&self) -> bool {
        matches!(self, Self::LeftShift | Self::RightShift)
    }

    /// Returns `true` if the codegen binary operator is [`Mult`].
    ///
    /// [`Mult`]: BinaryOperator::Mult
    #[must_use]
    pub fn is_mult(&self) -> bool {
        matches!(self, Self::Mul)
    }
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

                (Register::Dx, 1) => "%dl".to_string(),
                (Register::Dx, 2) => "%dx".to_string(),
                (Register::Dx, 4) => "%edx".to_string(),
                (Register::Dx, 8) => "%rdx".to_string(),

                (Register::Cx, 1) => "%cl".to_string(),
                (Register::Cx, 2) => "%cx".to_string(),
                (Register::Cx, 4) => "%ecx".to_string(),
                (Register::Cx, 8) => "%rcx".to_string(),

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

    /// Returns `true` if the operand is [`Stack`].
    ///
    /// [`Stack`]: Operand::Stack
    #[must_use]
    pub fn is_stack(&self) -> bool {
        matches!(self, Self::Stack(..))
    }

    pub fn is_constant(&self) -> bool {
        matches!(self, Self::Imm(_))
    }
}

impl UnaryOperator {
    pub fn op_str(&self) -> &'static str {
        match self {
            Self::BitwiseNot => "notl",
            Self::Negate => "negl",
            Self::Not => unimplemented!(),
        }
    }
}
