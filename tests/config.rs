// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:31.961496
//+ Revised: 	2026/11/24 16:26:03.379178
// -----------------------------------------------------------------------------

//! The settings file: what it keeps, its format, older versions, and the folder it lives in.

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use avalio_lite::error::AppError;
use avalio_lite::i18n::Language;
use avalio_lite::platform::config::{
    APP_DIR_NAME, AppConfig, CONFIG_FILE_NAME, ConfigStore, Preferences, VENDOR_DIR_NAME, WindowPlace,
    default_data_dir, move_legacy_settings,
};

fn sample() -> AppConfig {
    AppConfig {
        database_path: PathBuf::from(r"D:\data\avalio-lite.db"),
        log_dir: PathBuf::from(r"D:\logs"),
        preferences: Preferences { show_column_settings: false, ..Preferences::default() },
        shortcuts: BTreeMap::new(),
        last_class: None,
    }
}

#[test]
fn load_returns_none_on_first_run() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(ConfigStore::in_dir(dir.path()).load().unwrap(), None);
}

#[test]
fn save_then_load_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let store = ConfigStore::in_dir(&dir.path().join("nested"));
    store.save(&sample()).unwrap();
    assert_eq!(store.load().unwrap(), Some(sample()));
}

#[test]
fn save_leaves_no_temporary_file() {
    let dir = tempfile::tempdir().unwrap();
    ConfigStore::in_dir(dir.path()).save(&sample()).unwrap();
    let names: Vec<_> = fs::read_dir(dir.path()).unwrap().map(|entry| entry.unwrap().file_name()).collect();
    assert_eq!(names, vec![CONFIG_FILE_NAME]);
}

#[test]
fn saved_file_format_is_stable() {
    let dir = tempfile::tempdir().unwrap();
    let store = ConfigStore::in_dir(dir.path());
    store.save(&sample()).unwrap();
    let text = fs::read_to_string(store.path()).unwrap();
    assert_eq!(
        text,
        "database_path = 'D:\\data\\avalio-lite.db'\nlog_dir = 'D:\\logs'\n\n[preferences]\nshow_column_settings = false\ncheck_automatically = true\n"
    );
}

/// Only a version is kept as the skipped one: anything else (hand-edited, or longer than 64
/// characters) reads as none, so the update is offered again.
#[test]
fn a_skipped_version_that_is_not_a_version_reads_as_none() {
    let skipped = |written: &str| {
        let dir = tempfile::tempdir().unwrap();
        let text = format!(
            "database_path = 'D:\\data\\a.db'\nlog_dir = 'D:\\logs'\n\n[preferences]\nskipped_version = \"{written}\"\n"
        );
        fs::write(dir.path().join("config.toml"), text).unwrap();
        ConfigStore::in_dir(dir.path()).load().unwrap().unwrap().preferences.skipped_version
    };
    let too_long = format!("1.0.0-{}", "a".repeat(60));
    for written in ["soon", "1.0", "", too_long.as_str()] {
        assert_eq!(skipped(written), None, "{written:?}");
    }
    assert_eq!(skipped("1.2.3-beta.1").as_deref(), Some("1.2.3-beta.1"));
}

#[test]
fn chosen_language_is_saved_as_its_code() {
    let dir = tempfile::tempdir().unwrap();
    let store = ConfigStore::in_dir(dir.path());
    let mut config = sample();
    config.preferences.language = Some(Language::Galician);
    store.save(&config).unwrap();
    assert!(fs::read_to_string(store.path()).unwrap().contains("language = \"gl\""));
    assert_eq!(store.load().unwrap().unwrap().preferences.language, Some(Language::Galician));
}

#[test]
fn unknown_language_codes_are_reported() {
    let dir = tempfile::tempdir().unwrap();
    let store = ConfigStore::in_dir(dir.path());
    fs::write(store.path(), "database_path = 'a'\nlog_dir = 'b'\n[preferences]\nlanguage = 'xx'\n").unwrap();
    assert!(matches!(store.load(), Err(AppError::ConfigParse { .. })));
}

#[test]
fn settings_files_from_before_preferences_still_load_with_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let store = ConfigStore::in_dir(dir.path());
    fs::write(store.path(), "database_path = 'D:\\data\\avalio-lite.db'\nlog_dir = 'D:\\logs'\n").unwrap();
    let loaded = store.load().unwrap().unwrap();
    assert_eq!(loaded.preferences, Preferences::default());
    assert!(loaded.preferences.show_column_settings, "settings are shown until the user hides them");
    assert!(loaded.shortcuts.is_empty(), "no [shortcuts] table means every default");
}

#[test]
fn shortcuts_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let store = ConfigStore::in_dir(dir.path());
    let mut config = sample();
    config.shortcuts.insert("lock".into(), "Ctrl+K".into());
    config.shortcuts.insert("next-class".into(), "F8".into());
    store.save(&config).unwrap();
    let text = fs::read_to_string(store.path()).unwrap();
    assert!(text.contains("[shortcuts]\nlock = \"Ctrl+K\"\nnext-class = \"F8\"\n"), "{text}");
    assert_eq!(store.load().unwrap(), Some(config));
}

#[test]
fn no_shortcuts_table_is_written_when_nothing_changed() {
    let dir = tempfile::tempdir().unwrap();
    let store = ConfigStore::in_dir(dir.path());
    store.save(&sample()).unwrap();
    assert!(!fs::read_to_string(store.path()).unwrap().contains("[shortcuts]"));
}

#[test]
fn unreadable_file_is_reported_not_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let store = ConfigStore::in_dir(dir.path());
    fs::write(store.path(), "this is = = not toml").unwrap();
    assert!(matches!(store.load(), Err(AppError::ConfigParse { .. })));
}

#[test]
fn a_settings_file_that_cannot_be_read_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    let store = ConfigStore::in_dir(dir.path());
    fs::create_dir(store.path()).unwrap();
    assert!(matches!(store.load(), Err(AppError::Io { .. })), "a folder where the file should be");
}

#[test]
fn missing_field_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    let store = ConfigStore::in_dir(dir.path());
    fs::write(store.path(), "log_dir = 'D:\\logs'\n").unwrap();
    assert!(matches!(store.load(), Err(AppError::ConfigParse { .. })));
}

#[test]
fn default_data_dir_ends_with_app_folder() {
    assert!(default_data_dir().ends_with("avalio-lite"));
}

#[test]
fn an_override_folder_wins_over_the_system_one() {
    let dir = tempfile::tempdir().unwrap();
    let store = ConfigStore::location(Some(dir.path().to_path_buf()), Some(r"C:\elsewhere".into())).unwrap();
    assert_eq!(store.path(), dir.path().join(CONFIG_FILE_NAME));
}

#[test]
fn without_an_override_the_publishers_folder_inside_the_system_one_is_used() {
    let store = ConfigStore::location(None, Some(r"C:\Users\me\AppData\Roaming".into())).unwrap();
    assert_eq!(store.path(), std::path::Path::new(r"C:\Users\me\AppData\Roaming\Breren\avalio-lite\config.toml"));
}

/// `%APPDATA%` in a scratch folder: the folder of 0.10.0 and earlier, and the one used now.
fn settings_folders() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let base = tempfile::tempdir().unwrap();
    let legacy = base.path().join(APP_DIR_NAME);
    let dir = base.path().join(VENDOR_DIR_NAME).join(APP_DIR_NAME);
    (base, legacy, dir)
}

#[test]
fn the_settings_of_an_older_version_are_moved_to_the_publishers_folder() {
    let (base, legacy, dir) = settings_folders();
    fs::create_dir_all(&legacy).unwrap();
    fs::write(legacy.join(CONFIG_FILE_NAME), "database_path = 'x'").unwrap();
    fs::write(legacy.join("config.toml.bad"), "broken").unwrap();
    let store = ConfigStore::location(None, Some(base.path().to_path_buf())).unwrap();
    assert_eq!(store.path(), dir.join(CONFIG_FILE_NAME));
    assert_eq!(fs::read_to_string(dir.join(CONFIG_FILE_NAME)).unwrap(), "database_path = 'x'");
    assert_eq!(fs::read_to_string(dir.join("config.toml.bad")).unwrap(), "broken", "the kept copy goes along");
    assert!(!legacy.exists(), "the old folder, now empty, is removed");
}

#[test]
fn an_old_folder_holding_anything_else_stays() {
    let (_base, legacy, dir) = settings_folders();
    fs::create_dir_all(&legacy).unwrap();
    fs::write(legacy.join(CONFIG_FILE_NAME), "old").unwrap();
    fs::write(legacy.join("notes.txt"), "the user's").unwrap();
    assert_eq!(move_legacy_settings(&legacy, &dir), dir);
    assert!(!legacy.join(CONFIG_FILE_NAME).exists());
    assert_eq!(fs::read_to_string(legacy.join("notes.txt")).unwrap(), "the user's");
}

#[test]
fn settings_already_in_the_publishers_folder_are_never_overwritten() {
    let (_base, legacy, dir) = settings_folders();
    for (folder, text) in [(&legacy, "old"), (&dir, "new")] {
        fs::create_dir_all(folder).unwrap();
        fs::write(folder.join(CONFIG_FILE_NAME), text).unwrap();
    }
    assert_eq!(move_legacy_settings(&legacy, &dir), dir);
    assert_eq!(fs::read_to_string(dir.join(CONFIG_FILE_NAME)).unwrap(), "new");
    assert_eq!(fs::read_to_string(legacy.join(CONFIG_FILE_NAME)).unwrap(), "old", "left alone");
}

#[test]
fn with_nothing_to_move_the_publishers_folder_is_used() {
    let (_base, legacy, dir) = settings_folders();
    assert_eq!(move_legacy_settings(&legacy, &dir), dir);
    assert!(!dir.exists() && !legacy.exists(), "nothing is created before the first save");
}

#[test]
fn a_move_that_fails_keeps_using_the_old_folder() {
    let (base, legacy, _) = settings_folders();
    fs::create_dir_all(&legacy).unwrap();
    fs::write(legacy.join(CONFIG_FILE_NAME), "old").unwrap();
    // a file where the publisher's folder should be: the new folder can't be made
    fs::write(base.path().join(VENDOR_DIR_NAME), "in the way").unwrap();
    let dir = base.path().join(VENDOR_DIR_NAME).join(APP_DIR_NAME);
    assert_eq!(move_legacy_settings(&legacy, &dir), legacy);
    assert_eq!(fs::read_to_string(legacy.join(CONFIG_FILE_NAME)).unwrap(), "old", "the settings are untouched");
    let store = ConfigStore::location(None, Some(base.path().to_path_buf())).unwrap();
    assert_eq!(store.path(), legacy.join(CONFIG_FILE_NAME));
}

#[test]
fn no_settings_folder_at_all_is_reported() {
    assert!(matches!(ConfigStore::location(None, None), Err(AppError::NoConfigDir)));
}

#[test]
fn settings_that_cannot_be_written_are_reported() {
    let dir = tempfile::tempdir().unwrap();
    let in_the_way = dir.path().join("config");
    fs::write(&in_the_way, b"a file where the folder should be").unwrap();
    let result = ConfigStore::in_dir(&in_the_way).save(&sample());
    assert!(matches!(result, Err(AppError::Io { .. })), "{result:?}");
}

fn any_language() -> impl proptest::strategy::Strategy<Value = Option<Language>> {
    proptest::option::of(proptest::sample::select(avalio_lite::i18n::LANGUAGES.to_vec()))
}

fn any_theme() -> impl proptest::strategy::Strategy<Value = Option<avalio_lite::platform::config::ThemeChoice>> {
    use avalio_lite::platform::config::ThemeChoice;
    proptest::option::of(proptest::sample::select(vec![ThemeChoice::Light, ThemeChoice::Dark]))
}

proptest::proptest! {
    #![proptest_config(proptest::prelude::ProptestConfig::with_cases(64))]

    #[test]
    fn any_settings_survive_a_save_and_load(
        database in r"[A-Z]:\\[[^\p{C}]&&[^\\/:*?<>|]]{1,20}\\[a-z ]{1,12}\.db",
        logs in r"[A-Z]:\\[[^\p{C}]&&[^\\/:*?<>|]]{1,20}",
        show in proptest::bool::ANY,
        collapsed in proptest::bool::ANY,
        check_automatically in proptest::bool::ANY,
        last_check in proptest::option::of(0i64..4_102_444_800),
        skipped in proptest::option::of(proptest::strategy::Strategy::prop_map(
            (0u16..1000, 0u16..1000, 0u16..1000),
            |(major, minor, patch)| format!("{major}.{minor}.{patch}")
        )),
        language in any_language(),
        theme in any_theme(),
        shortcuts in proptest::collection::btree_map("[a-z-]{1,22}", "[ -~]{0,16}", 0..4),
        last_class in proptest::option::of(1i64..1_000_000),
        window in proptest::option::of((proptest::prelude::any::<i32>(), proptest::prelude::any::<i32>(), proptest::prelude::any::<u32>(), proptest::prelude::any::<u32>(), proptest::prelude::any::<bool>())),
    ) {
        let dir = tempfile::tempdir().unwrap();
        let store = ConfigStore::in_dir(dir.path());
        let config = AppConfig {
            database_path: PathBuf::from(database),
            log_dir: PathBuf::from(logs),
            preferences: Preferences {
                show_column_settings: show,
                language,
                theme,
                sidebar_collapsed: collapsed,
                check_automatically,
                last_check,
                skipped_version: skipped.clone(),
                last_run_version: skipped,
                update_failed: None,
                window: window.map(|(x, y, width, height, maximized)| WindowPlace { x, y, width, height, maximized }),
            },
            shortcuts,
            last_class,
        };
        store.save(&config).unwrap();
        proptest::prop_assert_eq!(store.load().unwrap(), Some(config));
    }
}

#[test]
fn regression_saving_over_an_unreadable_settings_file_keeps_a_copy() {
    let dir = tempfile::tempdir().unwrap();
    let store = ConfigStore::in_dir(dir.path());
    fs::write(store.path(), "database_path = [broken").unwrap();
    assert!(matches!(store.load(), Err(AppError::ConfigParse { .. })));
    store.save(&sample()).unwrap();
    assert_eq!(fs::read_to_string(dir.path().join("config.toml.bad")).unwrap(), "database_path = [broken");
    assert_eq!(store.load().unwrap(), Some(sample()));
    // later saves do not copy again
    store.save(&sample()).unwrap();
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
}

#[test]
fn a_second_unreadable_file_gets_a_timestamped_copy_and_the_first_is_kept() {
    let dir = tempfile::tempdir().unwrap();
    let store = ConfigStore::in_dir(dir.path());
    for broken in ["first = [", "second = ["] {
        fs::write(store.path(), broken).unwrap();
        assert!(store.load().is_err());
        store.save(&sample()).unwrap();
    }
    assert_eq!(fs::read_to_string(dir.path().join("config.toml.bad")).unwrap(), "first = [");
    let copies: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("config.toml.bad-"))
        .collect();
    assert_eq!(copies.len(), 1);
    assert_eq!(fs::read_to_string(dir.path().join(&copies[0])).unwrap(), "second = [");
}

#[test]
fn a_readable_or_missing_settings_file_leaves_no_copy() {
    let dir = tempfile::tempdir().unwrap();
    let store = ConfigStore::in_dir(dir.path());
    assert_eq!(store.load().unwrap(), None);
    store.save(&sample()).unwrap();
    store.load().unwrap();
    store.save(&sample()).unwrap();
    assert!(!dir.path().join("config.toml.bad").exists());
}

#[test]
fn a_failed_rename_leaves_no_temporary_file() {
    let dir = tempfile::tempdir().unwrap();
    let store = ConfigStore::in_dir(dir.path());
    // a directory at the settings path makes the final rename fail
    fs::create_dir(store.path()).unwrap();
    assert!(matches!(store.save(&sample()), Err(AppError::Io { .. })));
    assert!(!dir.path().join("config.toml.tmp").exists());
}

#[test]
fn an_unreadable_settings_file_that_is_gone_by_the_save_needs_no_copy() {
    let dir = tempfile::tempdir().unwrap();
    let store = ConfigStore::in_dir(dir.path());
    fs::write(store.path(), "database_path = [broken").unwrap();
    assert!(store.load().is_err());
    fs::remove_file(store.path()).unwrap();
    store.save(&sample()).unwrap();
    assert!(!dir.path().join("config.toml.bad").exists(), "nothing was left to keep");
    assert_eq!(store.load().unwrap(), Some(sample()));
}

#[test]
fn a_settings_file_without_the_sidebar_preference_opens_it_expanded() {
    let dir = tempfile::tempdir().unwrap();
    let store = ConfigStore::in_dir(dir.path());
    std::fs::write(
        store.path(),
        r"database_path = 'D:\data\avalio-lite.db'
log_dir = 'D:\logs'

[preferences]
show_column_settings = true
",
    )
    .unwrap();
    assert!(!store.load().unwrap().unwrap().preferences.sidebar_collapsed);

    let mut collapsed = sample();
    collapsed.preferences.sidebar_collapsed = true;
    store.save(&collapsed).unwrap();
    assert!(store.load().unwrap().unwrap().preferences.sidebar_collapsed, "true round-trips");
}

#[test]
fn an_old_settings_file_that_cannot_be_removed_stays_behind_the_moved_one() {
    let (_base, legacy, dir) = settings_folders();
    fs::create_dir_all(&legacy).unwrap();
    let old = legacy.join(CONFIG_FILE_NAME);
    fs::write(&old, "old").unwrap();
    let held = common::hold_against_removal(&old);
    assert_eq!(move_legacy_settings(&legacy, &dir), dir);
    drop(held);
    assert_eq!(fs::read_to_string(dir.join(CONFIG_FILE_NAME)).unwrap(), "old");
    assert!(old.exists(), "only a leftover: the new folder wins at the next start");
}

#[test]
fn a_kept_bad_copy_already_in_the_publishers_folder_is_never_overwritten() {
    let (_base, legacy, dir) = settings_folders();
    fs::create_dir_all(&legacy).unwrap();
    fs::create_dir_all(&dir).unwrap();
    fs::write(legacy.join(CONFIG_FILE_NAME), "old").unwrap();
    for (folder, text) in [(&legacy, "older"), (&dir, "newer")] {
        fs::write(folder.join("config.toml.bad"), text).unwrap();
    }
    assert_eq!(move_legacy_settings(&legacy, &dir), dir);
    assert_eq!(fs::read_to_string(dir.join(CONFIG_FILE_NAME)).unwrap(), "old", "the settings move");
    assert_eq!(fs::read_to_string(dir.join("config.toml.bad")).unwrap(), "newer");
    assert_eq!(fs::read_to_string(legacy.join("config.toml.bad")).unwrap(), "older", "and nothing is lost");
}

#[test]
fn a_kept_bad_copy_that_cannot_be_moved_stays_and_the_settings_move() {
    let (_base, legacy, dir) = settings_folders();
    fs::create_dir_all(&legacy).unwrap();
    fs::write(legacy.join(CONFIG_FILE_NAME), "old").unwrap();
    let bad = legacy.join("config.toml.bad");
    fs::write(&bad, "broken").unwrap();
    let held = common::hold_exclusively(&bad);
    assert_eq!(move_legacy_settings(&legacy, &dir), dir);
    drop(held);
    assert!(bad.exists() && dir.join(CONFIG_FILE_NAME).exists());
    assert!(!dir.join("config.toml.bad").exists());
}
