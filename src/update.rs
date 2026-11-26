// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/07 20:25:34.678835
//+ Revised: 	2026/11/15 17:42:00.717232
// -----------------------------------------------------------------------------

//! Updating the app from its releases, to the standard every one of the author's apps follows:
//! GitHub and GitLab asked together (`source`, `check`), the BLAKE3 checksum and Ed25519 signature
//! checked before the download is fetched (`signature`, `download`), the running program replaced
//! and restarted (`install`), and an installed copy's uninstall entry kept at its version
//! (`registration`). Asset names and which build this is are in `target`. Only `ui` knows the
//! window. docs/design-decisions.md, "In-app updates", has the reasons.

pub mod check;
pub mod download;
pub mod install;
pub mod registration;
pub mod release;
pub mod signature;
pub mod source;
pub mod target;
pub mod ui;
#[cfg(windows)]
pub mod windows;
