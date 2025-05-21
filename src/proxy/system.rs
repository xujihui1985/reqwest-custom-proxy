use std::{
    collections::HashMap,
    env,
    sync::{Arc, LazyLock, OnceLock},
};
use log::{debug, info, warn}; // Added for logging

use url::Url;

use super::{
    platform::{
        get_from_platform, parse_platform_values, resolve_proxy_from_url, start_background_watcher,
    },
    util::is_cgi,
};

pub(crate) static SYSTEM_PROXY_CACHE: OnceLock<arc_swap::ArcSwap<SystemProxyMap>> = OnceLock::new();
pub(crate) static ENV_PROXY_CACHE: LazyLock<SystemProxyMap> =
    LazyLock::new(|| SystemProxyMap::from_environment());

#[derive(Debug)]
pub struct SystemProxyMap(HashMap<String, String>);

impl SystemProxyMap {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn get(&self, k: &str) -> Option<&String> {
        self.0.get(k)
    }

    pub fn clear(&mut self) {
        self.0.clear()
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
        } else {
            self.0.insert(scheme.into(), addr);
            true
        }
    }
}

pub fn get_proxy_cache() -> Arc<SystemProxyMap> {
    let cache = SYSTEM_PROXY_CACHE.get_or_init(|| {
        let initial_proxies = parse_platform_values(get_from_platform());
        info!("Initialized system proxy cache with settings: {:?}", initial_proxies);
        arc_swap::ArcSwap::from_pointee(initial_proxies)
    });
    cache.load().clone()
}

pub fn update_proxy_cache(proxy_map: SystemProxyMap) {
    debug!("Updating SYSTEM_PROXY_CACHE with: {:?}", proxy_map);
    if let Some(cache) = SYSTEM_PROXY_CACHE.get() {
        cache.store(Arc::new(proxy_map));
    } else {
        // This case implies get_proxy_cache() was not called before update_proxy_cache().
        // This might be a logic error or an unexpected sequence of operations.
        warn!("SYSTEM_PROXY_CACHE was not initialized when update_proxy_cache was called. The update will be missed.");
        // Depending on strictness, one might panic here or attempt to initialize.
        // For now, logging a warning is a reasonable approach.
    }
}

pub fn create_auto_proxy_fn() -> impl Fn(&Url) -> Option<String> + Send + Sync + 'static {
    start_background_watcher();
    resolve_proxy_from_url
}
