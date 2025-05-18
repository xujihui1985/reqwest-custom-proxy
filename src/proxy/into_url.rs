use url::Url;

use super::error;


pub trait IntoUrlSealed {
    // Besides parsing as a valid `Url`, the `Url` must be a valid
    // `http::Uri`, in that it makes sense to use in a network request.
    fn into_url(self) -> super::Result<Url>;

    fn as_str(&self) -> &str;
}

impl IntoUrlSealed for Url {
    fn into_url(self) -> super::Result<Url> {
        if self.has_host() {
            Ok(self)
        } else {
            Err(error::url_bad_scheme(self))
        }
    }

    fn as_str(&self) -> &str {
        self.as_ref()
    }
}

impl<'a> IntoUrlSealed for &'a str {
    fn into_url(self) -> super::Result<Url> {
        Url::parse(self).unwrap().into_url()
    }

    fn as_str(&self) -> &str {
        self
    }
}