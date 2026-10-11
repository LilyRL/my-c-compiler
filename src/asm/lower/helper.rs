use super::*;

pub struct Lowering<'a> {
    pub instructions: Vec<asm::Instruction>,
    pub symbols: &'a Symbols,
    pub constants: &'a mut Constants,
}

impl<'a> Lowering<'a> {
    pub fn new(symbols: &'a Symbols, constants: &'a mut Constants) -> Self {
        Self {
            instructions: vec![],
            symbols,
            constants,
        }
    }

    pub fn into_instructions(self) -> Vec<asm::Instruction> {
        self.instructions
    }

    pub fn mov(&mut self, ty: AssemblyType, src: Operand, dst: Operand) {
        self.instructions
            .push(asm::Instruction::Mov { ty, src, dst });
    }

    pub fn mov_to_reg(&mut self, ty: AssemblyType, src: Operand, dst: Register) {
        self.mov(ty, src, Operand::Reg(dst));
    }

    pub fn movsx(&mut self, src: Operand, dst: Operand) {
        self.instructions.push(asm::Instruction::Movsx { src, dst });
    }

    pub fn movzx(&mut self, src: Operand, dst: Operand) {
        self.instructions.push(asm::Instruction::Movzx { src, dst });
    }

    pub fn unary(&mut self, ty: AssemblyType, operator: asm::UnaryOperator, operand: Operand) {
        self.instructions.push(asm::Instruction::Unary {
            ty,
            operator,
            operand,
        });
    }

    pub fn binary(
        &mut self,
        ty: AssemblyType,
        operator: asm::BinaryOperator,
        src: Operand,
        dst: Operand,
    ) {
        self.instructions.push(asm::Instruction::Binary {
            ty,
            operator,
            src,
            dst,
        });
    }

    pub fn add(&mut self, ty: AssemblyType, src: Operand, dst: Operand) {
        self.instructions.push(asm::Instruction::Binary {
            ty,
            operator: asm::BinaryOperator::Add,
            src,
            dst,
        });
    }

    pub fn sub(&mut self, ty: AssemblyType, src: Operand, dst: Operand) {
        self.instructions.push(asm::Instruction::Binary {
            ty,
            operator: asm::BinaryOperator::Sub,
            src,
            dst,
        });
    }

    pub fn mul(&mut self, ty: AssemblyType, src: Operand, dst: Operand) {
        self.instructions.push(asm::Instruction::Binary {
            ty,
            operator: asm::BinaryOperator::Mul,
            src,
            dst,
        });
    }

    pub fn and(&mut self, ty: AssemblyType, src: Operand, dst: Operand) {
        self.instructions.push(asm::Instruction::Binary {
            ty,
            operator: asm::BinaryOperator::BitwiseAnd,
            src,
            dst,
        });
    }

    pub fn or(&mut self, ty: AssemblyType, src: Operand, dst: Operand) {
        self.instructions.push(asm::Instruction::Binary {
            ty,
            operator: asm::BinaryOperator::BitwiseOr,
            src,
            dst,
        });
    }

    pub fn xor(&mut self, ty: AssemblyType, src: Operand, dst: Operand) {
        self.instructions.push(asm::Instruction::Binary {
            ty,
            operator: asm::BinaryOperator::BitwiseXor,
            src,
            dst,
        });
    }

    pub fn allocate_stack(&mut self, bytes: u32) {
        self.instructions
            .push(asm::Instruction::allocate_stack(bytes));
    }

    pub fn deallocate_stack(&mut self, bytes: u32) {
        self.instructions
            .push(asm::Instruction::deallocate_stack(bytes));
    }

    pub fn cdq(&mut self, ty: AssemblyType) {
        self.instructions.push(asm::Instruction::Cdq(ty));
    }

    pub fn idiv(&mut self, ty: AssemblyType, operand: Operand) {
        self.instructions.push(asm::Instruction::Idiv(ty, operand));
    }

    pub fn div(&mut self, ty: AssemblyType, operand: Operand) {
        self.instructions.push(asm::Instruction::Div(ty, operand));
    }

    pub fn zero_out(&mut self, ty: AssemblyType, operand: Operand) {
        self.instructions
            .push(asm::Instruction::ZeroOut(ty, operand));
    }

    pub fn cmp(&mut self, ty: AssemblyType, a: Operand, b: Operand) {
        self.instructions.push(asm::Instruction::Cmp(ty, a, b));
    }

    pub fn set_cc(&mut self, cond_code: CondCode, operand: Operand) {
        self.instructions
            .push(asm::Instruction::SetCC(cond_code, operand));
    }

    pub fn jump(&mut self, target: Identifier) {
        self.instructions.push(asm::Instruction::Jump(target));
    }

    pub fn jump_if(&mut self, cond_code: CondCode, target: Identifier) {
        self.instructions
            .push(asm::Instruction::JumpCC(cond_code, target));
    }

    pub fn label(&mut self, label: Identifier) {
        self.instructions.push(asm::Instruction::Label(label));
    }

    pub fn push(&mut self, operand: Operand) {
        self.instructions.push(asm::Instruction::Push(operand));
    }

    pub fn call(&mut self, name: Identifier) {
        self.instructions.push(asm::Instruction::Call(name));
    }

    pub fn convert(
        &mut self,
        src: Operand,
        src_type: AssemblyType,
        dst: Operand,
        dst_type: AssemblyType,
    ) {
        self.instructions.push(asm::Instruction::Convert {
            src,
            src_type,
            dst,
            dst_type,
        });
    }

    pub fn truncate_float(&mut self, ty: AssemblyType, src: Operand, dst: Operand) {
        self.instructions
            .push(asm::Instruction::TruncateFloat { ty, src, dst });
    }

    pub fn ret(&mut self) {
        self.instructions.push(asm::Instruction::Ret);
    }

    pub fn comment(&mut self, comment: &'static str) {
        self.instructions.push(asm::Instruction::Comment(comment));
    }

    pub fn zero(&mut self, ty: AssemblyType) -> Operand {
        match ty {
            AssemblyType::Longword | AssemblyType::Quadword => Operand::Imm(0),
            AssemblyType::Single | AssemblyType::Double => {
                self.constant_operand(Constant::Double(0.0), 8, "zero")
            }
        }
    }

    pub fn constant_operand(&mut self, constant: Constant, alignment: u32, name: &str) -> Operand {
        let name = self
            .constants
            .entry(constant)
            .or_insert_with(|| StaticConstant {
                init: constant.to_static_init(),
                name: Identifier::new(name),
                alignment,
            })
            .name
            .clone();

        Operand::Constant(name)
    }

    pub fn new_label(&mut self, name: &str) -> Identifier {
        Identifier::new(name)
    }
}
