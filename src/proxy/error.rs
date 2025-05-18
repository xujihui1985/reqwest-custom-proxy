use std::{error::Error as StdError, fmt};

use url::Url;


pub(crate) type BoxError = Box<dyn StdError + Send + Sync>;

#[derive(Debug)]
pub(crate) enum Kind {
    Builder,
    Request,
    Redirect,
    Body,
    Decode,
    Upgrade,
}

pub struct ProxyError {
    inner: Box<Inner>,
}

impl ProxyError {
    pub fn new<E>(kind: Kind, source: Option<E>) -> ProxyError
    where
        E: Into<BoxError>,
    {
        ProxyError {
            inner: Box::new(Inner {
                kind,
                source: source.map(Into::into),
                url: None,
            }),
        }
    }

    pub fn with_url(mut self, url: Url) -> Self {
        self.inner.url = Some(url);
        self
    }
}

impl fmt::Display for ProxyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.inner.kind {
            Kind::Builder => f.write_str("builder error")?,
            Kind::Request => f.write_str("error sending request")?,
            Kind::Body => f.write_str("request or response body error")?,
            Kind::Decode => f.write_str("error decoding response body")?,
            Kind::Redirect => f.write_str("error following redirect")?,
            Kind::Upgrade => f.write_str("error upgrading connection")?,
        };

        if let Some(url) = &self.inner.url {
            write!(f, " for url ({url})")?;
        }

        Ok(())
    }
}

impl fmt::Debug for ProxyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut builder = f.debug_struct("reqwest::Error");

        builder.field("kind", &self.inner.kind);

        if let Some(ref url) = self.inner.url {
            builder.field("url", &url.as_str());
        }
        if let Some(ref source) = self.inner.source {
            builder.field("source", source);
        }

        builder.finish()
    }
}

impl StdError for ProxyError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.inner.source.as_ref().map(|e| &**e as _)
    }
}

struct Inner {
    kind: Kind,
    source: Option<BoxError>,
    url: Option<Url>,
}

#[derive(Debug)]
pub(crate) struct BadScheme;

impl fmt::Display for BadScheme {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("URL scheme is not allowed")
    }
}

impl StdError for BadScheme {}

pub(crate) fn builder<E: Into<BoxError>>(e: E) -> ProxyError {
    ProxyError::new(Kind::Builder, Some(e))
}
pub(crate) fn url_bad_scheme(url: Url) -> ProxyError {
    ProxyError::new(Kind::Builder, Some(BadScheme)).with_url(url)
}