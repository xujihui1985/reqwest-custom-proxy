mod error;
mod platform;
mod system;
mod util;

#[cfg(all(target_os = "macos"))]
mod platform_macos;

#[cfg(all(target_os = "windows"))]
mod platform_windows;

pub use system::create_auto_proxy_fn;
