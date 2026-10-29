[//]: # ( ---------------------------------------------------------------------- )
[//]: # (+ Authors: 	Ran# <ran.hash@proton.me> )
[//]: # (+ Created: 	2026/10/27 19:34:10.315159 )
[//]: # (+ Revised: 	2026/10/29 02:05:29.521079 )
[//]: # ( ---------------------------------------------------------------------- )

# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
