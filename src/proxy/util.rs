use std::env;

use http::HeaderValue;

pub fn encode_basic_auth(username: &str, password: &str) -> HeaderValue {
    basic_auth(username, Some(password))
}

pub fn basic_auth<U, P>(username: U, password: Option<P>) -> HeaderValue
where
    U: std::fmt::Display,
    P: std::fmt::Display,
{
    use base64::prelude::BASE64_STANDARD;
    use base64::write::EncoderWriter;
    use std::io::Write;

    let mut buf = b"Basic ".to_vec();
    {
        let mut encoder = EncoderWriter::new(&mut buf, &BASE64_STANDARD);
        let _ = write!(encoder, "{username}:");
        if let Some(password) = password {
            let _ = write!(encoder, "{password}");
        }
    }
    let mut header = HeaderValue::from_bytes(&buf).expect("base64 is always valid HeaderValue");
    header.set_sensitive(true);
    header
}

pub fn is_cgi() -> bool {
    env::var_os("REQUEST_METHOD").is_some()
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
pub fn extract_type_prefix(address: &str) -> Option<&str> {
    if let Some(indice) = address.find("://") {
        if indice == 0 {
            None
        } else {
            let prefix = &address[..indice];
            let contains_banned = prefix.contains(|c| c == ':' || c == '/');

            if !contains_banned {
                Some(prefix)
            } else {
                None
            }
        }
    } else {
        None
    }
}