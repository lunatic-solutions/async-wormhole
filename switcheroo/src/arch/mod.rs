mod unix_x64;

#[cfg(all(target_family = "unix", target_arch = "x86_64"))]
pub use self::unix_x64::*;