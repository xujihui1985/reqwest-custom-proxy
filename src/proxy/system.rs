use std::{collections::HashMap, env};

use super::{platform::parse_platform_values, util::is_cgi, IntoProxyScheme, ProxyScheme};

#[derive(Debug)]
pub struct SystemProxyMap(HashMap<String, ProxyScheme>);

impl SystemProxyMap {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get(&self, k: &str) -> Option<&ProxyScheme> {
        self.0.get(k)
    }

    pub fn clear(&mut self) {
        self.0.clear()
    }

    pub fn with_proxy(&self, f: impl Fn(&SystemProxyMap)) {
        f(self)
    }

    pub fn from_environment() -> Self {
        let mut proxies = Self(HashMap::new());

        if !(proxies.insert_from_env("http", "ALL_PROXY")
            && proxies.insert_from_env("https", "ALL_PROXY"))
        {
            proxies.insert_from_env("http", "all_proxy");
            proxies.insert_from_env("https", "all_proxy");
        }

        if is_cgi() {
            if log::log_enabled!(log::Level::Warn) && env::var_os("HTTP_PROXY").is_some() {
                log::warn!("HTTP_PROXY environment variable ignored in CGI");
            }
        } else if !proxies.insert_from_env("http", "HTTP_PROXY") {
            proxies.insert_from_env("http", "http_proxy");
        }

        if !proxies.insert_from_env("https", "HTTPS_PROXY") {
            proxies.insert_from_env("https", "https_proxy");
        }
        proxies
    }

    fn insert_from_env(&mut self, scheme: &str, var: &str) -> bool {
        if let Ok(val) = env::var(var) {
            self.insert_proxy(scheme, val)
        } else {
            false
        }
    }

    pub fn insert_proxy(&mut self, scheme: impl Into<String>, addr: String) -> bool {
        if addr.trim().is_empty() {
            // do not accept empty or whitespace proxy address
            false
        } else if let Ok(valid_addr) = addr.into_proxy_scheme() {
            self.0.insert(scheme.into(), valid_addr);
            true
        } else {
            false
        }
    }
}

// fn get_sys_proxies(
//     #[cfg_attr(
//         not(any(target_os = "windows", target_os = "macos")),
//         allow(unused_variables)
//     )]
//     platform_proxies: Option<String>,
// ) -> SystemProxyMap {
//     let proxies = SystemProxyMap::from_environment();

//     #[cfg(any(target_os = "windows", target_os = "macos"))]
//     if proxies.is_empty() {
//         // if there are errors in acquiring the platform proxies,
//         // we'll just return an empty HashMap
//         if let Some(platform_proxies) = platform_proxies {
//             return parse_platform_values(platform_proxies);
//         }
//     }

//     proxies
// }
