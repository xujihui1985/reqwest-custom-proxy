use super::util::extract_type_prefix;
use super::{error::BoxError, system::SystemProxyMap};
use std::sync::mpsc::channel;
use std::sync::{Arc, LazyLock, Once};
use std::thread;
use std::{
    result::Result as StdResult,
    sync::{OnceLock, mpsc::Sender},
};
use system_configuration::{
    core_foundation::{
        array::CFArray,
        base::CFType,
        dictionary::CFDictionary,
        number::CFNumber,
        runloop::{CFRunLoop, kCFRunLoopDefaultMode},
        string::{CFString, CFStringRef},
    },
    dynamic_store::{SCDynamicStore, SCDynamicStoreBuilder, SCDynamicStoreCallBackContext},
    sys::schema_definitions::{
        kSCPropNetProxiesHTTPEnable, kSCPropNetProxiesHTTPPort, kSCPropNetProxiesHTTPProxy,
        kSCPropNetProxiesHTTPSEnable, kSCPropNetProxiesHTTPSPort, kSCPropNetProxiesHTTPSProxy,
    },
};
use url::Url;

const PROXY_KEY: &'static str = "State:/Network/Global/Proxies";

pub(crate) fn get_from_platform_impl() -> StdResult<Option<String>, BoxError> {
    let store = SCDynamicStoreBuilder::new("reqwest").build();

    let proxies_map = if let Some(proxies_map) = store.get_proxies() {
        proxies_map
    } else {
        return Ok(None);
    };

    let http_proxy_config = parse_setting_from_dynamic_store(
        &proxies_map,
        unsafe { kSCPropNetProxiesHTTPEnable },
        unsafe { kSCPropNetProxiesHTTPProxy },
        unsafe { kSCPropNetProxiesHTTPPort },
        "http",
    );
    let https_proxy_config = parse_setting_from_dynamic_store(
        &proxies_map,
        unsafe { kSCPropNetProxiesHTTPSEnable },
        unsafe { kSCPropNetProxiesHTTPSProxy },
        unsafe { kSCPropNetProxiesHTTPSPort },
        "https",
    );

    match http_proxy_config.as_ref().zip(https_proxy_config.as_ref()) {
        Some((http_config, https_config)) => Ok(Some(format!("{http_config};{https_config}"))),
        None => Ok(http_proxy_config.or(https_proxy_config)),
    }
}

fn parse_setting_from_dynamic_store(
    proxies_map: &CFDictionary<CFString, CFType>,
    enabled_key: CFStringRef,
    host_key: CFStringRef,
    port_key: CFStringRef,
    scheme: &str,
) -> Option<String> {
    let proxy_enabled = proxies_map
        .find(enabled_key)
        .and_then(|flag| flag.downcast::<CFNumber>())
        .and_then(|flag| flag.to_i32())
        .unwrap_or(0)
        == 1;

    if proxy_enabled {
        let proxy_host = proxies_map
            .find(host_key)
            .and_then(|host| host.downcast::<CFString>())
            .map(|host| host.to_string());
        let proxy_port = proxies_map
            .find(port_key)
            .and_then(|port| port.downcast::<CFNumber>())
            .and_then(|port| port.to_i32());

        return match (proxy_host, proxy_port) {
            (Some(proxy_host), Some(proxy_port)) => {
                Some(format!("{scheme}={proxy_host}:{proxy_port}"))
            }
            (Some(proxy_host), None) => Some(format!("{scheme}={proxy_host}")),
            (None, Some(_)) => None,
            (None, None) => None,
        };
    }
    None
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
pub fn parse_platform_values_impl(platform_values: String) -> SystemProxyMap {
    let mut proxies = SystemProxyMap::new();
    if platform_values.contains("=") {
        // per-protocol settings.
        for p in platform_values.split(";") {
            let protocol_parts: Vec<&str> = p.split("=").collect();
            match protocol_parts.as_slice() {
                [protocol, address] => {
                    // If address doesn't specify an explicit protocol as protocol://address
                    // then default to HTTP
                    let address = if extract_type_prefix(*address).is_some() {
                        String::from(*address)
                    } else {
                        format!("http://{address}")
                    };

                    proxies.insert_proxy(*protocol, address);
                }
                _ => {
                    // Contains invalid protocol setting, just break the loop
                    // And make proxies to be empty.
                    proxies.clear();
                    break;
                }
            }
        }
    } else {
        if let Some(scheme) = extract_type_prefix(&platform_values) {
            // Explicit protocol has been specified
            proxies.insert_proxy(scheme, platform_values.to_owned());
        } else {
            // No explicit protocol has been specified, default to HTTP
            proxies.insert_proxy("http", format!("http://{platform_values}"));
            proxies.insert_proxy("https", format!("http://{platform_values}"));
        }
    }
    proxies
}

fn on_change(_store: SCDynamicStore, changed_keys: CFArray<CFString>, info: &mut Sender<()>) {
    for i in 0..changed_keys.len() {
        let key = changed_keys.get(i).map(|k| k.to_string());
        let Some(key) = key else {
            continue;
        };
        if key.as_str() == PROXY_KEY {
            let _ = info.send(());
            break;
        }
    }
}

static SYSTEM_PROXY_CACHE: OnceLock<arc_swap::ArcSwap<SystemProxyMap>> = OnceLock::new();

fn get_proxy_cache() -> Arc<SystemProxyMap> {
    let cache = SYSTEM_PROXY_CACHE.get_or_init(|| {
        let p = get_from_platform_impl()
            .ok()
            .flatten()
            .map(parse_platform_values_impl)
            .unwrap_or_else(|| SystemProxyMap::new());
        arc_swap::ArcSwap::from_pointee(p)
    });
    cache.load().clone()
}

fn update_proxy_cache(proxy_map: SystemProxyMap) {
    SYSTEM_PROXY_CACHE
        .get()
        .map(|p| p.store(Arc::new(proxy_map)));
}

fn background_proxy_watcher() {
    println!("start backend watcher");
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        while let Ok(_) = rx.recv() {
            let proxy = get_from_platform_impl();
            match proxy {
                Ok(Some(p)) => {
                    println!("xxxxxxxxxxxxxxx {p}");
                    let new_system_map = parse_platform_values_impl(p);
                    update_proxy_cache(new_system_map);
                }
                Ok(None) => {
                    update_proxy_cache(SystemProxyMap::new());
                }
                _ => {}
            }
        }
    });
    let context = SCDynamicStoreCallBackContext {
        callout: on_change,
        info: tx,
    };
    let store: SCDynamicStore = SCDynamicStoreBuilder::new("proxy-watcher")
        .callback_context(context)
        .build();
    let keys = CFArray::from_CFTypes(&[CFString::new(PROXY_KEY)]);
    let pattern: CFArray<CFString> = CFArray::from_CFTypes(&[]);
    store.set_notification_keys(&keys, &pattern);
    let run = store.create_run_loop_source();
    CFRunLoop::get_current().add_source(&run, unsafe { kCFRunLoopDefaultMode });
    CFRunLoop::run_current();
}

static ENV_PROXY_CACHE: LazyLock<SystemProxyMap> =
    LazyLock::new(|| SystemProxyMap::from_environment());

pub fn create_resolver_fn() -> impl Fn(&Url) -> Option<String> + Send + Sync + 'static {
    static START_UPDATER_THREAD: Once = Once::new();
    START_UPDATER_THREAD.call_once(|| {
        thread::Builder::new()
            .name("dynamic-proxy-updater".into())
            .spawn(|| background_proxy_watcher())
            .expect("failed to spawn dynamic-proxy-updater thread");
    });

    |target_url: &Url| {
        println!("check url {}", target_url);
        if let Some(match_env) = ENV_PROXY_CACHE.get(target_url.scheme()) {
            println!("match_env {:?}", match_env);
            return Some("xxxxxxx".to_string());
        }
        let system_proxy_cache = get_proxy_cache();
        println!("system_proxy_cache {:?}", system_proxy_cache);

        if let Some(match_system) = system_proxy_cache.get(target_url.scheme()) {
            println!("match_system {:?}", match_system);
            return Some("127.0.0.1:8016".to_string());
        }
        None
    }
}
