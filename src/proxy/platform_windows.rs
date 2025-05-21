use super::error::BoxError;
use winreg::enums::*;
use winreg::RegKey;
use std::io;
use log::{debug, info}; // Added for logging

pub fn get_from_platform_impl() -> Result<Option<String>, BoxError> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let internet_settings = match hkcu.open_subkey_with_flags(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings",
        KEY_READ,
    ) {
        Ok(key) => key,
        Err(e) => {
            // If the key itself is not found, we can consider it as proxy not configured.
            if e.kind() == io::ErrorKind::NotFound {
                log::debug!("Windows proxy settings key not found.");
                return Ok(None);
            }
            // For other errors opening the key (e.g., permissions), propagate the error.
            return Err(Box::new(e));
        }
    };

    let proxy_enable: u32 = match internet_settings.get_value("ProxyEnable") {
        Ok(val) => val,
        Err(e) => {
            // If ProxyEnable value is not found, assume proxy is disabled.
            if e.kind() == io::ErrorKind::NotFound {
                log::debug!("ProxyEnable value not found, assuming proxy is disabled.");
                0 // Default to 0 (disabled)
            } else {
                // For other errors (e.g., unexpected type, permissions), propagate the error.
                return Err(Box::new(e));
            }
        }
    };

    if proxy_enable == 0 {
        log::debug!("Windows proxy is disabled (ProxyEnable is 0).");
        return Ok(None); // Proxy is disabled
    }

    let proxy_server: String = match internet_settings.get_value("ProxyServer") {
        Ok(val) => val,
        Err(e) => {
            // If ProxyServer value is not found, assume no proxy server is configured.
            if e.kind() == io::ErrorKind::NotFound {
                log::debug!("ProxyServer value not found.");
                return Ok(None);
            }
            // For other errors, propagate the error.
            return Err(Box::new(e));
        }
    };

    if proxy_server.is_empty() {
        log::debug!("Windows proxy server string is empty.");
        return Ok(None); // Proxy server string is empty, so no proxy.
    }

    // The prompt mentions ProxyOverride but doesn't specify how to use its value.
    // For now, we're just checking if ProxyEnable is 1 and ProxyServer is set.
    // If ProxyOverride logic needs to be incorporated into whether we return Some or None,
    // that would require further specification.
    // let proxy_override: String = internet_settings.get_value("ProxyOverride").unwrap_or_default();

    log::debug!("Successfully fetched Windows proxy settings: {:?}", proxy_server);
    Ok(Some(proxy_server))
}

use crate::proxy::system::SystemProxyMap;

pub fn parse_platform_values_impl(platform_values: Option<String>) -> SystemProxyMap {
    let mut proxy_map = SystemProxyMap::new();
    if let Some(proxy_string) = platform_values {
        if !proxy_string.is_empty() {
            // Assuming the proxy_string is valid and applies to both http and https
            // A more robust solution might parse the string to ensure it's a valid URI
            // and potentially handle different formats if ProxyServer can contain them.
            // For now, a direct insertion is done.
            // Windows proxy settings typically provide one server for http/https/ftp,
            // or allow specifying per protocol, or a PAC script.
            // get_from_platform_impl currently only fetches ProxyServer, which is the simple case.
            proxy_map.insert("http".to_string(), proxy_string.clone());
            proxy_map.insert("https".to_string(), proxy_string);
        }
    }
    proxy_map
}

use crate::proxy::platform::parse_platform_values; // This function calls the _impl version
use crate::proxy::system::update_proxy_cache;
use std::{thread, time::Duration};
// get_from_platform_impl is already in this file.

pub fn background_proxy_watcher_impl() {
    thread::spawn(|| {
        loop {
            match get_from_platform_impl() {
                Ok(platform_values_option) => {
                    // parse_platform_values itself calls the _impl version.
                    // However, the public API is designed such that platform.rs calls platform_windows.rs's _impl.
                    // Here, we are already in platform_windows.rs, so we can call our own _impl directly.
                    let system_proxy_map = parse_platform_values_impl(platform_values_option);
                    info!("Windows proxy settings polled, updating cache with: {:?}", system_proxy_map);
                    update_proxy_cache(system_proxy_map);
                }
                Err(e) => {
                    // Already logged in the previous subtask, but ensure log level consistency if desired.
                    // log::error! is appropriate here.
                    log::error!("Error fetching platform proxy settings in watcher: {}", e);
                    // Decide on error strategy:
                    // 1. Do nothing (cache remains stale).
                    // 2. Clear cache: update_proxy_cache(SystemProxyMap::new());
                    // For now, doing nothing and logging.
                }
            }
            thread::sleep(Duration::from_secs(5));
        }
    });
}
