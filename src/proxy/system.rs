use std::{
    collections::HashMap,
    env,
    sync::{Arc, LazyLock, OnceLock},
};

use url::Url;

use super::{
    platform::{
        get_from_platform, parse_platform_values, resolve_proxy_from_url, start_background_watcher,
    },
    util::is_cgi,
};

#[derive(Debug)]
pub enum MaybeUrl {
    Url(Url),
    String(String),
}

impl From<&MaybeUrl> for Option<Url> {
    fn from(value: &MaybeUrl) -> Self {
        match value {
            MaybeUrl::Url(u) => Some(u.clone()),
            MaybeUrl::String(s) => match s.parse() {
                Ok(u) => Some(u),
                Err(e) => {
                    tracing::error!("failed to parse url {} err: {}", s, e);
                    None
                }
            },
        }
    }
}

pub(crate) static SYSTEM_PROXY_CACHE: OnceLock<arc_swap::ArcSwap<SystemProxyMap>> = OnceLock::new();
pub(crate) static ENV_PROXY_CACHE: LazyLock<SystemProxyMap> =
    LazyLock::new(|| SystemProxyMap::from_environment());

#[derive(Debug)]
pub struct SystemProxyMap(HashMap<String, MaybeUrl>);

impl SystemProxyMap {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn get(&self, k: &str) -> Option<&MaybeUrl> {
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
            let url = {
                match addr.parse::<Url>() {
                    Ok(url) => MaybeUrl::Url(url),
                    Err(_) => MaybeUrl::String(addr),
                }
            };
            self.0.insert(scheme.into(), url);
            true
        }
    }
}

pub fn get_proxy_cache() -> Arc<SystemProxyMap> {
    let cache = SYSTEM_PROXY_CACHE.get_or_init(|| {
        arc_swap::ArcSwap::from_pointee(parse_platform_values(get_from_platform()))
    });
    cache.load().clone()
}

pub fn update_proxy_cache(proxy_map: SystemProxyMap) {
    SYSTEM_PROXY_CACHE
        .get()
        .map(|p| p.store(Arc::new(proxy_map)));
}

pub fn create_auto_proxy_fn() -> impl Fn(&Url) -> Option<Url> + Send + Sync + 'static {
    start_background_watcher();
    resolve_proxy_from_url
}
