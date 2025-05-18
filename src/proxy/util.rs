use std::env;

pub fn is_cgi() -> bool {
    env::var_os("REQUEST_METHOD").is_some()
}
