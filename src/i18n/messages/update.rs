// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:07.486606
//+ Revised: 	2026/11/23 16:34:36.076760
// -----------------------------------------------------------------------------

//! The update topic of `Message`. Its texts are in `english/update.rs` and `galician/update.rs`.

use crate::update::download::Progress;
use crate::update::release::{CheckError, UpdateError};

/// The update texts: the Settings status lines, the banner, and the notes after an update. The
/// variants are named without their `Update` prefix, which the enum's name already gives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateMessage<'a> {
    /// Settings › Updates: the running version; `target` is the release build's, `None` a build
    /// from source
    Version {
        version: &'a str,
        target: Option<&'a str>,
    },
    /// Settings › Updates on a build from source, which has no update controls
    SourceBuild,
    /// Settings › Updates before the first check of this run
    NotChecked,
    /// the status lines; `time` is when the check ended (14:02)
    UpToDate {
        time: &'a str,
    },
    Available {
        version: &'a str,
        time: &'a str,
    },
    /// newer, but this system's file or its proof is missing from the release
    NoBuild {
        version: &'a str,
    },
    Skipped {
        version: &'a str,
        time: &'a str,
    },
    /// every source failed: each one's name and reason, in order
    CheckFailed {
        reasons: &'a [(String, CheckError)],
    },
    /// the banner: a newer version is out
    Offer {
        version: &'a str,
    },
    /// the banner while the update downloads
    Downloading {
        version: &'a str,
        progress: Option<Progress>,
    },
    /// a download or install failed; `page` is the release page to download it from by hand
    Failed {
        version: &'a str,
        error: &'a UpdateError,
        page: &'a str,
    },
    /// shown once, after the app was updated
    Updated {
        version: &'a str,
    },
    /// the new version was put in place but couldn't be started as the app closed
    NotRestarted,
}
