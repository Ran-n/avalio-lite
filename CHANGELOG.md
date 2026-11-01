[//]: # ( ---------------------------------------------------------------------- )
[//]: # (+ Authors: 	Ran# <ran.hash@proton.me> )
[//]: # (+ Created: 	2026/10/27 19:34:10.315159 )
[//]: # (+ Revised: 	2026/10/29 15:46:50.995740 )
[//]: # ( ---------------------------------------------------------------------- )

# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Keyboard shortcuts for the theme (Ctrl+Shift+D), weights and scales (Ctrl+H), Settings (Ctrl+,) and switching classes (Ctrl+PageDown/PageUp), alongside Ctrl+L to lock; listed in Settings and changeable in config.toml under `[shortcuts]`, with bad entries reported and replaced by their defaults
- Keyboard shortcuts can be changed (press Change, then the new keys) and reset, one or all, in Settings; plain letters, Ctrl+Alt (AltGr), the text-editing keys Ctrl+A/C/V/X/Y/Z and keys another shortcut has are refused with the reason
- Settings → Backup → Export everything…: saves the whole database to one .avalio file, encrypted with the PIN when there is one
- Settings → Backup → Restore from a backup…: replaces the app's data with an .avalio backup, keeping the current PIN; a backup made with another PIN asks for it in the card
- Settings → Backup → Add classes from a backup…: copies another backup's classes into the app, with their pupils and marks; a taken class name gets " (2)", and a pupil already here with the same name is joined
- The avalio-lite logo (a marking tick on the accent tile) in the sidebar, first-run and lock screens, the window and taskbar, the program file, the installer and the README
- Export to Excel… in the sheet's ⋯ menu: saves the sheet as an .xlsx workbook, marks as numbers, with a second tab listing each column's settings and a note when the final grades are provisional
- Import from Excel… in the sheet's ⋯ menu: adds the pupils of an .xlsx, .xls or .ods file to the class and fills the marks under columns with the same titles, asking before it replaces any

### Changed

- Line coverage raised to 98 % and the gate with it; the coverage script lists the few lines left uncovered on purpose, and a desktop test now runs the real exe with its Windows dialogs
- The coverage gate checks branches (92 %) as well as lines, on the nightly toolchain; release.ps1 runs both, and the coverage script's -Branches mode, which never worked, now does
- Tests strengthened after mutation testing (cargo-mutants, configured in `.cargo/mutants.toml`)

### Fixed

- When the database had gone missing, finishing setup saved the default language and theme over the chosen ones; the settings file's preferences (and shortcuts) are now kept
- A greyed-out button could still be pressed through a screen reader

## [0.2.1] - 2026-10-29

### Fixed

- Screen readers couldn't press the lock screen's on-screen keys (numbers, letters, symbols, erase, unlock)

### Changed

- The README links the release downloads on GitHub, GitLab and Codeberg and lists publishing them as a release step

## [0.2.0] - 2026-10-29

### Added

- Letter and symbol keys on the lock screen: "ABC" switches the on-screen keypad to a letter keyboard (with ñ, capitals and space) and "#+=" to two pages of symbols, so password PINs can be clicked too; "123" switches back; the unlock button now sits next to the PIN field
- "Forgot the PIN?" on the lock screen: leaves it for the setup screen to open another database or start a new one, leaving the locked file untouched; "Back to the lock screen" returns, and a running guess wait keeps running
- The database file itself refuses typed values in final grade, average and rounded columns, and drops any already there (migration 004)

### Fixed

- A PIN change that couldn't finish swapping the files could leave the database renamed aside and open an empty one in its place; the original is now always put back, and the app never creates a file when reopening after a re-key
- A PIN change that succeeded but couldn't delete the old file was reported as failed and left the app asking for the old PIN; it now opens with the new PIN and reports only the leftover file
- Unlocking a database whose file has gone (e.g. on an unplugged drive) repeated "doesn't exist" forever; it now shows the setup screen, as a restart would
- After renaming a pupil or a class, the remove and delete questions (and the screen-reader label of the pupil's cells) still used the old name

## [0.1.0] - 2026-10-28

### Added

- Native Windows app (Rust + Slint): one spreadsheet-style sheet per class, rows = pupils, saved as you type
- Pupils shared between classes: renaming applies everywhere, removing from one class deletes only its marks
- Column kinds: info, test (score out of a maximum, weight %, optional bonus), final grade, average, rounded
- Weight check: final grades turn provisional with a warning while normal tests don't add up to 100 %
- Class summary: pupil count, class average, passing and failing
- Column settings panel, hideable weights and scales, "+ Add pupil" / "+ Add column" / "⋯" menus
- Light and dark theme, following Windows until chosen
- Galician and English interface, following Windows until chosen; decimal comma in Galician
- Optional PIN (4–128 characters, digits or letters) that encrypts the whole database with SQLCipher; lock screen, Ctrl+L, guess limiting; the PIN is wiped from memory on lock, quit and change
- User-chosen database and log folders, movable later (copy, verify, switch, delete); open another database
- The database file itself enforces that cells belong to pupils of their class and that rounded columns round a column of their own class (migration 003)
- Keyboard access: Tab reaches every button, Enter or Space presses it; screen readers can press them too
- Portable exe and per-user Inno Setup installer, both versioned, from `scripts/release.ps1`
