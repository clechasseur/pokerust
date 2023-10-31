//! Helpers for unit tests in this crate.

#[cfg(unix)]
pub fn get_invalid_os_string() -> std::ffi::OsString {
    use std::os::unix::ffi::OsStrExt;

    // See documentation for OsString::to_string_lossy for details
    let source = [0x66, 0x6f, 0x80, 0x6f];
    std::ffi::OsString::from(std::ffi::OsStr::from_bytes(&source))
}

#[cfg(windows)]
pub fn get_invalid_os_string() -> std::ffi::OsString {
    use std::os::windows::ffi::OsStringExt;

    let source = [0x0066, 0x006f, 0xD800, 0x006f];
    std::ffi::OsString::from_wide(&source)
}

mod unit_tests {
    use super::*;

    mod get_invalid_os_string {
        use assert_matches::assert_matches;

        use super::*;

        #[test]
        fn test_invalid() {
            let os_string = get_invalid_os_string();
            let string = os_string.into_string();
            assert_matches!(string, Err(_));
        }
    }
}
