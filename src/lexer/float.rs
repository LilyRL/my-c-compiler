use crate::{
    core::ConstantType,
    diagnostics::{Diagnostic, Stage},
    lexer::Lexer,
};

// just use dec2flt :(
mod fast {
    use super::*;
    use crate::{
        core::Constant,
        lexer::{SpannedToken, Token},
    };

    impl Lexer<'_> {
        pub fn lex_float(&mut self, start: usize, ty: ConstantType) -> Option<()> {
            // we keep a hold of the current pos so we can go back afterwards, lest we try to parse the suffix twice
            let end = self.pos;
            self.pos = start;

            // can be nothing
            self.consume_while(|c| c.is_ascii_digit());

            // can be nothing
            if self.consume(b'.') {
                self.consume_while(|c| c.is_ascii_digit());
            }

            if matches!(self.peek(), Some(b'e' | b'E')) {
                self.pos += 1;

                if matches!(self.peek(), Some(b'+' | b'-')) {
                    self.pos += 1;
                }

                self.consume_while(|c| c.is_ascii_digit());
            }

            let span = start..self.pos;
            self.pos = end;

            let text = &self.source[span.clone()];

            let constant = match ty {
                ConstantType::Double => text.parse::<f64>().ok().map(Constant::Double),
                ConstantType::Float => text.parse::<f32>().ok().map(Constant::Float),
                _ => unreachable!(),
            };

            let Some(constant) = constant else {
                let message = format!("invalid float literal '{text}'");
                self.diagnostics
                    .push(Diagnostic::new(Stage::Lex, span, message));
                return None;
            };

            self.push(SpannedToken {
                token: Token::Literal(constant),
                span,
            });

            Some(())
        }

        fn consume(&mut self, c: u8) -> bool {
            if self.peek() == Some(c) {
                self.pos += 1;
                true
            } else {
                false
            }
        }

        fn consume_while(&mut self, cond: impl Fn(u8) -> bool) -> usize {
            let mut n = 0;
            while self.peek().is_some_and(&cond) {
                n += 1;
                self.pos += 1;
            }
            n
        }
    }
}

// maybe later
#[cfg(false)]
mod fun {
    use std::cmp::Ordering;

    use crate::{
        core::Constant,
        lexer::{SpannedToken, Token},
    };

    use super::*;

    pub struct BigInt {
        digits: Vec<u32>,
        something_happened: bool,
    }

    impl Lexer<'_> {
        // in the case of floats, we discard the digits string, because we will reconsume the more complicated
        // form of a float literal (i.e. could look like 5.5E+3)
        //
        // the type candidates list is also guarenteed to be exactly one item long
        pub fn lex_float(&mut self, start: usize, ty: ConstantType) -> Option<()> {
            // need to reset to this afterwards to skip over the suffix
            let end = self.pos;
            // reset because we will be consuming again from scratch
            self.pos = start;

            let span = start..end;

            let (int, fract, exp) = self.get_components()?;
            let (int, exp) = Self::simplify_components(int, fract, exp);
            let (m, q) = Self::significand(&int, exp);
            let c = Self::parse_float(m, q, ty);

            self.tokens.push(SpannedToken {
                token: Token::Literal(c),
                span,
            });

            self.pos = end;

            Some(())
        }

        fn parse_float(m: u64, q: i32, ty: ConstantType) -> Constant {
            if m == 0 {
                match ty {
                    ConstantType::Double => Constant::Double(0.0),
                    ConstantType::Float => Constant::Float(0.0),
                    _ => unreachable!(),
                }
            } else if q > 400 {
                match ty {
                    ConstantType::Double => Constant::Double(f64::INFINITY),
                    ConstantType::Float => Constant::Float(f32::INFINITY),
                    _ => unreachable!(),
                }
            } else if q < -400 {
                match ty {
                    ConstantType::Double => Constant::Double(0.0),
                    ConstantType::Float => Constant::Float(0.0),
                    _ => unreachable!(),
                }
            } else {
            }
        }

        fn get_components(&mut self) -> Option<(Vec<u8>, Vec<u8>, i64)> {
            let mut integer_component: Vec<u8> = vec![];
            let mut fractional_component: Vec<u8> = vec![];
            let mut exponent: i64 = 0;

            match self.bytes[self.pos] {
                // [0-9]+\.
                b'.' => {
                    // we know that this will match at least one digit because there is no way to reach this bit of code otherwise
                    fractional_component = self.grab_digits();
                }
                c if c.is_ascii_digit() => {
                    integer_component = self.grab_digits();

                    // for the float literal to be valid, it has to have a period in it, with the exception of when it has an exponent in it.
                    // we could check if this literal has either a period or E in it here, but based due to the float detection logic in lex_number,
                    // the only way to reach this part of the code is for it to contain one of those two things somewhere.

                    if self.peek() == Some(b'.') {
                        self.pos += 1;
                        fractional_component = self.grab_digits();
                    }

                    if matches!(self.peek(), Some(c) if c == b'e' || c == b'E') {
                        self.pos += 1;

                        let sign = match self.peek() {
                            Some(b'+') => {
                                self.pos += 1;
                                1
                            }
                            Some(b'-') => {
                                self.pos += 1;
                                -1
                            }
                            _ => 1,
                        };

                        exponent = self.grab_i64("absurd float exponent")? * sign;
                    }
                }
                _ => unreachable!(),
            }

            Some((integer_component, fractional_component, exponent))
        }

        // took this idea from the implementation of dec2flt, used in the rust compiler
        // we simplify by moving digits from the fractional part to the integer part, and reducing
        // the exponent accordingly, so we end up with the same number.
        //
        // for example:
        //
        // 5.5      => 55e-1
        // 12.34e20 => 1234e18
        fn simplify_components(mut int: Vec<u8>, fract: Vec<u8>, mut exp: i64) -> (Vec<u8>, i64) {
            for fract_digit in fract {
                int.push(fract_digit);
                exp -= 1;
            }

            (int, exp)
        }

        #[allow(unused)]
        fn significand(digits: &[u8], exp: i64) -> (u64, i32) {
            // skip leading zeros
            let significant = match digits.iter().position(|&d| d != 0) {
                Some(i) => &digits[i..],
                None => return (0, 0), // just zero
            };

            // we can do up to 19 digits because 10^19 < 2^64

            let num_kept = significant.len().min(19);
            let mut m: u64 = 0;
            for &d in &significant[..num_kept] {
                m = m * 10 + digits[d as usize] as u64;
            }

            // we use the first dropped digit for rounding
            let dropped = significant.len() - num_kept;
            if dropped > 0 {
                match significant[num_kept].cmp(&5) {
                    Ordering::Greater => m += 1,
                    Ordering::Less => m += 0,
                    Ordering::Equal => {
                        // hardware default rounding mode rounds ties towards even
                        let is_exact_tie = significant[num_kept + 1..].iter().all(|d| *d == 0);

                        if is_exact_tie && m % 2 == 0 {
                            // the only scenario where this rounds down
                        } else {
                            // this can't overflow incase you're wondering
                            // 10^19 is much less than 2^64 ~= 1.8e19
                            m += 1;
                        }
                    }
                }
            }

            (
                m,
                // i mean theoretically
                exp.saturating_add(dropped as i64).clamp(-1000, 1000) as i32,
            )
        }

        fn consume_digits(&mut self) {
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.pos += 1;
            }
        }

        fn grab_digits(&mut self) -> Vec<u8> {
            let start = self.pos;
            self.consume_digits();

            let mut digits = Vec::with_capacity(self.pos - start);
            for i in start..self.pos {
                digits.push(self.bytes[i] - b'0');
            }

            digits
        }

        fn grab_i64(&mut self, message: impl ToString) -> Option<i64> {
            let start = self.pos;
            self.consume_digits();

            let s = &self.source[start..self.pos];
            match s.parse::<i64>() {
                Ok(i) => Some(i),
                Err(_) => {
                    self.diagnostics
                        .push(Diagnostic::new(Stage::Lex, start..self.pos, message));
                    None
                }
            }
        }
    }
}
