use url::Url;

use crate::proxy::system::get_proxy_cache;

use super::system::{ENV_PROXY_CACHE, SystemProxyMap};
use std::{sync::Once, thread};

#[cfg(any(target_os = "windows", target_os = "macos"))]
pub fn get_from_platform() -> Option<String> {
    #[cfg(target_os = "macos")]
    use super::platform_macos::get_from_platform_impl;
    #[cfg(target_os = "windows")]
    use super::platform_windows::get_from_platform_impl;

    get_from_platform_impl().ok().flatten()
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
pub fn parse_platform_values(platform_values: Option<String>) -> SystemProxyMap {
    #[cfg(target_os = "macos")]
    use super::platform_macos::parse_platform_values_impl;
    #[cfg(target_os = "windows")]
    use super::platform_windows::parse_platform_values_impl;
    parse_platform_values_impl(platform_values)
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
pub fn background_proxy_watcher() {
    #[cfg(target_os = "macos")]
    use super::platform_macos::background_proxy_watcher_impl;
    #[cfg(target_os = "windows")]
    use super::platform_windows::background_proxy_watcher_impl;
    background_proxy_watcher_impl()
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
pub fn extract_type_prefix(address: &str) -> Option<&str> {
    if let Some(indice) = address.find("://") {
        if indice == 0 {
            None
        } else {
            let prefix = &address[..indice];
            let contains_banned = prefix.contains(|c| c == ':' || c == '/');

            if !contains_banned { Some(prefix) } else { None }
        }
    } else {
        None
    }
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
pub fn start_background_watcher() {
    static START_UPDATER_THREAD: Once = Once::new();
    START_UPDATER_THREAD.call_once(|| {
        thread::Builder::new()
            .name("dynamic-proxy-updater".into())
            .spawn(|| background_proxy_watcher())
            .expect("failed to spawn dynamic-proxy-updater thread");
    });
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn start_background_watcher() {}

pub fn resolve_proxy_from_url(target_url: &Url) -> Option<Url> {
    resolve_from_env(target_url).or_else(|| resolve_from_system_proxy(target_url))
}

fn resolve_from_env(target_url: &Url) -> Option<Url> {
    ENV_PROXY_CACHE.get(target_url.scheme())?.into()
}

fn resolve_from_system_proxy(target_url: &Url) -> Option<Url> {
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    {
        let system_proxy_cache = get_proxy_cache();
        return system_proxy_cache.get(target_url.scheme())?.into();
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    None
}
