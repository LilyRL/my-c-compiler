mod global {
    use super::TargetOs;
    sge_global::global!(TargetOs, target_os);

    pub fn set(value: TargetOs) {
        set_target_os(value);
    }
}

pub fn target_os() -> TargetOs {
    *global::get_target_os()
}

use global::set as set_target_os;

// TODO: windows? idk what the difference is
#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
pub enum TargetOs {
    Linux,
    MacOs,
}

impl TargetOs {
    pub fn is_elf(self) -> bool {
        self == TargetOs::Linux
    }

    pub fn decorate_symbol(self, name: &str, global: bool) -> String {
        match (self, global) {
            (TargetOs::MacOs, true) => format!("_{name}"),
            _ => name.to_string(),
        }
    }

    pub fn global_directive(self, name: &str, global: bool) -> String {
        let name = self.decorate_symbol(name, global);
        match (global, self) {
            (true, _) => format!(".globl {name}"),
            (false, TargetOs::MacOs) => format!(".private_extern {name}"),
            (false, TargetOs::Linux) => String::new(),
        }
    }
}

pub fn handle_target_os_arguement(arg: &str) {
    match arg {
        "linux" => set_target_os(TargetOs::Linux),
        "macos" => set_target_os(TargetOs::MacOs),
        #[cfg(target_os = "linux")]
        "host" => set_target_os(TargetOs::Linux),
        #[cfg(target_os = "macos")]
        "host" => set_target_os(TargetOs::MacOs),
        #[cfg(not(target_os = "linux"))]
        #[cfg(not(target_os = "macos"))]
        "host" => {
            println!(
                "target os was set to (or defaulted to) 'host', but the host OS is not supported."
            );
            println!("supported target operating systems are: 'linux', 'macos'");
        }
        a => {
            println!("--target set to {}", a);
            println!("supported target operating systems are: 'linux', 'macos', 'host'");
        }
    }
}
