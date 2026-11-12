[//]: # ( ---------------------------------------------------------------------- )
[//]: # (+ Authors: 	Ran# <ran.hash@proton.me> )
[//]: # (+ Created: 	2026/11/01 21:11:24.728640 )
[//]: # (+ Revised: 	2026/11/11 17:10:57.552253 )
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

`.gitattributes` keeps the golden files and migrations in LF; don't remove it, or the golden tests fail on a Windows checkout.

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
cargo mutants --gitignore true --jobserver false --in-place --file src/lock/mod.rs -- --test lock   # mutation testing, one module and its tests at a time
cargo test --test desktop -- --ignored --test-threads=1   # the real app in its real window
cargo test regression            # only the regression tests, one per fixed bug
```

- **Default suite** (`tests/`, one file per module): unit and property tests (proptest) for the logic; integration tests on real databases and folders; golden files for every saved format (`tests/golden/`); a step-by-step migration walk; the entry point and quit path; error logging; the version; the dependency rules (`tests/architecture.rs`, see D4 in the [design decisions](design-decisions.md)); and the UI (`tests/ui.rs`). The UI tests run the real window on Slint's headless backend, click and type into it through its controls' accessible labels, and answer dialogs with a fake.
- **Desktop tests** (`tests/desktop.rs`) open real windows, so `cargo test` leaves them out. They start the app through `examples/app_driver.rs` on scratch data, click and type through Windows messages (the test posts clicks and keys; the driver posts typed text on the window's own thread, because winit drops a character that isn't already queued when its key press is handled), and check colours and positions pixel by pixel. They compare every screen in light and dark, at the normal and the minimum window size, and in Galician at the minimum size, against reviewed baselines in `tests/desktop/baselines/` (the README screenshots are these images).
  - Baselines are per Windows display scale (`<screen>-<look>-<width>x<height>@<scale>.png`); this repo has them at 100 %, 125 % and 150 %. The whole window must fit on the primary monitor: 1280 × 800 at 150 % needs one at least 1200 physical pixels high, or the part below the screen is captured black. The driver opens the window on the primary monitor, wherever the mouse is, so the scale under test is the primary monitor's. The pixel checks measure in logical pixels, so they hold at any scale. At a fractional scale (125 %, 150 %) a line of text can snap one physical pixel up or down between runs, so there a snapshot allows that one-row shift; at 100 % it must match exactly. A capture counts as settled once two frames in a row differ by no more than a blinking caret. Each capture first tells the window it is the active one, so a window brought to the front during the run doesn't change the images; the user's foreground window is never touched.
  - At a scale with no baselines, or on a machine whose fonts render differently, the snapshots fail and print the command that records them. To add a scale: set it on the primary monitor (Settings → System → Display → Scale), run `$env:AVALIO_UPDATE_BASELINES = "1"; cargo test --test desktop -- --ignored --test-threads=1`, open and review every new image for clipped, wrapped or cut-off text (Galician at 1000 × 600 above all), then run the suite again without the variable.
  - After a deliberate change to the look, rewrite the baselines with `$env:AVALIO_UPDATE_BASELINES = "1"`. Open every changed image and check nothing is clipped, wrapped or cut off before committing.
  - A failure saves the screen and a marked diff in `target\desktop-failures\`. For a screen that never stops changing, it saves the last two frames and what still changes.
- **Golden files** change only when a format does: `$env:AVALIO_UPDATE_GOLDEN = "1"; cargo test --test golden` rewrites the current-format samples. Samples of older formats are never rewritten.
- **Coverage** needs `cargo install cargo-llvm-cov` and `rustup component add llvm-tools-preview`.
  - Raise the gates in `scripts\coverage.ps1` (`$MinLines`, and `$MinBranches` for `-Branches`) as coverage grows; never lower them.
  - Branch data needs the nightly toolchain (`rustup toolchain install nightly --component llvm-tools-preview`). Without it, `-Branches` fails with that hint instead of skipping the check.
  - The script lists the few lines left uncovered on purpose, with the reason for each: the real exe and its Windows dialogs (a desktop test covers them instead), and double faults. Any other uncovered line needs a test.
  - If the coverage build kept stale code from an earlier build (a file then reports more executable lines than it has), the script says so, cleans it and measures again.
- **Mutation testing** (`cargo install cargo-mutants`) changes the code one small edit at a time and checks that some test fails. Run it now and then on the logic modules (not the UI wiring: `src/app/`, `src/window/` and each feature's `ui`; `src/state/` is included), with `--in-place` so it reuses the already built `target\` instead of rebuilding OpenSSL in a copy. Don't edit sources while it runs. Every mutant it reports as missed needs a test, unless the mutant can't change behaviour; those are listed with the reason in `.cargo/mutants.toml`. A *timeout* is a mutant the tests caught by hanging (cargo-mutants exits with 3 then), not a missed one. The last runs, over `compute.rs`, `columns.rs`, `cells.rs`, `migrations.rs`, `storage.rs`, `lock`, `i18n/mod.rs` and `src/state/` (505 mutants, about 13 hours), caught every one but the equivalent ones listed in `.cargo/mutants.toml`.

## Layout

Why it is laid out like this: [design decisions](design-decisions.md).

```
src/
  gradebook/    classes, pupils, columns, cells (SQL) and grade calculation (compute.rs); names new columns through `ColumnTitles`, so it never imports `i18n`; `ui` wires it to the window
  state/        the app's core: settings, the open database and its PIN, data moves, backups; no UI code
  app/          starts the app and wires the features (setup and settings screens; where the window opens, placement.rs)
  window/       the UI kit every feature UI uses: callback wrappers, shared handles, dialogs, status and error line, theme, language, and the worker that runs work off the UI thread (D7)
  backup/       .avalio backup files: export, open and check, add classes; `ui` wires the Backup card
  spreadsheet/  Excel export (writing, safe names) and import (read, plan, apply) of a class sheet; `ui` wires it to the window
  error.rs      the app's error type
  lock/         PIN rules and guess limiting; `ui` wires the lock screen
  licence/      the licence check through Clavren (clavren-client): server, product, trusted token keys, and what the licence screen says for each answer; `ui` wires the screen
  update/       the updater: the latest GitHub release (source.rs), release signatures (signature.rs), downloads (download.rs), which file each copy needs and the portable swap (install.rs), the Windows side (windows.rs: registry, starting the installer); `ui` wires the update bar and the Updates card
  shortcuts/    keyboard shortcuts: the actions, their keys and the rules for config.toml, with no Slint code; `ui/` wires them to the window and reads key events (`ui/key_events.rs`)
  i18n/         interface language: one file per language (`english.rs`, `galician.rs`) for Rust-side texts and error messages
  platform/     plumbing every feature uses
    config.rs   config.toml: data locations and view preferences
    db/         SQLite connection, copy/verify, schema migrations
    logging.rs  file logger whose folder can change at runtime
    storage.rs  safe cross-drive file moves
ui/             Slint UI with its own light and dark theme (theme.slint); `components/` the shared widgets, one file per family (buttons, fields, menu, navigation, icons, surfaces, focus); one folder per page (`sheet/`, `settings/`, `lock/`, `licence/`, `column-panel/`) holding the page and its parts, with the folding sidebar in sidebar.slint; gradebook.slint holds the sheet's data and actions, shortcuts.slint the keyboard shortcuts, backup.slint the Backup card's actions
lang/           translations of the UI strings, lang/<code>/LC_MESSAGES/avalio-lite.po, bundled into the exe
assets/         logo (SVG source, generated .ico/.png; regenerate with cargo run --example render_logo -- <preview folder>)
examples/       demo_data: builds a sample database; app_driver: runs the app for the desktop tests; render_logo: the icon files
migrations/     numbered SQL schema files, embedded in the exe
installer/      Inno Setup script
scripts/        release, coverage gate, Perl setup
tests/          integration tests, one file per module
  common/       fixtures shared by the tests and the app driver; licence.rs is a fake Clavren in memory (tests and the driver start activated; the driver's `unlicensed` argument starts on the licence screen); http.rs a small local server for the download tests
  golden/       saved-format samples
  desktop/      desktop-test baselines
  desktop_support/  desktop-test harness: Win32 window, pixels, driver client
```

## Translations and schema changes

Any text added to the UI with `@tr("…")` needs an entry in every `lang/*/…/avalio-lite.po`, and any text built in Rust needs a translation in every `src/i18n/<language>.rs`. `tests/i18n.rs` fails until both are complete.

Schema changes go in a new `migrations/NNN_*.sql` file, added to the list in `src/platform/db/migrations.rs`. Shipped migrations are never edited. Migrations run with foreign keys off (a table rebuild would otherwise cascade-delete the rows pointing at it), each in its own transaction, which is committed only once `PRAGMA foreign_key_check` finds no dangling reference; the runner turns foreign keys back on afterwards, also when a migration fails. Before rewriting the golden samples for a new migration, copy `tests/golden/db-current.db` to `db-v<previous version>.db` and add an upgrade test for that sample in `tests/golden.rs`, so files users already have stay covered.

## Release

```powershell
.\scripts\release.ps1
```

The script runs the default suite under its coverage gate and the desktop tests, builds the optimized exe, writes both versioned files to `dist\`, and signs each one (a `.sig` next to it). The version comes from `Cargo.toml`. Then tag the release commit `vX.Y.Z` and publish a release on GitHub, GitLab and Codeberg with all four files and the version's CHANGELOG section as notes.

### Release signing

The in-app updater only uses a download whose `.sig` checks out against the public key built into the app (`PUBLIC_KEY` in `src/update/mod.rs`: `8b8n9Uqg_5-ReHLUbp5xyfL6HwdVgYNXiJQdP0Uex5o`). A `.sig` is one line, `<signature>  <file name>`: an Ed25519 signature (base64url) of `avalio-lite-release-v1`, the file name and the file's BLAKE3 hash in hex, one per line.

- The private key is a file outside the repository: `..\secrets\avalio-lite-release.key`, or the path in `AVALIO_RELEASE_KEY`. `release.ps1` stops if it is missing. Keep a second copy somewhere safe, off this computer: without it, no copy of the app can update itself again.
- `cargo run --example release_sign -- sign <key file> <file>...` signs files by hand (each `.sig` goes next to its file), `verify <file>` checks one against the built-in key (or `--public-key <key>`), and `new-key <key file>` makes a new key. After a new key, update `PUBLIC_KEY`; copies built with the old key won't accept files signed with the new one, so their users must install by hand once.
- The updater finds its files by name: a release must keep `avalio-lite-<version>-setup.exe`, `avalio-lite-<version>-portable.exe` and their `.sig` files, under the tag `v<version>`. It reads GitHub only.


