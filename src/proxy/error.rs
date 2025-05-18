use std::error::Error;

pub(crate) type BoxError = Box<dyn Error + Send + Sync>;
