// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:04.087510
//+ Revised: 	2026/11/24 17:54:09.265378
// -----------------------------------------------------------------------------

//! Whether the test window shows on the screen: hidden unless `AVALIO_LITE_SHOW_TEST_WINDOW` is 1.
//! Runs in the default suite, without a window.

/// Only `AVALIO_LITE_SHOW_TEST_WINDOW=1` shows the window; unset or anything else keeps it hidden.
#[test]
fn only_the_variable_set_to_1_shows_the_test_window() {
    use crate::common::win32::{SHOW_WINDOW_ENV, shows_window};
    use std::ffi::OsStr;
    assert_eq!(SHOW_WINDOW_ENV, "AVALIO_LITE_SHOW_TEST_WINDOW");
    assert!(shows_window(Some(OsStr::new("1"))));
    for hidden in [None, Some(OsStr::new("")), Some(OsStr::new("0")), Some(OsStr::new("true")), Some(OsStr::new("1 "))]
    {
        assert!(!shows_window(hidden), "{hidden:?}");
    }
}
