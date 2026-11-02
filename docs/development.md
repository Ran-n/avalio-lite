[//]: # ( ---------------------------------------------------------------------- )
[//]: # (+ Authors: 	Ran# <ran.hash@proton.me> )
[//]: # (+ Created: 	2026/11/01 21:11:24.728640 )
[//]: # (+ Revised: 	2026/11/01 21:21:23.200657 )
[//]: # ( ---------------------------------------------------------------------- )

# Developing avalio-lite

How to build, test and release avalio-lite. To report a bug or send a change, see [CONTRIBUTING.md](../CONTRIBUTING.md); for what the app does, the [user guide](user-guide.md).

## Building from source

### Requirements

- **Windows** with the **MSVC C++ build tools**: Visual Studio Build Tools with the *Desktop development with C++* workload (`winget install Microsoft.VisualStudio.2022.BuildTools`, then add the workload in the Visual Studio Installer).
- **Rust** through [rustup](https://rustup.rs/) (`winget install Rustlang.Rustup`). The version is pinned in `rust-toolchain.toml`, and rustup installs it on the first build.
- **[Strawberry Perl](https://strawberryperl.com/)** (`winget install StrawberryPerl.StrawberryPerl`), because SQLCipher's OpenSSL is compiled from source. Git's bundled Perl lacks modules the build needs. Perl is only used at build time.
- **[Inno Setup 6](https://jrsoftware.org/isinfo.php)** (`winget install JRSoftware.InnoSetup`), only to build the installer.
- **A short checkout path:** OpenSSL's build fails on paths longer than Windows' 260-character limit.

### Steps

In PowerShell:

```powershell
git clone https://codeberg.org/ran-n/avalio-lite.git C:\src\avalio-lite   # or the GitHub / GitLab mirror
cd C:\src\avalio-lite
. .\scripts\use-perl.ps1   # puts Strawberry Perl first on PATH for this session
cargo build --release      # the first build compiles OpenSSL and takes a while
```

The app is then `target\release\avalio-lite.exe`: one file with everything in it, which you can run as is or copy anywhere (it is the same file as the portable release).

To build the installer as well, run Inno Setup on that exe, with the version from `Cargo.toml`:

```powershell
$version = (cargo metadata --no-deps --format-version 1 | ConvertFrom-Json).packages[0].version
& "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe" "/DAppVersion=$version" "/DSourceExe=$PWD\target\release\avalio-lite.exe" installer\avalio-lite.iss
```

ISCC.exe may be under `Program Files (x86)\Inno Setup 6` instead, depending on how Inno Setup was installed. The source exe needs a full path, because Inno Setup reads relative paths from the `installer\` folder. The installer is written to `dist\avalio-lite-<version>-setup.exe`; run it to install the app as from a release. To build both files the way releases are made, with every test first, use `.\scripts\release.ps1` (see [Release](#release)).

To update later: `git pull`, then `cargo build --release` again.

## Working on it

```powershell
cargo run                  # debug build, keeps a console for log output
cargo clippy --all-targets -- -D warnings
cargo run --example demo_data -- D:\somewhere\avalio-lite.db gl   # sample classes to try the app with (en or gl)
```

Set `LOG_LEVEL` (`error`, `warn`, `info`, `debug`, `trace`) to change how much is logged; the default is `info`. Set `AVALIO_LITE_CONFIG_DIR` to keep the settings file somewhere other than `%APPDATA%\avalio-lite`, for example to try the app on scratch data. The first-run screen then suggests that folder for the data too.

## Tests

```powershell
cargo test                       # the default suite: logic, database, files, the UI headless
.\scripts\coverage.ps1           # the same under coverage; fails below the line-coverage gate
.\scripts\coverage.ps1 -Branches # plus the branch-coverage gate (nightly toolchain); what release.ps1 runs
cargo mutants --gitignore true --jobserver false --in-place --file src/lock.rs -- --test lock   # mutation testing, one module and its tests at a time
cargo test --test desktop -- --ignored --test-threads=1   # the real app in its real window
cargo test regression            # only the regression tests, one per fixed bug
```

- **Default suite** (`tests/`, one file per module): unit and property tests (proptest) for the logic; integration tests on real databases and folders; golden files for every saved format (`tests/golden/`); a step-by-step migration walk; the entry point and quit path; error logging; the version; and the UI (`tests/ui.rs`). The UI tests run the real window on Slint's headless backend, click and type into it through its controls' accessible labels, and answer dialogs with a fake.
- **Desktop tests** (`tests/desktop.rs`) open real windows, so `cargo test` leaves them out. They start the app through `examples/app_driver.rs` on scratch data, click and type through Windows messages, and check colours and positions pixel by pixel. They compare every screen in light and dark, at the normal and the minimum window size, and in Galician at the minimum size, against reviewed baselines in `tests/desktop/baselines/` (the README screenshots are these images).
  - Baselines are per Windows display scale (`<screen>-<look>@<scale>.png`). The pixel checks measure in logical pixels, so they hold at any scale. At a scale with no baselines, or on a machine whose fonts render differently, the snapshots fail and print the command that records them; record them the same way, at that scale.
  - After a deliberate change to the look, rewrite the baselines with `$env:AVALIO_UPDATE_BASELINES = "1"`. Open every changed image and check nothing is clipped, wrapped or cut off before committing.
  - A failure saves the screen and a marked diff in `target\desktop-failures\`. For a screen that never stops changing, it saves the last two frames and what still changes.
- **Golden files** change only when a format does: `$env:AVALIO_UPDATE_GOLDEN = "1"; cargo test --test golden` rewrites the current-format samples. Samples of older formats are never rewritten.
- **Coverage** needs `cargo install cargo-llvm-cov` and `rustup component add llvm-tools-preview`.
  - Raise the gates in `scripts\coverage.ps1` (`$MinLines`, and `$MinBranches` for `-Branches`) as coverage grows; never lower them.
  - Branch data needs the nightly toolchain (`rustup toolchain install nightly --component llvm-tools-preview`). Without it, `-Branches` fails with that hint instead of skipping the check.
  - The script lists the few lines left uncovered on purpose, with the reason for each: the real exe and its Windows dialogs (a desktop test covers them instead), and double faults. Any other uncovered line needs a test.
  - If the coverage build kept stale code from an earlier build (a file then reports more executable lines than it has), the script says so, cleans it and measures again.
- **Mutation testing** (`cargo install cargo-mutants`) changes the code one small edit at a time and checks that some test fails. Run it now and then on the logic modules (not `src/app/`), with `--in-place` so it reuses the already built `target\` instead of rebuilding OpenSSL in a copy. Don't edit sources while it runs. Every mutant it reports as missed needs a test, unless the mutant can't change behaviour; those are listed with the reason in `.cargo/mutants.toml`.

## Layout

```
src/
  gradebook/    classes, pupils, columns, cells (SQL) and grade calculation (compute.rs); no UI code
  app/          window startup, UI callbacks, lock screen, theme and language switching, the Backup card (backup.rs)
    state/      data locations, the open database and its PIN (protection.rs)
    gradebook/  connects the sheet UI to src/gradebook
  db/           SQLite connection, copy/verify, schema migrations
  backup.rs     .avalio backup files
  spreadsheet/  Excel export and import of a class sheet
  config.rs     config.toml: data locations and view preferences
  error.rs      the app's error type
  lock.rs       PIN rules and guess limiting
  logging.rs    file logger whose folder can change at runtime
  shortcuts.rs  keyboard shortcuts: the actions, their keys and the rules for config.toml
  storage.rs    safe cross-drive file moves
  i18n/         interface language: Rust-side texts and error messages per language
ui/             Slint UI with its own light and dark theme (theme.slint); gradebook.slint holds the sheet's data and actions, shortcuts.slint the keyboard shortcuts, backup.slint the Backup card's actions
lang/           translations of the UI strings, lang/<code>/LC_MESSAGES/avalio-lite.po, bundled into the exe
assets/         logo (SVG source, generated .ico/.png; regenerate with cargo run --example render_logo -- <preview folder>)
examples/       demo_data: builds a sample database; app_driver: runs the app for the desktop tests; render_logo: the icon files
migrations/     numbered SQL schema files, embedded in the exe
installer/      Inno Setup script
scripts/        release, coverage gate, Perl setup
tests/          integration tests, one file per module
  common/       fixtures shared by the tests and the app driver
  golden/       saved-format samples
  desktop/      desktop-test baselines
  desktop_support/  desktop-test harness: Win32 window, pixels, driver client
```

## Translations and schema changes

Any text added to the UI with `@tr("…")` needs an entry in every `lang/*/…/avalio-lite.po`, and any text built in Rust needs a translation in `src/i18n/`. `tests/i18n.rs` fails until both are complete.

Schema changes go in a new `migrations/NNN_*.sql` file, added to the list in `src/db/migrations.rs`. Shipped migrations are never edited. Before rewriting the golden samples for a new migration, copy `tests/golden/db-current.db` to `db-v<previous version>.db` and add an upgrade test for that sample in `tests/golden.rs`, so files users already have stay covered.

## Release

```powershell
.\scripts\release.ps1
```

The script runs the default suite under its coverage gate and the desktop tests, builds the optimized exe, and writes both versioned files to `dist\`. The version comes from `Cargo.toml`. Then tag the release commit `vX.Y.Z` and publish a release on GitHub, GitLab and Codeberg with both files and the version's CHANGELOG section as notes.

