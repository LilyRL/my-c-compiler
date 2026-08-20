use crate::ir;
use crate::lexer::Token;
use strum::EnumIs;

#[derive(Debug)]
pub struct Program(pub FunctionDefinition);

pub type Block = Vec<BlockItem>;

#[derive(Debug)]
pub struct FunctionDefinition {
    pub name: Identifier,
    pub block: Block,
}

#[derive(Debug, Clone, Hash, PartialEq, PartialOrd, Ord, Eq)]
pub struct Identifier(pub String);

impl Identifier {
    fn inner() -> String {
        (0..4).map(|_| rand::random_range('a'..'z')).collect()
    }

    pub fn new(name: &str) -> Self {
        #[cfg(target_os = "linux")]
        return Self(format!(".L_{name}__{}", Self::inner()));
        #[cfg(target_os = "macos")]
        return Self(format!("L_{name}__{}", Self::inner()));
    }

    pub fn with_suffix(&self, suffix: &str) -> Self {
        return Self(format!("{}{suffix}", self.0));
    }
}

#[derive(Debug)]
pub enum BlockItem {
    Stmt(Statement),
    Decl(Declaration),
}

#[derive(Debug)]
pub enum Statement {
    Return(Expression),
    Expression(Expression),
    If {
        cond: Expression,
        then: Box<Statement>,
        else_: Option<Box<Statement>>,
    },
    Null,
    Goto(Identifier),
    Label(Identifier),
    Compound(Block),
}

#[derive(Debug)]
pub struct Declaration {
    pub name: Identifier,
    pub init: Option<Expression>,
}

#[derive(Debug, EnumIs)]
pub enum Expression {
    Var(Identifier),
    Constant(Constant),
    Unary {
        operator: UnaryOperator,
        expr: Box<Expression>,
    },
    Binary {
        operator: BinaryOperator,
        lhs: Box<Expression>,
        rhs: Box<Expression>,
    },
    CompoundAssign {
        operator: BinaryOperator,
        lhs: Box<Expression>,
        rhs: Box<Expression>,
    },
    Assignment(Box<Expression>, Box<Expression>),
    Prefix(IncDec, Box<Expression>),
    Postfix(IncDec, Box<Expression>),
    Conditional(Box<Expression>, Box<Expression>, Box<Expression>),
}

#[derive(Debug, EnumIs, Clone, Copy)]
pub enum IncDec {
    Increment,
    Decrement,
}

impl IncDec {
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Increment => "++",
            Self::Decrement => "--",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Increment => "inc",
            Self::Decrement => "dec",
        }
    }

    pub fn n(self) -> i32 {
        match self {
            Self::Increment => 1,
            Self::Decrement => -1,
        }
    }
}

#[derive(Debug, Clone, Copy, EnumIs)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    LeftShift,
    RightShift,
    BitwiseAnd,
    BitwiseXor,
    BitwiseOr,
    And,
    Or,
    Equal,
    NotEqual,
    LessThan,
    LessEqual,
    GreaterThan,
    GreaterEqual,
    Assign,
    AddAssign,
    SubtractAssign,
    MultiplyAssign,
    DivideAssign,
    RemainderAssign,
    BitwiseAndAssign,
    BitwiseXorAssign,
    BitwiseOrAssign,
    LeftShiftAssign,
    RightShiftAssign,
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOperator {
    BitwiseNot,
    Negate,
    Not,
}

impl UnaryOperator {
    pub fn lower(self) -> ir::UnaryOperator {
        match self {
            Self::BitwiseNot => ir::UnaryOperator::BitwiseNot,
            Self::Negate => ir::UnaryOperator::Negate,
            Self::Not => ir::UnaryOperator::Not,
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Self::BitwiseNot => "~",
            Self::Negate => "-",
            Self::Not => "!",
        }
    }
}

pub const CONDITIONAL_PRECEDENCE: u32 = 10;

impl BinaryOperator {
    pub fn precedence(self) -> u32 {
        match self {
            Self::Assign
            | Self::AddAssign
            | Self::SubtractAssign
            | Self::MultiplyAssign
            | Self::DivideAssign
            | Self::RemainderAssign
            | Self::BitwiseAndAssign
            | Self::BitwiseXorAssign
            | Self::BitwiseOrAssign
            | Self::LeftShiftAssign
            | Self::RightShiftAssign => 5,
            Self::Or => 15,
            Self::And => 20,
            Self::BitwiseOr => 25,
            Self::BitwiseXor => 30,
            Self::BitwiseAnd => 35,
            Self::Equal | Self::NotEqual => 38,
            Self::GreaterThan | Self::GreaterEqual | Self::LessThan | Self::LessEqual => 39,
            Self::LeftShift | Self::RightShift => 40,
            Self::Add | Self::Subtract => 45,
            Self::Multiply | Self::Divide | Self::Remainder => 50,
        }
    }

    pub fn can_be_lowered(self) -> bool {
        match self {
            Self::Add
            | Self::Subtract
            | Self::Multiply
            | Self::Divide
            | Self::Remainder
            | Self::LeftShift
            | Self::RightShift
            | Self::BitwiseAnd
            | Self::BitwiseXor
            | Self::BitwiseOr
            | Self::Equal
            | Self::NotEqual
            | Self::LessThan
            | Self::LessEqual
            | Self::GreaterThan
            | Self::GreaterEqual => true,
            Self::And
            | Self::Or
            | Self::Assign
            | Self::AddAssign
            | Self::SubtractAssign
            | Self::MultiplyAssign
            | Self::DivideAssign
            | Self::RemainderAssign
            | Self::BitwiseAndAssign
            | Self::BitwiseXorAssign
            | Self::BitwiseOrAssign
            | Self::LeftShiftAssign
            | Self::RightShiftAssign => false,
        }
    }

    pub fn compound_assign(self) -> Option<BinaryOperator> {
        match self {
            Self::AddAssign => Some(Self::Add),
            Self::SubtractAssign => Some(Self::Subtract),
            Self::MultiplyAssign => Some(Self::Multiply),
            Self::DivideAssign => Some(Self::Divide),
            Self::RemainderAssign => Some(Self::Remainder),
            Self::BitwiseAndAssign => Some(Self::BitwiseAnd),
            Self::BitwiseXorAssign => Some(Self::BitwiseXor),
            Self::BitwiseOrAssign => Some(Self::BitwiseOr),
            Self::LeftShiftAssign => Some(Self::LeftShift),
            Self::RightShiftAssign => Some(Self::RightShift),
            _ => None,
        }
    }

    pub fn is_compound_assign(self) -> bool {
        match self {
            Self::AddAssign
            | Self::SubtractAssign
            | Self::MultiplyAssign
            | Self::DivideAssign
            | Self::RemainderAssign
            | Self::BitwiseAndAssign
            | Self::BitwiseXorAssign
            | Self::BitwiseOrAssign
            | Self::LeftShiftAssign
            | Self::RightShiftAssign => true,
            _ => false,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Subtract => "sub",
            Self::Multiply => "mul",
            Self::Divide => "div",
            Self::Remainder => "rem",
            Self::LeftShift => "shl",
            Self::RightShift => "shr",
            Self::BitwiseAnd => "and",
            Self::BitwiseXor => "xor",
            Self::BitwiseOr => "or",
            Self::And => "and",
            Self::Or => "or",
            Self::Equal => "eq",
            Self::NotEqual => "ne",
            Self::LessThan => "lt",
            Self::LessEqual => "le",
            Self::GreaterThan => "gt",
            Self::GreaterEqual => "ge",
            Self::Assign => "assign",
            Self::AddAssign => "add_assign",
            Self::SubtractAssign => "sub_assign",
            Self::MultiplyAssign => "mul_assign",
            Self::DivideAssign => "div_assign",
            Self::RemainderAssign => "rem_assign",
            Self::BitwiseAndAssign => "and_assign",
            Self::BitwiseXorAssign => "xor_assign",
            Self::BitwiseOrAssign => "or_assign",
            Self::LeftShiftAssign => "shl_assign",
            Self::RightShiftAssign => "shr_assign",
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::Remainder => "%",
            Self::LeftShift => "<<",
            Self::RightShift => ">>",
            Self::BitwiseAnd => "&",
            Self::BitwiseXor => "^",
            Self::BitwiseOr => "|",
            Self::And => "&&",
            Self::Or => "||",
            Self::Equal => "==",
            Self::NotEqual => "!=",
            Self::LessThan => "<",
            Self::LessEqual => "<=",
            Self::GreaterThan => ">",
            Self::GreaterEqual => ">=",
            Self::Assign => "=",
            Self::AddAssign => "+=",
            Self::SubtractAssign => "-=",
            Self::MultiplyAssign => "*=",
            Self::DivideAssign => "/=",
            Self::RemainderAssign => "%=",
            Self::BitwiseAndAssign => "&=",
            Self::BitwiseXorAssign => "^=",
            Self::BitwiseOrAssign => "|=",
            Self::LeftShiftAssign => "<<=",
            Self::RightShiftAssign => ">>=",
        }
    }

    pub fn lower(self) -> ir::BinaryOperator {
        match self {
            Self::Add => ir::BinaryOperator::Add,
            Self::Subtract => ir::BinaryOperator::Subtract,
            Self::Multiply => ir::BinaryOperator::Multiply,
            Self::Divide => ir::BinaryOperator::Divide,
            Self::Remainder => ir::BinaryOperator::Remainder,
            Self::LeftShift => ir::BinaryOperator::LeftShift,
            Self::RightShift => ir::BinaryOperator::RightShift,
            Self::BitwiseAnd => ir::BinaryOperator::BitwiseAnd,
            Self::BitwiseXor => ir::BinaryOperator::BitwiseXor,
            Self::BitwiseOr => ir::BinaryOperator::BitwiseOr,
            Self::NotEqual => ir::BinaryOperator::NotEqual,
            Self::LessThan => ir::BinaryOperator::LessThan,
            Self::LessEqual => ir::BinaryOperator::LessEqual,
            Self::GreaterThan => ir::BinaryOperator::GreaterThan,
            Self::GreaterEqual => ir::BinaryOperator::GreaterEqual,
            Self::Equal => ir::BinaryOperator::Equal,
            Self::And
            | Self::Or
            | Self::Assign
            | Self::AddAssign
            | Self::SubtractAssign
            | Self::MultiplyAssign
            | Self::DivideAssign
            | Self::RemainderAssign
            | Self::BitwiseAndAssign
            | Self::BitwiseXorAssign
            | Self::BitwiseOrAssign
            | Self::LeftShiftAssign
            | Self::RightShiftAssign => unimplemented!(),
        }
    }

    pub fn from_token(token: Token) -> Option<BinaryOperator> {
        match token {
            Token::Plus => Some(BinaryOperator::Add),
            Token::Hyphen => Some(BinaryOperator::Subtract),
            Token::Asterisk => Some(BinaryOperator::Multiply),
            Token::Slash => Some(BinaryOperator::Divide),
            Token::Percent => Some(BinaryOperator::Remainder),
            Token::Ampersand => Some(BinaryOperator::BitwiseAnd),
            Token::Caret => Some(BinaryOperator::BitwiseXor),
            Token::Pipe => Some(BinaryOperator::BitwiseOr),
            Token::LeftShift => Some(BinaryOperator::LeftShift),
            Token::RightShift => Some(BinaryOperator::RightShift),
            Token::LogicalAnd => Some(BinaryOperator::And),
            Token::LogicalOr => Some(BinaryOperator::Or),
            Token::Equal => Some(BinaryOperator::Equal),
            Token::NotEqual => Some(BinaryOperator::NotEqual),
            Token::LessThan => Some(BinaryOperator::LessThan),
            Token::LessEqual => Some(BinaryOperator::LessEqual),
            Token::GreaterThan => Some(BinaryOperator::GreaterThan),
            Token::GreaterEqual => Some(BinaryOperator::GreaterEqual),
            Token::Assign => Some(BinaryOperator::Assign),
            Token::AddAssign => Some(BinaryOperator::AddAssign),
            Token::SubtractAssign => Some(BinaryOperator::SubtractAssign),
            Token::MultiplyAssign => Some(BinaryOperator::MultiplyAssign),
            Token::DivideAssign => Some(BinaryOperator::DivideAssign),
            Token::RemainderAssign => Some(BinaryOperator::RemainderAssign),
            Token::BitwiseAndAssign => Some(BinaryOperator::BitwiseAndAssign),
            Token::BitwiseXorAssign => Some(BinaryOperator::BitwiseXorAssign),
            Token::BitwiseOrAssign => Some(BinaryOperator::BitwiseOrAssign),
            Token::LeftShiftAssign => Some(BinaryOperator::LeftShiftAssign),
            Token::RightShiftAssign => Some(BinaryOperator::RightShiftAssign),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum Constant {
    Int(i32),
}
