// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:30.644694
//+ Revised: 	2026/11/23 13:00:30.644694
// -----------------------------------------------------------------------------

//! Settings › About: who makes the app, which version it is, and the links it opens.

use crate::driver::{click, english, shows, shows_text};
use crate::fixtures::{open_settings, scroll_settings_to, scroll_settings_until, seeded};
use crate::screens::error_on_screen;
use avalio_lite::about::{self, Link, REPOSITORY, WEBSITE};
use avalio_lite::i18n::{AppMessage, Message, UpdateMessage};
use avalio_lite::ui::{About, AboutLink};
use slint::ComponentHandle;

// ---------------------------------------------------------------------------------------------
// Settings › About

#[test]
fn the_about_section_says_who_makes_the_app_and_which_version_it_is() {
    let ui = seeded();
    open_settings(&ui);
    let version = avalio_lite::update::target::current_version().to_string();
    let line = english(&Message::Update(UpdateMessage::Version { version: &version, target: Some("windows-x86_64") }));
    let author = "By Ran# · ran.hash@proton.me";
    for text in ["About", "avalio lite", line.as_str(), author, "Made by Breren"] {
        scroll_settings_until(&ui, text, |ui| shows_text(ui, text));
    }
    scroll_settings_to(&ui, "Breren");
    assert!(shows(&ui, "Breren"), "the Breren logo, named for a screen reader");
}

#[test]
fn every_about_link_opens_its_page() {
    let ui = seeded();
    open_settings(&ui);
    let links = [
        ("Open the Breren website", AboutLink::Website),
        ("Read the licence", AboutLink::Licence),
        ("Third-party licences", AboutLink::ThirdParty),
        ("Source code", AboutLink::Source),
        ("All versions", AboutLink::Releases),
        ("What's new", AboutLink::Changelog),
        ("Report a problem", AboutLink::Issues),
        ("Write to support", AboutLink::Support),
    ];
    for (label, link) in links {
        scroll_settings_to(&ui, label);
        click(&ui, label);
        assert_eq!(ui.dialogs.links.borrow().last(), Some(&about::ui::link(link).address()), "{label}");
    }
    assert_eq!(ui.dialogs.links.borrow().len(), Link::ALL.len(), "one page each");
    assert_eq!(ui.session.window.get_error_message(), "");
}

#[test]
fn about_links_lead_only_to_web_pages_and_mail() {
    for link in Link::ALL {
        let address = link.address();
        let page = avalio_lite::update::release::is_web_address(&address);
        assert!(page || address == "mailto:support.avalio.lite@breren.com", "{link:?}: {address}");
    }
    assert_eq!(Link::Website.address(), WEBSITE);
    assert_eq!(Link::Source.address(), REPOSITORY);
    // every button of the card names its own link
    let clicked = [
        AboutLink::Website,
        AboutLink::Licence,
        AboutLink::ThirdParty,
        AboutLink::Source,
        AboutLink::Releases,
        AboutLink::Changelog,
        AboutLink::Issues,
        AboutLink::Support,
    ];
    assert_eq!(clicked.map(about::ui::link), Link::ALL);
    for file in ["LICENSE", "CHANGELOG.md", "THIRD-PARTY-LICENSES.md"] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(file);
        assert!(path.is_file(), "the About section links to {file}, which must be in the repository");
    }
}

#[test]
fn a_link_windows_wont_open_says_where_it_leads() {
    let ui = seeded();
    *ui.dialogs.link_failure.borrow_mut() = Some("no default browser".to_owned());
    ui.session.window.global::<About>().invoke_open(AboutLink::Source);
    assert_eq!(error_on_screen(&ui), english(&Message::App(AppMessage::LinkFailed { address: REPOSITORY })));
}
