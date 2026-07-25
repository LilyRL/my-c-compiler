use crate::parser::Identifier;

#[derive(Debug)]
pub struct Program(pub FunctionDefinition);

#[derive(Debug)]
pub struct FunctionDefinition {
    pub name: Identifier,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug)]
pub enum Instruction {
    Mov { src: Operand, dst: Operand },
    Ret,
}

#[derive(Debug)]
pub enum Operand {
    Imm(i32),
    Register,
}

impl Program {
    pub fn format(&self) -> String {
        #[cfg(not(target_os = "linux"))]
        return self.0.format();
        #[cfg(target_os = "linux")]
        {
            let inner = self.0.format();
            format!("    .section .not.GNU-stack,\"\",@progbits\n{inner}")
        }
    }
}

impl FunctionDefinition {
    pub fn format(&self) -> String {
        let name = self.name.0.to_string();
        // TODO: this should probably be a flag instead, so you can cross compile
        #[cfg(target_os = "macos")]
        let name = format!("_{name}");

        let instructions = self
            .instructions
            .iter()
            .map(|i| i.format())
            .collect::<Vec<String>>()
            .join("\n    ");
        format!("    .type main, @function\n    .globl {name}\n{name}:\n\n    {instructions}")
    }
}

impl Instruction {
    pub fn format(&self) -> String {
        match self {
            Self::Mov { src, dst } => format!("movl {}, {}", src.format(), dst.format()),
            Self::Ret => "ret".to_string(),
        }
    }
}

impl Operand {
    pub fn format(&self) -> String {
        match self {
            Self::Imm(i) => format!("${i}"),
            Self::Register => "%eax".to_string(),
        }
    }
}
