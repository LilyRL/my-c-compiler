mod global {
    use super::Target;
    sge_global::global!(Target, target);

    pub fn set(value: Target) {
        set_target(value);
    }
}

use global::set as set_global;

pub fn target() -> &'static Target {
    global::get_target()
}

pub fn set_target(value: Target) {
    set_global(value);
}

// TODO: windows? idk what the difference is
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Ord, Eq)]
pub enum BinaryFormat {
    Elf,
    Mach,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordSize {
    S32 = 32,
    S64 = 64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Target {
    pub binary_format: BinaryFormat,
    pub word_size: u32,
    pub use_avx: bool,
    pub accurate_float_conversions: bool,
}

impl Target {
    pub fn is_elf(&self) -> bool {
        self.binary_format == BinaryFormat::Elf
    }

    pub fn decorate_symbol(&self, name: &str, global: bool) -> String {
        match (self.binary_format, global) {
            (BinaryFormat::Mach, true) => format!("_{name}"),
            _ => name.to_string(),
        }
    }

    pub fn global_directive(&self, name: &str, global: bool) -> String {
        let name = self.decorate_symbol(name, global);
        match (global, self.binary_format) {
            (true, _) => format!(".globl {name}"),
            (false, BinaryFormat::Mach) => format!(".private_extern {name}"),
            (false, BinaryFormat::Elf) => String::new(),
        }
    }
}

pub fn handle_binary_format_argument(arg: &str) -> BinaryFormat {
    match arg {
        "linux" => BinaryFormat::Elf,
        "macos" => BinaryFormat::Mach,
        "host" => host_binary_format(),
        a => {
            println!("--binary-format set to {a}");
            println!("supported target binary formats are: 'linux', 'macos', 'host'");
            host_binary_format()
        }
    }
}

pub fn handle_word_size_argument(arg: &str) -> u32 {
    match arg {
        "32" => WordSize::S32 as u32,
        "64" => WordSize::S64 as u32,
        "host" => host_word_size(),
        a => {
            println!("--word-size set to {a}");
            println!("supported target word sizes are: '32', '64', 'host'");
            host_word_size()
        }
    }
}

fn host_binary_format() -> BinaryFormat {
    #[cfg(target_os = "linux")]
    {
        BinaryFormat::Elf
    }
    #[cfg(target_os = "macos")]
    {
        BinaryFormat::Mach
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        println!(
            "target binary format was set to (or defaulted to) 'host', but the host OS is not supported."
        );
        println!("supported target binary formats are: 'linux', 'macos'");
        BinaryFormat::Elf
    }
}

fn host_word_size() -> u32 {
    (size_of::<usize>() * 8) as u32
}
