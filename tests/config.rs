// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:31.961496
//+ Revised: 	2026/10/28 15:53:09.190732
// -----------------------------------------------------------------------------

use std::fs;
use std::path::PathBuf;

use avalio_lite::config::{AppConfig, CONFIG_FILE_NAME, ConfigStore, Preferences, default_data_dir};
use avalio_lite::error::AppError;
use avalio_lite::i18n::Language;

fn sample() -> AppConfig {
    AppConfig {
        database_path: PathBuf::from(r"D:\data\avalio-lite.db"),
        log_dir: PathBuf::from(r"D:\logs"),
        preferences: Preferences { show_column_settings: false, language: None, theme: None },
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
        "database_path = 'D:\\data\\avalio-lite.db'\nlog_dir = 'D:\\logs'\n\n[preferences]\nshow_column_settings = false\n"
    );
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
}

#[test]
fn unreadable_file_is_reported_not_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let store = ConfigStore::in_dir(dir.path());
    fs::write(store.path(), "this is = = not toml").unwrap();
    assert!(matches!(store.load(), Err(AppError::ConfigParse { .. })));
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
fn without_an_override_the_app_folder_inside_the_system_one_is_used() {
    let store = ConfigStore::location(None, Some(r"C:\Users\me\AppData\Roaming".into())).unwrap();
    assert_eq!(store.path(), std::path::Path::new(r"C:\Users\me\AppData\Roaming\avalio-lite\config.toml"));
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

fn any_theme() -> impl proptest::strategy::Strategy<Value = Option<avalio_lite::config::ThemeChoice>> {
    use avalio_lite::config::ThemeChoice;
    proptest::option::of(proptest::sample::select(vec![ThemeChoice::Light, ThemeChoice::Dark]))
}

proptest::proptest! {
    #![proptest_config(proptest::prelude::ProptestConfig::with_cases(64))]

    #[test]
    fn any_settings_survive_a_save_and_load(
        database in r"[A-Z]:\\[[^\p{C}]&&[^\\/:*?<>|]]{1,20}\\[a-z ]{1,12}\.db",
        logs in r"[A-Z]:\\[[^\p{C}]&&[^\\/:*?<>|]]{1,20}",
        show in proptest::bool::ANY,
        language in any_language(),
        theme in any_theme(),
    ) {
        let dir = tempfile::tempdir().unwrap();
        let store = ConfigStore::in_dir(dir.path());
        let config = AppConfig {
            database_path: PathBuf::from(database),
            log_dir: PathBuf::from(logs),
            preferences: Preferences { show_column_settings: show, language, theme },
        };
        store.save(&config).unwrap();
        proptest::prop_assert_eq!(store.load().unwrap(), Some(config));
    }
}
