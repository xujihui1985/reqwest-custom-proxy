use super::system::SystemProxyMap;

#[cfg(any(target_os = "windows", target_os = "macos"))]
pub fn get_from_platform() -> Option<String> {
    #[cfg(target_os = "macos")]
    use super::platform_macos::get_from_platform_impl;
    #[cfg(target_os = "windows")]
    use super::platform_windows::get_from_platform_impl;

    get_from_platform_impl().ok().flatten()
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
pub fn parse_platform_values(platform_values: String) -> SystemProxyMap {
    #[cfg(target_os = "macos")]
    use super::platform_macos::parse_platform_values_impl;
    #[cfg(target_os = "windows")]
    use super::platform_windows::parse_platform_values_impl;

    parse_platform_values_impl(platform_values)

}