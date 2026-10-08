#[cfg(windows)]
mod windows_impl;

#[cfg(windows)]
pub use windows_impl::COMMAND_CLSID;

#[cfg(not(windows))]
pub const COMMAND_CLSID: &str = "{4987C7B0-BFC5-41B0-94C8-7D06E3F1471C}";
