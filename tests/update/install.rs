// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:33.831411
//+ Revised: 	2026/11/24 07:24:32.853817
// -----------------------------------------------------------------------------

//! Installing: replacing the running program, relaunching it, leftover cleanup and the uninstall entry.

use crate::common::releases::{TARGET, version};
use crate::support::files_in;
use avalio_lite::update::download::PREFIX;
use avalio_lite::update::install::{LEFTOVER_AGE, clean_leftovers, relaunch, replace};
use avalio_lite::update::registration::{self, APP_ID, DISPLAY_NAME, Outcome, Record, Value};
use avalio_lite::update::release::UpdateError;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant, SystemTime};

// ---------------------------------------------------------------------------------------------
// Installing: replacing the program, relaunching, leftovers

/// The example program next to the test binaries (cargo builds examples for the tests).
fn example(name: &str) -> PathBuf {
    let deps = std::env::current_exe().unwrap();
    deps.parent().unwrap().parent().unwrap().join("examples").join(format!("{name}.exe"))
}

#[test]
fn a_running_program_really_replaces_its_own_binary() {
    let dir = tempfile::tempdir().unwrap();
    let app = dir.path().join("app.exe");
    fs::copy(example("replace_self"), &app).unwrap();
    // an old program an earlier update couldn't delete makes way for this one
    fs::write(dir.path().join("app.exe.old"), b"older version").unwrap();
    let new = dir.path().join("new.exe");
    fs::copy(env!("CARGO_BIN_EXE_avalio-lite-asset-name"), &new).unwrap();
    let status = Command::new(&app).arg(&new).status().unwrap();
    assert!(status.success());
    // the replaced copy is deleted after the old process exits; give it a moment
    let deadline = Instant::now() + Duration::from_secs(10);
    while fs::read(&app).ok() != fs::read(env!("CARGO_BIN_EXE_avalio-lite-asset-name")).ok()
        && Instant::now() < deadline
    {
        std::thread::sleep(Duration::from_millis(100));
    }
    assert_eq!(
        fs::read(&app).unwrap(),
        fs::read(env!("CARGO_BIN_EXE_avalio-lite-asset-name")).unwrap(),
        "app.exe is now the new program"
    );
    assert!(!new.exists(), "the download is removed");
    let output = Command::new(&app).args([TARGET, "setup"]).output().unwrap();
    assert!(String::from_utf8_lossy(&output.stdout).contains("-setup.exe"), "and it runs");
}

/// Runs a scratch copy of the program that replaces itself with `new`, and returns what it printed.
fn fails_to_replace_itself(app: &Path, new: &Path) -> String {
    fs::copy(example("replace_self"), app).unwrap();
    let output = Command::new(app).arg(new).output().unwrap();
    assert!(!output.status.success());
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// A failed copy (a full disk) must not cost the user the program: it stays where it was.
#[test]
fn regression_a_program_that_cannot_be_replaced_stays_in_place() {
    let dir = tempfile::tempdir().unwrap();
    let app = dir.path().join("app.exe");
    // copying a folder as a file fails, and so does deleting it as one
    let not_a_file = dir.path().join("download");
    fs::create_dir(&not_a_file).unwrap();
    for new in [not_a_file, dir.path().join("missing.exe")] {
        let said = fails_to_replace_itself(&app, &new);
        assert!(said.contains("app.exe"), "{said}");
        // a deletion planned for the program's exit would have happened by now
        std::thread::sleep(Duration::from_secs(1));
        assert!(fs::read(&app).ok() == fs::read(example("replace_self")).ok(), "the program is gone or changed");
        let mut left = files_in(dir.path());
        left.sort();
        assert_eq!(left, ["app.exe", "download"], "nothing half-made is left beside it");
    }
}

/// Something in the way of moving it aside: the program is left as it was.
#[test]
fn a_program_that_cannot_move_aside_stays_in_place() {
    let dir = tempfile::tempdir().unwrap();
    let (exe, new, aside) = (dir.path().join("app.exe"), dir.path().join("new.exe"), dir.path().join("app.exe.old"));
    fs::write(&exe, b"old version").unwrap();
    fs::write(&new, b"new version").unwrap();
    fs::create_dir(&aside).unwrap();
    let error = replace(&exe, &new).unwrap_err();
    assert!(matches!(&error, UpdateError::Replace { path, .. } if path.ends_with("app.exe")), "{error:?}");
    assert_eq!(fs::read(&exe).unwrap(), b"old version");
    assert!(!new.exists(), "the download is removed either way");
    assert!(!dir.path().join("app.exe.new").exists(), "and so is its copy");
}

#[test]
fn a_relaunch_passes_its_arguments_and_reports_a_missing_program() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("ran.txt");
    let script = format!("echo %1 %2> \"{}\"", marker.display());
    let batch = dir.path().join("relaunched.cmd");
    fs::write(&batch, script).unwrap();
    relaunch(Path::new("cmd.exe"), &["/c".into(), batch.display().to_string(), "first".into(), "second".into()])
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !marker.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(fs::read_to_string(&marker).unwrap().trim(), "first second");
    assert!(relaunch(&dir.path().join("missing.exe"), &[]).is_err());
}

fn age(path: &Path, by: Duration) {
    let file = fs::File::options().write(true).open(path).unwrap();
    file.set_modified(SystemTime::now() - by).unwrap();
}

#[test]
fn leftover_cleanup_removes_old_downloads_only() {
    let dir = tempfile::tempdir().unwrap();
    let (old, young, other) = (
        dir.path().join(format!("{PREFIX}old")),
        dir.path().join(format!("{PREFIX}young")),
        dir.path().join("keep.txt"),
    );
    for file in [&old, &young, &other] {
        fs::write(file, b"x").unwrap();
    }
    fs::create_dir(dir.path().join(format!("{PREFIX}folder"))).unwrap();
    age(&old, LEFTOVER_AGE + Duration::from_secs(60));
    age(&other, LEFTOVER_AGE * 3);
    age(&young, LEFTOVER_AGE - Duration::from_secs(60));
    clean_leftovers(dir.path(), None, SystemTime::now());
    let mut left = files_in(dir.path());
    left.sort();
    assert_eq!(left, [format!("{PREFIX}folder"), format!("{PREFIX}young"), "keep.txt".to_owned()]);
    assert_eq!(LEFTOVER_AGE, Duration::from_secs(40 * 60));
}

/// 0.10.0 left `<exe>.old` (and a half download as `<exe>.new`) when it swapped itself.
#[test]
fn leftover_cleanup_removes_what_0_10_0_left_next_to_the_program() {
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("avalio-lite.exe");
    for file in [&exe, &dir.path().join("avalio-lite.exe.old"), &dir.path().join("avalio-lite.exe.new")] {
        fs::write(file, b"x").unwrap();
    }
    clean_leftovers(&dir.path().join("downloads"), Some(&exe), SystemTime::now());
    assert_eq!(files_in(dir.path()), ["avalio-lite.exe"]);
}

#[test]
fn a_leftover_that_cannot_be_removed_stays_and_the_others_go() {
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("avalio-lite.exe");
    let (old, new) = (dir.path().join("avalio-lite.exe.old"), dir.path().join("avalio-lite.exe.new"));
    for file in [&old, &new] {
        fs::write(file, b"x").unwrap();
    }
    let held = crate::common::hold_against_removal(&old);
    clean_leftovers(&dir.path().join("downloads"), Some(&exe), SystemTime::now());
    drop(held);
    assert_eq!(files_in(dir.path()), ["avalio-lite.exe.old"], "tried again at the next start");
}

// ---------------------------------------------------------------------------------------------
// The uninstall entry

#[derive(Default)]
struct FakeRecord {
    values: HashMap<String, Value>,
    /// writing this value fails
    refuses: Option<&'static str>,
}

impl Record for FakeRecord {
    fn text(&self, name: &str) -> Option<String> {
        match self.values.get(name) {
            Some(Value::Text(text)) => Some(text.clone()),
            _ => None,
        }
    }
    fn number(&self, name: &str) -> Option<u32> {
        match self.values.get(name) {
            Some(Value::Number(number)) => Some(*number),
            _ => None,
        }
    }
    fn write(&mut self, name: &str, value: &Value) -> std::io::Result<()> {
        if self.refuses == Some(name) {
            return Err(std::io::Error::other("refused"));
        }
        self.values.insert(name.to_owned(), value.clone());
        Ok(())
    }
}

/// As the installer of 0.10.0 left it: its name with the version in it.
fn installed_at(folder: &str) -> FakeRecord {
    let mut record = FakeRecord::default();
    record.values.insert("InstallLocation".into(), Value::Text(format!("{folder}\\")));
    record.values.insert("DisplayName".into(), Value::Text("avalio-lite version 0.10.0".into()));
    record.values.insert("DisplayVersion".into(), Value::Text("0.10.0".into()));
    record
}

#[test]
fn the_installed_copy_brings_its_entry_up_to_date() {
    let mut record = installed_at(r"C:\Users\a\AppData\Local\Programs\avalio-lite");
    let folder = Path::new(r"c:\users\A\AppData\Local\Programs\avalio-lite");
    assert_eq!(registration::bring_up_to_date(&mut record, folder, &version("1.2.3")).unwrap(), Outcome::Updated);
    assert_eq!(record.text("DisplayName").as_deref(), Some(DISPLAY_NAME));
    assert_eq!(record.text("DisplayVersion").as_deref(), Some("1.2.3"));
    for (name, number) in [("VersionMajor", 1), ("MajorVersion", 1), ("VersionMinor", 2), ("MinorVersion", 2)] {
        assert_eq!(record.number(name), Some(number), "{name}");
    }
    assert_eq!(registration::bring_up_to_date(&mut record, folder, &version("1.2.3")).unwrap(), Outcome::AlreadyRight);
}

#[test]
fn a_portable_copy_elsewhere_or_no_installation_leaves_the_entry_alone() {
    let mut record = installed_at(r"C:\Programs\avalio-lite");
    assert_eq!(
        registration::bring_up_to_date(&mut record, Path::new(r"D:\USB"), &version("1.2.3")).unwrap(),
        Outcome::Elsewhere
    );
    assert_eq!(record.text("DisplayVersion").as_deref(), Some("0.10.0"));
    let mut nothing = FakeRecord::default();
    assert_eq!(
        registration::bring_up_to_date(&mut nothing, Path::new(r"D:\USB"), &version("1.2.3")).unwrap(),
        Outcome::NotRegistered
    );
    assert!(nothing.values.is_empty());
}

#[test]
fn the_version_is_written_last_so_a_partial_failure_is_retried() {
    let mut record = installed_at(r"C:\Programs\avalio-lite");
    record.refuses = Some("MinorVersion");
    assert!(
        registration::bring_up_to_date(&mut record, Path::new(r"C:\Programs\avalio-lite"), &version("1.2.3")).is_err()
    );
    assert_eq!(
        record.text("DisplayVersion").as_deref(),
        Some("0.10.0"),
        "still the old one, so the next start tries again"
    );
}

/// The key the app writes is the installer's: its fixed application id.
#[test]
fn the_entry_is_the_installers_app_id() {
    let script =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("packaging/windows/avalio-lite.iss")).unwrap();
    let app_id = script.lines().find_map(|line| line.strip_prefix("AppId=")).unwrap().trim();
    // Inno Setup escapes the opening brace by doubling it
    assert_eq!(app_id.replacen("{{", "{", 1), APP_ID);
    assert_eq!(
        registration::uninstall_key(),
        format!(r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{APP_ID}_is1")
    );
    // the installer lists the app under the same name the app writes after an update
    for setting in [format!("UninstallDisplayName={DISPLAY_NAME}"), format!("AppName={DISPLAY_NAME}")] {
        assert!(script.lines().any(|line| line.trim_end() == setting), "{setting} in avalio-lite.iss");
    }
    assert_eq!(DISPLAY_NAME, "avalio lite");
}

/// The real registry, under a scratch key of this user's that the test removes.
#[test]
fn the_windows_entry_reads_and_writes_text_and_numbers() {
    use avalio_lite::update::windows::UninstallEntry;
    let key = format!(r"Software\avalio-lite-tests\{}", uuid::Uuid::new_v4());
    let full = format!(r"HKCU\{key}");
    assert!(
        Command::new("reg")
            .args(["add", &full, "/v", "InstallLocation", "/d", r"C:\x\", "/f"])
            .output()
            .unwrap()
            .status
            .success()
    );
    let mut entry = UninstallEntry::at(&key);
    let outcome = registration::bring_up_to_date(&mut entry, Path::new(r"C:\x"), &version("3.4.5"));
    let read = (
        entry.text("DisplayVersion"),
        entry.text("DisplayName"),
        entry.number("MajorVersion"),
        entry.number("VersionMinor"),
    );
    let missing = entry.text("Nothing");
    // a value of another type than asked for is as good as none
    let wrong_type = (entry.number("DisplayVersion"), entry.text("MajorVersion"));
    Command::new("reg").args(["delete", r"HKCU\Software\avalio-lite-tests", "/f"]).output().unwrap();
    assert_eq!(outcome.unwrap(), Outcome::Updated);
    assert_eq!(read, (Some("3.4.5".into()), Some(DISPLAY_NAME.into()), Some(3), Some(4)));
    assert_eq!(missing, None);
    assert_eq!(wrong_type, (None, None));
    let mut gone = UninstallEntry::at(r"Software\avalio-lite-tests\gone");
    assert!(gone.write("DisplayVersion", &Value::Text("1".into())).is_err(), "no key, no write");
}
