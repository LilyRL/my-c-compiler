use std::collections::HashMap;

use crate::{
    asm::{
        self, AX, AssemblyType, CondCode, DOUBLE_TO_UINT_UPPER_BOUND, DX,
        FLOAT_TO_UINT_UPPER_BOUND, Operand, R10, R11, Register, StaticConstant, UINT_UPPER_BOUND,
        XMM8, XMM9, XMM10,
    },
    core::{Constant, Identifier, Symbols, Type},
    tacky::{
        BinaryOperator, FunctionDefinition, Instruction, Program, StaticVariable, TopLevel,
        UnaryOperator, Value,
    },
    target::target,
};

use helper::Lowering;
mod helper;

pub type Constants = HashMap<Constant, StaticConstant>;

const INT_ARG_REGISTERS: [Register; 6] = {
    use Register::*;
    [Di, Si, Dx, Cx, R8, R9]
};

const FLOAT_ARG_REGISTERS: [Register; 8] = {
    use Register::*;
    [Xmm0, Xmm1, Xmm2, Xmm3, Xmm4, Xmm5, Xmm6, Xmm7]
};

impl Value {
    pub fn lower(self, out: &mut Lowering<'_>) -> asm::Operand {
        match self {
            Self::Constant(c) => match c {
                Constant::Int(n) => asm::Operand::Imm(n as i64),
                Constant::UInt(n) => asm::Operand::Imm(n as i64),
                Constant::Long(n) => asm::Operand::Imm(n as i64),
                Constant::ULong(n) => asm::Operand::Imm(n as i64),
                a @ Constant::Double(_) | a @ Constant::Float(_) => {
                    out.constant_operand(a, 8, "float_literal")
                }
            },
            Self::Var(ident) => asm::Operand::Pseudo(ident),
        }
    }
}

impl Instruction {
    pub fn lower(self, out: &mut Lowering<'_>) {
        match self {
            Self::Return(val) => {
                let ty = val.asm_type(out.symbols);

                let register = if ty.is_float() {
                    Register::Xmm0
                } else {
                    Register::Ax
                };

                let val = val.lower(out);
                out.mov_to_reg(ty, val, register);
                out.ret();
            }
            Self::Unary {
                operator: UnaryOperator::Not,
                src,
                dst,
            } => {
                let ty = src.asm_type(out.symbols);
                let src = src.lower(out);
                let dst_ty = dst.asm_type(out.symbols);
                let dst = dst.lower(out);

                let zero = out.zero(ty);

                out.cmp(ty, zero.clone(), src);
                out.mov(dst_ty, zero, dst.clone());
                out.set_cc(CondCode::Eq, dst);
            }
            Self::Unary {
                operator: UnaryOperator::Negate,
                src,
                dst,
            } if src.ty(out.symbols).is_floating_point() => {
                let ty = dst.asm_type(out.symbols);
                let src = src.lower(out);
                let dst = dst.lower(out);

                let neg_zero = {
                    let c = match ty {
                        AssemblyType::Single => Constant::Float(-0.0),
                        AssemblyType::Double => Constant::Double(-0.0),
                        _ => unreachable!(),
                    };
                    out.constant_operand(c, 16, "neg_zero")
                };

                out.mov(ty, src, dst.clone());
                out.xor(ty, neg_zero, dst);
            }
            Self::Unary { operator, src, dst } => {
                let ty = dst.asm_type(out.symbols);
                let dst = dst.lower(out);
                let src = src.lower(out);
                out.mov(ty, src, dst.clone());
                out.unary(ty, operator.lower(), dst);
            }
            Self::Binary {
                operator,
                lhs,
                rhs,
                dst,
            } => {
                let is_signed_int = lhs.ty(out.symbols).is_signed_int();
                let is_float = lhs.ty(out.symbols).is_floating_point();
                let ty = lhs.asm_type(out.symbols);
                let dst_ty = dst.asm_type(out.symbols);
                let dst = dst.lower(out);
                let lhs = lhs.lower(out);
                let rhs = rhs.lower(out);

                match operator {
                    BinaryOperator::Divide if is_float => {
                        out.mov(ty, lhs, dst.clone());
                        out.binary(ty, asm::BinaryOperator::FloatDiv, rhs, XMM10);
                        out.mov(ty, XMM10, dst);
                    }
                    BinaryOperator::Remainder if is_float => {
                        // dst = lhs - floor(lhs / rhs) * rhs

                        out.mov(ty, lhs.clone(), XMM10);
                        out.binary(ty, asm::BinaryOperator::FloatDiv, rhs.clone(), XMM10);
                        out.truncate_float(ty, XMM10, XMM10);
                        out.mul(ty, rhs, XMM10);

                        out.mov(ty, lhs, XMM8);
                        out.sub(ty, XMM10, XMM8);
                        out.mov(ty, XMM8, dst);
                    }
                    BinaryOperator::Divide | BinaryOperator::Remainder => {
                        let register = match operator {
                            BinaryOperator::Divide => Register::Ax,
                            BinaryOperator::Remainder => Register::Dx,
                            _ => unreachable!(),
                        };

                        out.mov(ty, lhs, AX);

                        if is_signed_int {
                            out.cdq(ty);
                            out.idiv(ty, rhs);
                        } else {
                            out.zero_out(ty, DX);
                            out.div(ty, rhs);
                        }

                        out.mov(ty, Operand::Reg(register), dst);
                    }
                    BinaryOperator::GreaterThan
                    | BinaryOperator::LessThan
                    | BinaryOperator::GreaterEqual
                    | BinaryOperator::LessEqual
                    | BinaryOperator::Equal
                    | BinaryOperator::NotEqual => {
                        let cond_code = match (operator, is_signed_int) {
                            (BinaryOperator::GreaterThan, true) => CondCode::Gt,
                            (BinaryOperator::GreaterThan, false) => CondCode::A,

                            (BinaryOperator::GreaterEqual, true) => CondCode::Ge,
                            (BinaryOperator::GreaterEqual, false) => CondCode::Ae,

                            (BinaryOperator::LessThan, true) => CondCode::Lt,
                            (BinaryOperator::LessThan, false) => CondCode::B,

                            (BinaryOperator::LessEqual, true) => CondCode::Le,
                            (BinaryOperator::LessEqual, false) => CondCode::Be,

                            (BinaryOperator::Equal, _) => CondCode::Eq,
                            (BinaryOperator::NotEqual, _) => CondCode::Ne,
                            _ => unreachable!(),
                        };

                        // xor clobbers flags apparently
                        out.mov(dst_ty, Operand::Imm(0), dst.clone());
                        out.cmp(ty, rhs, lhs);
                        out.set_cc(cond_code, dst);
                    }

                    _ => {
                        let asm_operator = match (operator, is_signed_int) {
                            (BinaryOperator::LeftShift, true) => asm::BinaryOperator::Sal,
                            (BinaryOperator::LeftShift, false) => asm::BinaryOperator::Shl,
                            (BinaryOperator::RightShift, true) => asm::BinaryOperator::Sar,
                            (BinaryOperator::RightShift, false) => asm::BinaryOperator::Shr,
                            _ => operator.lower(),
                        };

                        out.mov(ty, lhs, dst.clone());
                        out.binary(ty, asm_operator, rhs, dst);
                    }
                }
            }
            Self::Jump(label) => out.jump(label),
            Self::JumpIfZero { condition, target } => {
                let ty = condition.asm_type(out.symbols);
                let condition = condition.lower(out);

                if ty.is_float() {
                    out.zero_out(ty, XMM9);
                    out.cmp(ty, condition, XMM9);
                } else {
                    out.cmp(ty, Operand::Imm(0), condition);
                }

                out.jump_if(CondCode::Eq, target);
            }
            Self::JumpNotZero { condition, target } => {
                let ty = condition.asm_type(out.symbols);
                let cond = condition.lower(out);
                out.cmp(ty, Operand::Imm(0), cond);
                out.jump_if(CondCode::Ne, target);
            }
            Self::Label(label) => out.label(label),
            Self::Copy { src, dst } => {
                let ty = src.asm_type(out.symbols);
                let src = src.lower(out);
                let dst = dst.lower(out);
                out.mov(ty, src, dst)
            }
            Self::Comment(c) => out.comment(c),
            Self::FunctionCall { name, args, dst } => {
                let ty = dst.asm_type(out.symbols);

                let (int_reg_args, float_reg_args, stack_args) = classify_arguements(
                    args.into_iter()
                        .map(|arg| {
                            let ty = arg.ty(out.symbols);
                            (arg, ty)
                        })
                        .collect(),
                );

                // TODO: this assumes that all values are 32 bits, fine for now but always be on the lookout
                // comment above makes it sound like this shouldn't be working but it is. im going to leave it for now
                let stack_padding = if stack_args.len().is_multiple_of(2) {
                    0
                } else {
                    8
                };

                if stack_padding != 0 {
                    out.allocate_stack(stack_padding);
                }

                let bytes_to_remove = 8 * stack_args.len() as u32 + stack_padding;

                for (i, tacky_arg) in int_reg_args.into_iter().enumerate() {
                    let r = INT_ARG_REGISTERS[i];
                    let arg_ty = tacky_arg.asm_type(out.symbols);
                    let assembly_arg = tacky_arg.lower(out);
                    out.mov_to_reg(arg_ty, assembly_arg, r);
                }

                for (i, tacky_arg) in float_reg_args.into_iter().enumerate() {
                    let r = FLOAT_ARG_REGISTERS[i];
                    let arg_ty = tacky_arg.asm_type(out.symbols);
                    let assembly_arg = tacky_arg.lower(out);
                    out.mov_to_reg(arg_ty, assembly_arg, r);
                }

                for tacky_arg in stack_args.into_iter().rev() {
                    let asm_type = tacky_arg.asm_type(out.symbols);
                    let assembly_arg = tacky_arg.lower(out);
                    if matches!(assembly_arg, Operand::Reg(_) | Operand::Imm(_))
                        || asm_type.size_bytes() == 8
                    {
                        out.push(assembly_arg);
                    } else {
                        // stack operands can't be pushed directly
                        out.mov_to_reg(asm_type, assembly_arg, Register::Ax);
                        out.push(AX);
                    }
                }

                out.call(name);

                if bytes_to_remove != 0 {
                    out.deallocate_stack(bytes_to_remove);
                }

                let return_type = dst.ty(out.symbols);
                let dst = dst.lower(out);
                if return_type.is_floating_point() {
                    out.mov(ty, Operand::Reg(Register::Xmm0), dst);
                } else {
                    out.mov(ty, AX, dst);
                }
            }
            Self::SignExtend { src, dst } => {
                let src = src.lower(out);
                let dst = dst.lower(out);
                out.movsx(src, dst);
            }
            Self::Truncate { src, dst } => {
                let src = src.lower(out);
                let dst = dst.lower(out);
                out.mov(AssemblyType::Longword, src, dst);
            }
            Self::ZeroExtend { src, dst } => {
                let src = src.lower(out);
                let dst = dst.lower(out);
                out.movzx(src, dst);
            }
            // TODO: bounds checking
            Self::DoubleToInt { src, dst } => {
                out.lower_float_to_int(src, dst, AssemblyType::Double)
            }
            Self::FloatToInt { src, dst } => out.lower_float_to_int(src, dst, AssemblyType::Single),
            // TODO: AVX has an instruction to convert straight from double/float -> uint
            Self::DoubleToUInt { src, dst } => {
                out.lower_float_to_uint(src, dst, AssemblyType::Double)
            }
            Self::FloatToUInt { src, dst } => {
                out.lower_float_to_uint(src, dst, AssemblyType::Single)
            }
            Self::IntToFloat { src, dst } => out.lower_int_to_float(src, dst, AssemblyType::Single),
            Self::IntToDouble { src, dst } => {
                out.lower_int_to_float(src, dst, AssemblyType::Double)
            }
            Self::UIntToFloat { src, dst } => {
                out.lower_uint_to_float(src, dst, AssemblyType::Single)
            }
            Self::UIntToDouble { src, dst } => {
                out.lower_uint_to_float(src, dst, AssemblyType::Double)
            }
            Self::DoubleToFloat { src, dst } => {
                let src = src.lower(out);
                let dst = dst.lower(out);
                out.convert(src, AssemblyType::Double, dst, AssemblyType::Single)
            }
            Self::FloatToDouble { src, dst } => {
                let src = src.lower(out);
                let dst = dst.lower(out);
                out.convert(src, AssemblyType::Single, dst, AssemblyType::Double)
            }
        }
    }
}

fn classify_arguements(args: Vec<(Value, Type)>) -> (Vec<Value>, Vec<Value>, Vec<Value>) {
    let mut int_reg_args = Vec::with_capacity(args.len().min(6));
    let mut float_reg_args = Vec::with_capacity(args.len().min(8));
    let mut stack_args = Vec::with_capacity(args.len().saturating_sub(6));

    for (i, (arg, ty)) in args.into_iter().enumerate() {
        if ty.is_floating_point() {
            if float_reg_args.len() < 8 {
                float_reg_args.push(arg);
            } else {
                stack_args.push(arg);
            }
        } else {
            if int_reg_args.len() < 6 {
                int_reg_args.push(arg);
            } else {
                stack_args.push(arg);
            }
        }
    }
    (int_reg_args, float_reg_args, stack_args)
}

impl Lowering<'_> {
    fn lower_int_to_float(&mut self, src: Value, dst: Value, float_type: AssemblyType) {
        let src_type = src.asm_type(self.symbols);
        let src = src.lower(self);
        let dst = dst.lower(self);

        self.convert(src, src_type, dst, float_type);
    }

    fn lower_uint_to_float(&mut self, src: Value, dst: Value, float_type: AssemblyType) {
        let src_type = src.asm_type(self.symbols);
        let src = src.lower(self);
        let dst = dst.lower(self);

        match src_type {
            AssemblyType::Longword => {
                // extend to long and then convert it
                // cvtsi2sx reads numbers as two's complement form, so for this case we can just extend it so that the sign bit is always 0
                // movl src, %eax
                // cvtsi2sdq %rax, dst

                self.movzx(src, AX);
                self.convert(AX, AssemblyType::Quadword, dst, float_type);
            }
            AssemblyType::Quadword => {
                // check if src can be represented by a signed long,
                // if it can:   convert it normally
                // if it can't: (
                //     the number is extremely big, and the gaps between numbers this big that floats can represent is like 1000.
                //     we halve the number, rounding to odd, convert it, and then double it again.
                //     rounding to odd prevents some double rounding or something.
                // )

                let accurate_float_conversions = target().accurate_float_conversions;
                let out_of_range_label = self.new_label("float_convert_out_of_range");
                let end_label = self.new_label("float_convert_end");

                self.cmp(AssemblyType::Quadword, Operand::Imm(0), src.clone());
                self.jump_if(CondCode::Lt, out_of_range_label.clone());
                self.convert(src.clone(), src_type, dst.clone(), float_type);
                self.jump(end_label.clone());

                self.label(out_of_range_label);

                if accurate_float_conversions {
                    self.mov(AssemblyType::Quadword, src, R10);
                    self.binary(AssemblyType::Quadword, asm::BinaryOperator::Shr, R11, R11);
                    self.and(AssemblyType::Quadword, Operand::Imm(1), R10);
                    self.or(AssemblyType::Quadword, R10, R11);
                } else {
                    self.mov(AssemblyType::Quadword, src, R11);
                    self.binary(AssemblyType::Quadword, asm::BinaryOperator::Shr, R11, R11);
                }

                self.convert(R11, src_type, XMM9, float_type);
                self.add(float_type, XMM9, XMM9);
                self.mov(float_type, XMM9, dst);
            }
            _ => unreachable!(),
        }
    }

    fn lower_float_to_int(&mut self, src: Value, dst: Value, float_type: AssemblyType) {
        let dst_type = dst.asm_type(self.symbols);
        let src = src.lower(self);
        let dst = dst.lower(self);

        self.convert(src, float_type, dst, dst_type);
    }

    fn lower_float_to_uint(&mut self, src: Value, dst: Value, float_type: AssemblyType) {
        let dst_type = dst.asm_type(self.symbols);
        let src = src.lower(self);
        let dst = dst.lower(self);

        match dst_type {
            AssemblyType::Longword => {
                // convert to signed long, and truncate which removes the sign anyway
                //
                // would look something like this:
                //     cvttsd2siq  %xmm0, %rax
                //     movl  %eax, -4(%rbp)

                self.convert(src, float_type, R10, AssemblyType::Quadword);
                self.mov(AssemblyType::Longword, R10, dst);
            }
            AssemblyType::Quadword => {
                // check if it is within the bounds of a signed long,
                //     if it is: convert it directly
                //     if it isnt: subtract the upper bound, convert, then add it back after

                let upper_bound = self.constant_operand(
                    match float_type {
                        AssemblyType::Single => FLOAT_TO_UINT_UPPER_BOUND,
                        AssemblyType::Double => DOUBLE_TO_UINT_UPPER_BOUND,
                        _ => unreachable!(),
                    },
                    8,
                    "upper_bound",
                );

                let out_of_range_label = self.new_label("out_of_range");
                let end_label = self.new_label("float_to_int_end");

                // ucomisd upper_bound, src
                // jae out_of_range

                self.cmp(float_type, upper_bound.clone(), src.clone());
                self.jump_if(CondCode::Ae, out_of_range_label.clone());

                // cvtts2si src dst
                // jmp end

                self.convert(src.clone(), float_type, dst.clone(), dst_type);
                self.jump(end_label.clone());

                // out_of_range:
                //     mov src, %xmm8
                //     sub upper_bound, %xmm8
                //     cvtts2si %xmm8, dst
                //     movq $9223372036854775808, tmp
                //     add tmp, dst
                //  end:

                self.label(out_of_range_label);
                self.mov(float_type, src, XMM8);
                self.sub(float_type, upper_bound, XMM8);
                self.convert(XMM8, float_type, dst.clone(), dst_type);
                self.mov(AssemblyType::Quadword, UINT_UPPER_BOUND, DX);
                self.add(AssemblyType::Quadword, DX, dst);
                self.label(end_label);
            }
            _ => unreachable!(),
        }
    }
}

impl FunctionDefinition {
    pub fn lower(self, symbols: &Symbols, constants: &mut Constants) -> asm::FunctionDefinition {
        let FunctionDefinition {
            name,
            params,
            body,
            global,
        } = self;

        let mut out = Lowering::new(symbols, constants);

        let (int_reg_args, float_reg_args, stack_args) = classify_arguements(
            params
                .into_iter()
                .map(|p| (Value::Var(p.name), p.ty))
                .collect(),
        );

        for (i, param) in int_reg_args.into_iter().enumerate() {
            let ty = param.ty(out.symbols).to_asm_type().unwrap();
            let param = param.lower(&mut out);
            out.mov(ty, Operand::Reg(INT_ARG_REGISTERS[i]), param);
        }

        for (i, param) in float_reg_args.into_iter().enumerate() {
            let ty = param.ty(out.symbols).to_asm_type().unwrap();
            let param = param.lower(&mut out);
            out.mov(ty, Operand::Reg(FLOAT_ARG_REGISTERS[i]), param);
        }

        for (i, param) in stack_args.into_iter().enumerate() {
            let ty = param.ty(out.symbols).to_asm_type().unwrap();
            let offset = 8 * (i as u32) + 16;
            let param = param.lower(&mut out);
            out.mov(ty, Operand::Stack(offset as i32), param);
        }

        for instruction in body {
            instruction.lower(&mut out);
        }

        asm::FunctionDefinition {
            name,
            instructions: out.into_instructions(),
            global,
        }
    }
}

impl StaticVariable {
    pub fn lower(self) -> asm::StaticVariable {
        asm::StaticVariable {
            alignment: self.ty.alignment(),
            name: self.name,
            global: self.global,
            init: self.init,
        }
    }
}

impl Program {
    pub fn lower(self, symbols: &Symbols, constants: &mut Constants) -> asm::Program {
        asm::Program(
            self.0
                .into_iter()
                .map(|f| f.lower(symbols, constants))
                .collect(),
        )
    }
}

impl TopLevel {
    pub fn lower(self, symbols: &Symbols, constants: &mut Constants) -> asm::TopLevel {
        match self {
            Self::F(f) => asm::TopLevel::F(f.lower(symbols, constants)),
            Self::V(v) => asm::TopLevel::V(v.lower()),
        }
    }
}

impl UnaryOperator {
    pub fn lower(self) -> asm::UnaryOperator {
        match self {
            Self::Negate => asm::UnaryOperator::Negate,
            Self::BitwiseNot => asm::UnaryOperator::BitwiseNot,
            Self::Not => asm::UnaryOperator::Not,
        }
    }
}

impl BinaryOperator {
    pub fn lower(self) -> asm::BinaryOperator {
        match self {
            Self::Add => asm::BinaryOperator::Add,
            Self::Subtract => asm::BinaryOperator::Sub,
            Self::Multiply => asm::BinaryOperator::Mul,
            Self::BitwiseAnd => asm::BinaryOperator::BitwiseAnd,
            Self::BitwiseXor => asm::BinaryOperator::BitwiseXor,
            Self::BitwiseOr => asm::BinaryOperator::BitwiseOr,
            Self::NotEqual => asm::BinaryOperator::NotEqual,
            Self::LessThan => asm::BinaryOperator::LessThan,
            Self::LessEqual => asm::BinaryOperator::LessEqual,
            Self::GreaterThan => asm::BinaryOperator::GreaterThan,
            Self::GreaterEqual => asm::BinaryOperator::GreaterEqual,
            _ => unimplemented!(),
        }
    }
}
