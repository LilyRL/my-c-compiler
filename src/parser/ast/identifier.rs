use crate::os::{TargetOs, target_os};
use std::{
    fmt::Display,
    sync::atomic::{AtomicUsize, Ordering},
};

static IDENT_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Hash, PartialEq, PartialOrd, Ord, Eq)]
pub struct Identifier(pub String, pub String);

impl Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn base64_encode(mut n: usize) -> String {
    const CHARS: [char; 64] = [
        'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r',
        's', 't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J',
        'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '0', '1',
        '2', '3', '4', '5', '6', '7', '8', '9', '_', '.',
    ];

    let mut s = String::new();

    while n != 0 {
        let q = n % 64;
        n /= 64;

        s.push(CHARS[q]);
    }

    s
}

impl Identifier {
    fn next() -> String {
        let n = IDENT_COUNTER.load(Ordering::Relaxed);
        IDENT_COUNTER.fetch_add(1, Ordering::Relaxed);

        base64_encode(n)
    }

    pub fn new(name: impl Display) -> Self {
        match target_os() {
            TargetOs::Linux => Self(format!(".L_{name}.{}", Self::next()), name.to_string()),
            TargetOs::MacOs => Self(format!("L_{name}.{}", Self::next()), name.to_string()),
        }
    }

    pub fn new_raw(name: &str) -> Self {
        return Self(name.to_string(), name.to_string());
    }

    pub fn local(&self) -> Self {
        match target_os() {
            TargetOs::Linux => Self(format!(".L_{}", self.0), self.1.clone()),
            TargetOs::MacOs => Self(format!("L_{}", self.0), self.1.clone()),
        }
    }

    pub fn with_suffix(&self, suffix: impl Display) -> Self {
        return Self(format!("{}{suffix}", self.0), self.1.clone());
    }

    pub fn _start(&self) -> Self {
        self.with_suffix("_start")
    }

    pub fn _break(&self) -> Self {
        self.with_suffix("_break")
    }

    pub fn _continue(&self) -> Self {
        self.with_suffix("_continue")
    }

    pub fn dummy() -> Self {
        return Self(
            "DUMMY_IDENTIFIER__SHOULD_NOT_APPEAR_IN_OUTPUT".to_string(),
            String::new(),
        );
    }
}
