// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/15 21:29:16.206680
//+ Revised: 	2026/11/22 21:06:18.343858
// -----------------------------------------------------------------------------

//! Settings › About: who makes the app, its licence, and where to read more. The card's words are in
//! `ui/settings/about-card.slint`; the addresses its links open are here, in one place.

pub mod ui;

use crate::licence::SUPPORT_EMAIL;

/// The app's name as people read it: the window title, message boxes, `--version`, Windows'
/// installed apps, the texts. Program, file and folder names stay `avalio-lite`.
pub const DISPLAY_NAME: &str = "avalio lite";

/// The main repository; the others (GitLab, Codeberg) mirror it.
pub const REPOSITORY: &str = "https://github.com/Ran-n/avalio-lite";
/// The publisher's site.
pub const WEBSITE: &str = "https://breren.com";

/// The links the About section offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Link {
    Website,
    Licence,
    ThirdParty,
    Source,
    Releases,
    Changelog,
    Issues,
    Support,
}

impl Link {
    pub const ALL: [Self; 8] = [
        Self::Website,
        Self::Licence,
        Self::ThirdParty,
        Self::Source,
        Self::Releases,
        Self::Changelog,
        Self::Issues,
        Self::Support,
    ];

    /// Where the link leads: always an `https://` page or a `mailto:` address, never a program.
    pub fn address(self) -> String {
        match self {
            Self::Website => WEBSITE.to_owned(),
            Self::Licence => format!("{REPOSITORY}/blob/main/LICENSE"),
            Self::ThirdParty => format!("{REPOSITORY}/blob/main/THIRD-PARTY-LICENSES.md"),
            Self::Source => REPOSITORY.to_owned(),
            Self::Releases => format!("{REPOSITORY}/releases"),
            Self::Changelog => format!("{REPOSITORY}/blob/main/CHANGELOG.md"),
            Self::Issues => format!("{REPOSITORY}/issues"),
            Self::Support => format!("mailto:{SUPPORT_EMAIL}"),
        }
    }
}
