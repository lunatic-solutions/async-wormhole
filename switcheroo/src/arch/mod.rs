#[cfg(all(target_family = "unix", target_arch = "x86_64"))]
mod unix_x64;
#[cfg(all(target_family = "unix", target_arch = "x86_64"))]
pub use self::unix_x64::*;

#[cfg(all(target_family = "unix", target_arch = "aarch64"))]
mod unix_aarch64;
#[cfg(all(target_family = "unix", target_arch = "aarch64"))]
pub use self::unix_aarch64::*;

#[cfg(all(target_family = "windows", target_arch = "x86_64"))]
mod windows_x64;
#[cfg(all(target_family = "windows", target_arch = "x86_64"))]
pub use self::windows_x64::*;
