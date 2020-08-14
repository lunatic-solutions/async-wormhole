mod unix_x64;
mod windows_x64;

#[cfg(all(target_family = "unix", target_arch = "x86_64"))]
pub use self::unix_x64::*;

#[cfg(all(target_family = "windows", target_arch = "x86_64"))]
pub use self::windows_x64::*;