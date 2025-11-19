use std::io;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "windows")]
pub mod windows;

pub trait TapDevice: Send + Sync {
    fn read(&self, buf: &mut [u8]) -> io::Result<usize>;
    fn write(&self, buf: &[u8]) -> io::Result<usize>;
    fn name(&self) -> &str;
}

pub fn create_tap(name: &str) -> io::Result<Box<dyn TapDevice>> {
    #[cfg(target_os = "linux")]
    return linux::create_tap_linux(name);

    #[cfg(target_os = "macos")]
    return macos::create_tap_macos(name);

    #[cfg(target_os = "windows")]
    return windows::create_tap_windows(name);
}
