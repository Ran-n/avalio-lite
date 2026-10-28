[//]: # ( ---------------------------------------------------------------------- )
[//]: # (+ Authors: 	Ran# <ran.hash@proton.me> )
[//]: # (+ Created: 	2026/10/27 19:34:10.315159 )
[//]: # (+ Revised: 	2026/10/28 23:14:50.366886 )
[//]: # ( ---------------------------------------------------------------------- )

# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
