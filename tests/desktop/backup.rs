// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:02.112958
//+ Revised: 	2026/11/24 17:54:07.301772
// -----------------------------------------------------------------------------

//! The backup card on the Settings page: exporting everything, how the card and the restore prompt
//! look, and the restore prompt in Galician. The restore reads a backup made elsewhere, with its own PIN.

use std::path::{Path, PathBuf};

use crate::common::Fixture;
use crate::desktop_support::{self, App};

use super::fixtures::{PIN, config, with_sample_class};
use super::screens::{MINIMUM, assert_on_screen, every_look_after, galician_look};

/// Settings → "Export everything…", saved to `saved.avalio` in the scratch folder the app runs in.
fn export_everything(app: &mut App) {
    app.click("Settings");
    app.scroll_into_view("Add classes from a backup…");
    app.answer_saves(&[Path::new("saved.avalio")]);
    app.click("Export everything…");
}

/// A backup of a sample class made elsewhere, with another PIN, so the restore asks for it. The
/// fixture it is in must live as long as the backup is used.
fn make_backup() -> (Fixture, PathBuf) {
    let other = Fixture::new();
    let mut state = other.set_up();
    crate::common::sample_class(state.connection().unwrap());
    state.set_pin(PIN, PIN).unwrap();
    let backup = other.path("avalio-lite-backup-2026-10-28.avalio");
    state.export_backup(&backup).unwrap();
    state.shutdown();
    (other, backup)
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn exporting_everything_saves_a_backup_file() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    export_everything(&mut app);
    let state = app.state();
    assert_eq!(state["status"], "Backup saved to saved.avalio", "{state}");
    assert!(fixture.path("saved.avalio").is_file());
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_backup_card_looks_right() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    // the card is captured after the export, which its status line then shows
    export_everything(&mut app);
    every_look_after(
        &mut app,
        "settings-backup",
        |app| app.scroll_into_view("Add classes from a backup…"),
        |app, image, _, _| {
            for button in ["Export everything…", "Restore from a backup…", "Add classes from a backup…"] {
                assert_on_screen(image, app.located(button), button);
            }
        },
    );
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_restore_prompt_looks_right() {
    let _turn = desktop_support::turn();
    let (_other, backup) = make_backup();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    export_everything(&mut app);
    app.scroll_into_view("Restore from a backup…");
    app.answer_dialogs(&[], &[&backup], &[]);
    app.click("Restore from a backup…");
    every_look_after(
        &mut app,
        "settings-backup-pin",
        |app| app.scroll_into_view("Continue"),
        |app, image, _, _| {
            for control in ["Backup PIN", "Continue", "Cancel"] {
                assert_on_screen(image, app.located(control), control);
            }
        },
    );
    app.click("Cancel");
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_restore_prompt_in_galician() {
    let _turn = desktop_support::turn();
    let (_other, backup) = make_backup();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "gl");
    app.click("Axustes");
    app.scroll_into_view("Engadir clases dunha copia…");
    app.answer_dialogs(&[], &[&backup], &[]);
    app.click("Restaurar unha copia…");
    // the prompt opens below the buttons: at the minimum size it needs the page scrolled down
    app.resize(MINIMUM.0, MINIMUM.1);
    app.scroll_into_view("Continuar");
    let image = galician_look(&mut app, "settings-backup-pin");
    for control in ["Engadir clases dunha copia…", "PIN da copia", "Continuar", "Cancelar"] {
        assert_on_screen(&image, app.located(control), control);
    }
    app.click("Cancelar");
    app.quit();
}
