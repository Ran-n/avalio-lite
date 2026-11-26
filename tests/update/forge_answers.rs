// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:33.746190
//+ Revised: 	2026/11/24 07:24:32.785712
// -----------------------------------------------------------------------------

//! Reading the GitHub and GitLab answers: fields, links and tags.

use crate::common::releases::version;
use avalio_lite::update::release::{Asset, CheckError, Release, parse_github, parse_gitlab, parse_tag};
use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------------------------
// Parsing the forges' answers

fn golden(name: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(name)).unwrap()
}

#[test]
fn a_real_github_answer_is_read() {
    let release = parse_github(&golden("github-release-latest.json")).unwrap();
    assert_eq!(release.version, version("0.10.0"));
    assert_eq!(release.page, "https://github.com/Ran-n/avalio-lite/releases/tag/v0.10.0");
    let setup = release.asset("avalio-lite-0.10.0-setup.exe.sig").unwrap();
    assert!(setup.url.starts_with("https://github.com/Ran-n/avalio-lite/releases/download/v0.10.0/"), "{}", setup.url);
    assert_eq!(release.assets.len(), 6);
}

#[test]
fn a_real_gitlab_answer_is_read() {
    let release = parse_gitlab(&golden("gitlab-release-latest.json")).unwrap();
    assert_eq!(release.version, version("0.10.0"));
    assert_eq!(release.page, "https://gitlab.com/Ran-n/avalio-lite/-/releases/v0.10.0");
    let portable = release.asset("avalio-lite-0.10.0-portable.exe").unwrap();
    assert_eq!(
        portable.url,
        "https://gitlab.com/Ran-n/avalio-lite/-/releases/v0.10.0/downloads/avalio-lite-0.10.0-portable.exe"
    );
    assert_eq!(release.assets.len(), 6);
}

#[test]
fn a_gitlab_link_without_a_direct_url_uses_its_url() {
    let json = r#"{"tag_name":"1.0.0","_links":{"self":"https://gitlab.com/x"},"assets":{"links":[{"name":"a","url":"https://gitlab.com/a"}]}}"#;
    assert_eq!(
        parse_gitlab(json).unwrap().assets,
        vec![Asset { name: "a".into(), url: "https://gitlab.com/a".into() }]
    );
}

#[test]
fn tags_are_read_with_or_without_their_v() {
    assert_eq!(parse_tag("v1.2.3").unwrap(), version("1.2.3"));
    assert_eq!(parse_tag("1.2.3-rc.1").unwrap(), version("1.2.3-rc.1"));
    for bad in ["latest", "v1.2", "vv1.2.3", "", "1.2.3 "] {
        assert!(matches!(parse_tag(bad), Err(CheckError::Unreadable(_))), "{bad:?}");
    }
}

#[test]
fn a_missing_field_is_an_error_naming_it() {
    let named = |json: &str, parse: fn(&str) -> Result<Release, CheckError>| match parse(json) {
        Err(CheckError::Unreadable(reason)) => reason,
        other => panic!("{other:?}"),
    };
    assert!(named(r#"{"html_url":"https://x","assets":[]}"#, parse_github).contains("tag_name"));
    assert!(named(r#"{"tag_name":"v1.0.0","assets":[]}"#, parse_github).contains("html_url"));
    assert!(named(r#"{"tag_name":"v1.0.0","html_url":"https://x"}"#, parse_github).contains("assets"));
    assert!(
        named(r#"{"tag_name":"v1.0.0","html_url":"https://x","assets":[{"name":"a"}]}"#, parse_github)
            .contains("browser_download_url")
    );
    assert!(named(r#"{"tag_name":"v1.0.0","assets":{"links":[]}}"#, parse_gitlab).contains("_links.self"));
    assert!(named(r#"{"tag_name":"v1.0.0","_links":{"self":"https://x"}}"#, parse_gitlab).contains("assets.links"));
    assert!(named("not json", parse_github).contains("not JSON"));
}

/// The page is handed to Windows to open, which would as readily start a program named there.
#[test]
fn regression_a_release_page_that_is_not_a_web_address_is_refused() {
    for page in [
        "C:\\Windows\\System32\\calc.exe",
        "file:///C:/Windows/System32/calc.exe",
        "http://example.org",
        "HTTPS://example.org",
        "https://",
        " https://x",
        "https://a b",
        "https://x\nstart calc",
        "",
    ] {
        let json = format!(r#"{{"tag_name":"v1.0.0","html_url":{page:?},"assets":[]}}"#);
        assert!(matches!(parse_github(&json), Err(CheckError::Unreadable(_))), "{page:?}");
        let json = format!(r#"{{"tag_name":"v1.0.0","_links":{{"self":{page:?}}},"assets":{{"links":[]}}}}"#);
        assert!(matches!(parse_gitlab(&json), Err(CheckError::Unreadable(_))), "{page:?}");
    }
}
