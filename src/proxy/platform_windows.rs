#[cfg(all(target_os = "windows"))]
pub(crate) fn get_from_platform_impl() -> Result<Option<String>, BoxError> {
    Ok(None)
}
