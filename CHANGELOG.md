[//]: # ( ---------------------------------------------------------------------- )
[//]: # (+ Authors: 	Ran# <ran.hash@proton.me> )
[//]: # (+ Created: 	2026/10/27 19:34:10.315159 )
[//]: # (+ Revised: 	2026/11/09 14:40:23.753444 )
[//]: # ( ---------------------------------------------------------------------- )

# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.0] - 2026-11-09

### Added

- avalio-lite now needs a licence key: enter it once and the app works offline for up to 30 days between checks. While it is locked you can still back up your data
- The app confirms its licence once a day in the background, warns a few days before it would lock, and shows the licence in Settings → Licence
- The app looks for a new version once a day and says so in a bar above the sheet, with **What's new** and **Hide**; Settings → Updates turns this off, and its **Check now** looks straight away
- **Update now** downloads the new version, checks its signature, and installs it (installed copy) or replaces the exe (portable copy), then opens the app again; the next start says which version it is now
- Every release file comes with a `.sig` signature, which the app checks before using a download

### Changed

- Messages on the Settings page stay above the cards, so a result from a card further down is seen without scrolling up
- The desktop tests tell the window it is the active one before each capture, so a window brought to the front during a run no longer changes the images
- The desktop tests type text from the app's own thread, so no typed character is lost, and wait for a closed Windows dialog to go instead of a fixed half second
- Desktop test baselines re-recorded at 125 % and 150 % for the licence, update and Settings screens

### Fixed

- A copy set up and updated in the same session now says "Updated to …" at its next start

## [0.5.0] - 2026-11-03

### Added

- ⋯ → Duplicate class, with or without its pupils and marks; the copy is named e.g. "1A (2)" and opens at once
- The sidebar folds to a narrow strip of icons with the « button or Ctrl+B, giving the sheet more width; the choice is remembered
- ‹ and › in a column's settings panel, and Alt+← / Alt+→, open the next or previous column's settings without closing the panel
- Ctrl+← and Ctrl+→ move a column left or right while its settings panel is open
- Desktop test baselines for 125 % and 150 % display scaling

### Changed

- The app's core state moved out of the UI code and was split by concern; no visible change
- Source reorganised by feature; no visible change
- Tests strengthened after mutation testing: every mutant in the grade calculation, columns, cells, migrations, file moves, PIN rules, languages and app state is caught
- The desktop tests open the window on the primary monitor, so the display scale they test no longer depends on where the mouse is; a test also guards the module dependency rules

### Fixed

- The sheet's menus no longer stay open, partly cut off, when the window is resized
- Escape now closes a column's settings panel after a click in it and from every one of its fields
- The window no longer opens partly off-screen on smaller or scaled displays: it opens centred on the current screen, sized to fit, and maximised when the screen is too small

## [0.4.0] - 2026-11-02

### Added

- A pupil who is in no class (they left their last class, or their only class was deleted) is marked "no class" in **⋯ → Add from another class**, with × to delete them for good after a confirmation
- Menus work from the keyboard: the first option is focused when one opens, arrows move, Enter picks, Esc closes; a column's settings panel focuses its title when it opens and closes with Esc
- Enter in the Repeat PIN field of Settings sets or changes the PIN, like the button
- Screen readers read calculated marks (and whether they are provisional)
- The log is set aside as `avalio-lite.log.1` once it passes 5 MB, when the app starts

### Changed

- Calculated marks are shown rounded half up (6.125 shows 6.13, as a rounded column does), and pass or fail by the mark shown: a final grade that shows as 5 out of 10 passes
- Scores, weights and maximums accept only plain numbers (digits with one decimal point or comma, and a minus sign); `1e1` or `+5` no longer count as numbers
- A test's weight goes up to 1000 %, a maximum up to 10 000
- Classes, and the pupils offered in **Add from another class**, are listed alphabetically ignoring accents and case ("Álvaro" with the A's, Ñ after N); a class's own pupils keep their order. Names in another case ("ÁLVARO", "álvaro"), or with accents stored as separate marks, are recognised as the same name in imports and backups
- Errors are told in plain words in the app's language, without the technical text behind them (which goes to the log), and show on the page that is open
- Percentages are written "50 %" in Galician

### Fixed

- Changing, setting or removing the PIN, or restoring a backup, deleted the previous database even when the new one then failed to open; the previous one is now put back, or, if another program holds the files, kept aside and named in the message
- A database file that was empty, cut short or not an avalio-lite database was set up as a new, empty one; it is now reported, and choosing such a file changes nothing
- A settings file that couldn't be read was overwritten; it is now kept as `config.toml.bad`
- Moving the logs could leave the log and the settings pointing at different folders when saving the settings failed; the log is now moved back (if even that fails, the log says where it is). Moving a large log no longer leaves the old `.1` log behind
- A move that failed part way left a partial copy behind; a copy is now checked before the original is deleted
- Adding classes from a backup failed when it had two pupils with the same name as one pupil already here, and could join two of its pupils into one
- A backup copy cut short (a full disk) was left next to the database
- Imported times of day became the date 1899-12-31, negative durations read "-1:-30:-05", and numbers worked out by Excel formulas kept their binary noise (7.299999999999999)
- A workbook that made the reader fail inside would have closed the app; it is now reported as a file that can't be read
- After deleting "Test 1", the next new test was called "Test 2" a second time
- A damaged plain database said "Wrong PIN"
- Deleting a class, column or pupil that was already gone reported success
- After restoring a backup or opening another database, an open column panel could attach to an unrelated column of the new file
- A name that was briefly empty or taken while being retyped left the field out of step with what was saved; leaving the field now puts the saved name back
- A new class or pupil name that was refused was cleared from the field
- Some errors and hints (Ctrl+L without a PIN, a failed theme change from the sheet) were shown on a page that wasn't open
- Galician messages for one class or pupil used the plural verb

## [0.3.0] - 2026-11-01

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
