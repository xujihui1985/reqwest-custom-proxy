use std::error::Error;

use error::{BadScheme, ProxyError};
use http::HeaderValue;
use into_url::IntoUrlSealed;
use percent_encoding::percent_decode;
use reqwest::IntoUrl;
use url::Url;
use util::encode_basic_auth;

mod error;
mod into_url;
mod platform;
mod system;
mod util;

#[cfg(all(target_os = "macos"))]
mod platform_macos;

#[cfg(all(target_os = "windows"))]
mod platform_windows;

pub type Result<T> = std::result::Result<T, ProxyError>;

pub use platform_macos::create_resolver_fn;

pub trait IntoProxyScheme {
    fn into_proxy_scheme(self) -> Result<ProxyScheme>;
}

impl<S: IntoUrl> IntoProxyScheme for S {
    fn into_proxy_scheme(self) -> Result<ProxyScheme> {
        // validate the URL
        let url = match self.as_str().into_url() {
            Ok(ok) => ok,
            Err(e) => {
                let mut presumed_to_have_scheme = true;
                let mut source = e.source();
                while let Some(err) = source {
                    if let Some(parse_error) = err.downcast_ref::<url::ParseError>() {
                        if *parse_error == url::ParseError::RelativeUrlWithoutBase {
                            presumed_to_have_scheme = false;
                            break;
                        }
                    } else if err.downcast_ref::<BadScheme>().is_some() {
                        presumed_to_have_scheme = false;
                        break;
                    }
                    source = err.source();
                }
                if presumed_to_have_scheme {
                    return Err(error::builder(e));
                }
                // the issue could have been caused by a missing scheme, so we try adding http://
                let try_this = format!("http://{}", self.as_str());
                try_this.into_url().map_err(|_| error::builder(e))?
            }
        };
        ProxyScheme::parse(url)
    }
}

#[derive(Clone, Debug)]
pub enum ProxyScheme {
    Http {
        auth: Option<HeaderValue>,
        host: http::uri::Authority,
    },
    Https {
        auth: Option<HeaderValue>,
        host: http::uri::Authority,
    },
}

impl ProxyScheme {
    // To start conservative, keep builders private for now.

    /// Proxy traffic via the specified URL over HTTP
    fn http(host: &str) -> Result<Self> {
        Ok(ProxyScheme::Http {
            auth: None,
            host: host.parse().map_err(error::builder)?,
        })
    }

    /// Proxy traffic via the specified URL over HTTPS
    fn https(host: &str) -> Result<Self> {
        Ok(ProxyScheme::Https {
            auth: None,
            host: host.parse().map_err(error::builder)?,
        })
    }

    /// Use a username and password when connecting to the proxy server
    fn with_basic_auth<T: Into<String>, U: Into<String>>(
        mut self,
        username: T,
        password: U,
    ) -> Self {
        self.set_basic_auth(username, password);
        self
    }

    fn set_basic_auth<T: Into<String>, U: Into<String>>(&mut self, username: T, password: U) {
        match *self {
            ProxyScheme::Http { ref mut auth, .. } => {
                let header = encode_basic_auth(&username.into(), &password.into());
                *auth = Some(header);
            }
            ProxyScheme::Https { ref mut auth, .. } => {
                let header = encode_basic_auth(&username.into(), &password.into());
                *auth = Some(header);
            }
        }
    }

    fn set_custom_http_auth(&mut self, header_value: HeaderValue) {
        match *self {
            ProxyScheme::Http { ref mut auth, .. } => {
                *auth = Some(header_value);
            }
            ProxyScheme::Https { ref mut auth, .. } => {
                *auth = Some(header_value);
            }
        }
    }

    fn if_no_auth(mut self, update: &Option<HeaderValue>) -> Self {
        match self {
            ProxyScheme::Http { ref mut auth, .. } => {
                if auth.is_none() {
                    *auth = update.clone();
                }
            }
            ProxyScheme::Https { ref mut auth, .. } => {
                if auth.is_none() {
                    *auth = update.clone();
                }
            }
        }

        self
    }

    /// Convert a URL into a proxy scheme
    ///
    /// Supported schemes: HTTP, HTTPS, (SOCKS4, SOCKS5, SOCKS5H if `socks` feature is enabled).
    // Private for now...
    fn parse(url: Url) -> Result<Self> {
        use url::Position;

        // Resolve URL to a host and port
        let mut scheme = match url.scheme() {
            "http" => Self::http(&url[Position::BeforeHost..Position::AfterPort])?,
            "https" => Self::https(&url[Position::BeforeHost..Position::AfterPort])?,
            _ => panic!("xx"),
        };

        if let Some(pwd) = url.password() {
            let decoded_username = percent_decode(url.username().as_bytes()).decode_utf8_lossy();
            let decoded_password = percent_decode(pwd.as_bytes()).decode_utf8_lossy();
            scheme = scheme.with_basic_auth(decoded_username, decoded_password);
        }

        Ok(scheme)
    }
}
