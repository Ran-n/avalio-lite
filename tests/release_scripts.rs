// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/26 16:43:15.193171
//+ Revised: 	2026/11/26 16:43:15.193171
// -----------------------------------------------------------------------------

//! The release scripts run in Windows PowerShell 5.1, as the maintainer runs them: every script
//! parses, and the rule for freeing Codeberg's quota (`scripts/codeberg-quota.ps1`) chooses what it
//! should on sample answers.

use std::path::PathBuf;
use std::process::Command;

use serde_json::{Value, json};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Runs `script` in Windows PowerShell after dot-sourcing `codeberg-quota.ps1`, with `data` in
/// `$data`; returns whether it succeeded, what it printed and its error output.
fn powershell(data: &Value, script: &str) -> (bool, String, String) {
    let quota = root().join("scripts").join("codeberg-quota.ps1");
    // the error's message alone, unwrapped: PowerShell's own error display breaks lines at its width
    let full = format!(
        "$ErrorActionPreference = 'Stop'; try {{ . '{}'; $data = $env:RELEASE_TEST_DATA | ConvertFrom-Json; {script} }} \
         catch {{ [Console]::Error.WriteLine($_.Exception.Message); exit 1 }}",
        quota.display()
    );
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &full])
        .env("RELEASE_TEST_DATA", data.to_string())
        .output()
        .expect("Windows PowerShell runs");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).trim().to_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn release(id: u64, tag: &str, created: &str, assets: &[(u64, &str, u64)]) -> Value {
    json!({
        "id": id,
        "tag_name": tag,
        "created_at": created,
        "body": format!("notes of {tag}"),
        "assets": assets.iter().map(|(id, name, size)| json!({"id": id, "name": name, "size": size})).collect::<Vec<_>>(),
    })
}

/// Three older releases, listed newest first as Codeberg lists them, and the one being made.
fn releases() -> Value {
    json!([
        release(40, "v0.4.0", "2026-11-26T10:00:00Z", &[]),
        release(30, "v0.3.0", "2026-11-20T10:00:00Z", &[(302, "c-setup.exe", 50), (301, "c-portable.exe", 70)]),
        release(20, "v0.2.0", "2026-11-10T10:00:00Z", &[(201, "b.exe", 30), (202, "b.exe.b3", 1)]),
        release(10, "v0.1.0", "2026-11-01T10:00:00Z", &[(101, "a.exe", 20)]),
    ])
}

/// The names `Select-CodebergFilesToFree` chooses for `needed` bytes, in order, or its error.
fn chosen(needed: i64) -> Result<Vec<String>, String> {
    let (ok, out, err) = powershell(
        &releases(),
        &format!(
            "$files = Select-CodebergFilesToFree -Releases @($data | ForEach-Object {{ $_ }}) -KeepTag 'v0.4.0' -Needed {needed}; \
             @($files | ForEach-Object {{ $_.Name }}) -join ','"
        ),
    );
    if ok { Ok(out.split(',').filter(|name| !name.is_empty()).map(str::to_owned).collect()) } else { Err(err) }
}

#[test]
fn every_release_script_parses() {
    let scripts = ["scripts", "packaging/windows"]
        .iter()
        .flat_map(|folder| std::fs::read_dir(root().join(folder)).unwrap())
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "ps1"))
        .collect::<Vec<_>>();
    assert!(scripts.len() >= 8, "found {scripts:?}");
    for script in scripts {
        let (ok, out, err) = powershell(
            &json!(null),
            &format!(
                "$errors = $null; [void][System.Management.Automation.Language.Parser]::ParseFile('{}', [ref]$null, [ref]$errors); \
                 @($errors | ForEach-Object {{ $_.Message }}) -join '; '",
                script.display()
            ),
        );
        assert!(ok && out.is_empty(), "{}: {out}{err}", script.display());
    }
}

#[test]
fn the_shortfall_is_what_is_over_the_limit_plus_the_uploads() {
    let quota = json!({
        "used": {"size": {"assets": {"attachments": {"releases": 1_100}}}},
        "groups": [{"rules": [
            {"limit": 9_999, "subjects": ["size:repos:all"]},
            {"limit": 1_000, "subjects": ["size:assets:all", "size:git:lfs"]},
        ]}],
    });
    let shortfall =
        |upload: u32| powershell(&quota, &format!("Get-CodebergShortfall -Quota $data -ToUpload {upload}")).1;
    assert_eq!(shortfall(50), "150", "100 over the limit, and 50 more to upload");
    let (ok, _, err) =
        powershell(&json!({"used": quota["used"], "groups": []}), "Get-CodebergShortfall -Quota $data -ToUpload 1");
    assert!(!ok && err.contains("no limit for release files"), "{err}");
}

#[test]
fn the_oldest_files_go_first_and_only_as_many_as_needed() {
    assert_eq!(chosen(15), Ok(vec!["a.exe".to_owned()]), "one file is enough");
    assert_eq!(chosen(21), Ok(vec!["a.exe".to_owned(), "b.exe".to_owned()]), "then the next oldest release's");
    assert_eq!(
        chosen(52),
        Ok(["a.exe", "b.exe", "b.exe.b3", "c-portable.exe"].map(str::to_owned).to_vec()),
        "a release's files go in the order they were uploaded"
    );
    assert_eq!(chosen(0), Ok(vec![]), "nothing when the files fit");
}

#[test]
fn the_new_release_keeps_its_files_and_too_little_room_deletes_nothing() {
    let all_older = 20 + 30 + 1 + 70 + 50;
    assert_eq!(chosen(all_older).map(|names| names.len()), Ok(5), "every older file, and none of v0.4.0's");
    let error = chosen(all_older + 1).unwrap_err();
    assert!(error.contains("hold only 171") && error.contains("nothing was deleted"), "{error}");
}

#[test]
fn a_stripped_release_points_to_github_once() {
    let notes = |body: &str| {
        powershell(
            &json!({"body": body}),
            "Get-NotesWithGitHubPointer -Body $data.body -Tag 'v0.2.0' -GitHubRepo 'Ran-n/avalio-lite'",
        )
        .1
    };
    let line = "every file is on GitHub: https://github.com/Ran-n/avalio-lite/releases/tag/v0.2.0";
    let once = notes("### Added\n\n- a thing\n");
    assert!(once.starts_with("### Added") && once.ends_with(line), "{once}");
    assert_eq!(notes(&once), once, "a second strip doesn't add the line again");
    assert!(notes("").ends_with(line), "a release with no notes gets the line alone");
}
