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

pub fn handle_target_os_arguement(arg: String) {
    match arg.as_str() {
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
            println!("--target-os set to {}", a);
            println!("supported target operating systems are: 'linux', 'macos', 'host'");
        }
    }
}
