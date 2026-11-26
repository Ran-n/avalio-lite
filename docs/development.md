[//]: # ( ---------------------------------------------------------------------- )
[//]: # (+ Authors: 	Ran# <ran.hash@proton.me> )
[//]: # (+ Created: 	2026/11/01 21:11:24.728640 )
[//]: # (+ Revised: 	2026/11/26 16:43:15.256275 )
[//]: # ( ---------------------------------------------------------------------- )

# Developing avalio-lite

[README](../README.md) · [User guide](user-guide.md) · **Development guide** · [Design decisions](design-decisions.md)

How to build, test and release avalio-lite. To report a bug or send a change, see [CONTRIBUTING.md](../CONTRIBUTING.md); for what the app does, the [user guide](user-guide.md).

**Contents:** [Building from source](#building-from-source) · [Working on it](#working-on-it) · [Tests](#tests) · [Layout](#layout) · [Translations and schema changes](#translations-and-schema-changes) · [Releases](#releases)

## Building from source

### Requirements

- **Windows** with the **MSVC C++ build tools**: Visual Studio Build Tools with the *Desktop development with C++* workload (`winget install Microsoft.VisualStudio.2022.BuildTools`, then add the workload in the Visual Studio Installer).
- **Rust** through [rustup](https://rustup.rs/) (`winget install Rustlang.Rustup`). The version is pinned in `rust-toolchain.toml`, and rustup installs it on the first build.
- **[Strawberry Perl](https://strawberryperl.com/)** (`winget install StrawberryPerl.StrawberryPerl`), because SQLCipher's OpenSSL is compiled from source. Git's bundled Perl lacks modules the build needs. Perl is only used at build time.
- **[Inno Setup 6](https://jrsoftware.org/isinfo.php)** (`winget install JRSoftware.InnoSetup`), only to build the installer.
- **A short checkout path:** OpenSSL's build fails on paths longer than Windows' 260-character limit.

`.gitattributes` keeps the golden files and migrations in LF; don't remove it, or the golden tests fail on a Windows checkout.

### Steps

Clone from any of the three forges. HTTPS needs no key; SSH needs a key added to that forge's account.

| Forge | HTTPS | SSH |
|---|---|---|
| GitHub | `https://github.com/Ran-n/avalio-lite.git` | `git@github.com:Ran-n/avalio-lite.git` |
| GitLab | `https://gitlab.com/Ran-n/avalio-lite.git` | `git@gitlab.com:Ran-n/avalio-lite.git` |
| Codeberg | `https://codeberg.org/ran-n/avalio-lite.git` | `git@codeberg.org:ran-n/avalio-lite.git` |

In PowerShell:

```powershell
git clone https://codeberg.org/ran-n/avalio-lite.git C:\src\avalio-lite   # or the GitHub / GitLab mirror
cd C:\src\avalio-lite
. .\scripts\use-perl.ps1   # puts Strawberry Perl first on PATH for this session
cargo build --release      # the first build compiles OpenSSL and takes a while
```

The app is then `target\release\avalio-lite.exe`: one file with everything in it, which you can run as is or copy anywhere (it is the same file as the portable release).

To build the installer as well, after the release build:

```powershell
.\packaging\windows\build-installer.ps1
```

It writes the setup to `dist\` under the name the release uses (see [Releases](#releases)), with the version from `Cargo.toml`. It finds Inno Setup 6 on PATH or in its usual folders, and says where to get it if it's missing. Run the setup to install the app as from a release. Releases are built and tested by CI, not on your machine; see [Releases](#releases).

To update later: `git pull`, then `cargo build --release` again.

## Working on it

```powershell
cargo run                  # debug build, keeps a console for log output
cargo clippy --all-targets -- -D warnings   # the lint gate; CI and release-check.ps1 run it too
cargo fmt
cargo run --example demo_data -- D:\somewhere\avalio-lite.db gl   # sample classes to try the app with (en or gl)
```

Clippy enforces the size limits (`clippy.toml`, `[lints.clippy]` in `Cargo.toml`): a function over 40 lines, more than 4 parameters, more than 3 bools in a struct, any bool parameter, or blocks nested deeper than 3, and a folder's module file is `<folder>.rs`, never `mod.rs`. An exception is `#[expect(clippy::…, reason = "…")]` beside the code, never a bare `#[allow]`, so the reason is written down and the compiler says when the exception is no longer needed.

Set `LOG_LEVEL` (`error`, `warn`, `info`, `debug`, `trace`) to change how much is logged; the default is `info`. Set `AVALIO_LITE_CONFIG_DIR` to keep the settings file somewhere other than `%APPDATA%\Breren\avalio-lite`, for example to try the app on scratch data. The first-run screen then suggests that folder for the data too.

## Tests

```powershell
cargo test                       # the default suite: logic, database, files, the UI headless
.\scripts\coverage.ps1           # the same under coverage; fails below the line-coverage gate
.\scripts\coverage.ps1 -Branches # the branch-coverage gate instead (nightly toolchain); release-check.ps1 runs both
cargo mutants --gitignore true --jobserver false --in-place --file src/lock.rs -- --test lock   # mutation testing, one module and its tests at a time
cargo test --test desktop -- --ignored --test-threads=1   # the real app in its real window
cargo test regression            # only the regression tests, one per fixed bug
```

- **Default suite** (`tests/`, one file per module, or a folder per big binary, see the next bullet): unit and property tests (proptest) for the logic; integration tests on real databases and folders; golden files for every saved format (`tests/golden/`); a step-by-step migration walk; the entry point and quit path; the real worker thread handing its results back through an event loop (`tests/worker.rs`); error logging; the version; the dependency rules (`tests/architecture.rs`, see D4 in the [design decisions](design-decisions.md)); and the UI (`tests/ui/`). The UI tests run the real window on Slint's headless backend, click and type into it through its controls' accessible labels, and answer dialogs with a fake.
- **Test folders**: a test binary too big for one file is a folder, `tests/<name>/`: `main.rs` is the binary's root (it pulls in `tests/common/` and declares one `mod` per feature), each feature is one module file (for example `tests/ui/lock.rs` or `tests/update/check.rs`), and the helpers the features share sit in helper modules of the same folder (`tests/ui/driver.rs`, `fixtures.rs`, `screens.rs` and `pupil_screens.rs` for the UI, `support.rs` elsewhere). The folders are `ui`, `update`, `spreadsheet`, `gradebook`, `db`, `backup`, `state`, `i18n` and `desktop`, and each is still one binary (`cargo test --test ui`). A helper used by one module lives in that module; one used by several lives in the folder's helper module; one that several binaries share lives in `tests/common/`. A new test goes in the module of its feature.
- **Desktop tests** (`tests/desktop/`) open real windows, so `cargo test` leaves them out. They start the app through `examples/app_driver/` on scratch data, click and type through Windows messages (the test posts clicks and keys; the driver posts typed text on the window's own thread, because winit drops a character that isn't already queued when its key press is handled), and check colours and positions pixel by pixel. They compare every screen in light and dark, at the normal and the minimum window size, and in Galician at the minimum size, against reviewed baselines in `tests/desktop/baselines/` (the README shows these images). The app shows the version the baselines were recorded with, whatever the release (`AVALIO_LITE_VERSION`, which only a debug build reads), so a version bump changes no screenshot.
  - The window stays hidden while they run: the driver cloaks it (DWM still draws it, so captures and posted clicks work), so it never shows on the screen, in the taskbar or in Alt+Tab, and never takes the foreground. Set `$env:AVALIO_LITE_SHOW_TEST_WINDOW = "1"` to watch a run. The one exception is `the_real_dialogs_open_and_close`: it runs the shipped exe and its real folder dialog, which the tests can't hide, for a few seconds.
  - Nothing personal reaches the screen: every test's scratch folder is under `%PUBLIC%\avalio-lite-tests\` (one `run-*` folder per test, deleted afterwards), never under the user profile, and the helper that makes it (`tests/common/scratch.rs`) fails if its own path names the Windows account or profile folder. The driver checks every `state` answer, every key and value at any depth, with the same matcher (`avalio_lite::platform::personal`) and refuses one that names the user before anything is captured, because baselines are committed and no text scanner reads pixels.
  - Baselines are per Windows display scale (`<screen>-<look>-<width>x<height>@<scale>.png`); this repo has them at 100 %, 125 % and 150 %. The whole window must fit on the primary monitor: 1280 × 800 at 150 % needs one at least 1200 physical pixels high, or the part below the screen is captured black. The driver opens the window on the primary monitor, wherever the mouse is, so the scale under test is the primary monitor's. The pixel checks measure in logical pixels, so they hold at any scale. At a fractional scale (125 %, 150 %) a line of text can snap one physical pixel up or down between runs, so there a snapshot allows that one-row shift; at 100 % it must match exactly. A capture counts as settled once two frames in a row differ by no more than a blinking caret. Each capture first tells the window it is the active one, so a window brought to the front during the run doesn't change the images; the user's foreground window is never touched.
  - At a scale with no baselines, or on a machine whose fonts render differently, the snapshots fail and print the command that records them. To add a scale: set it on the primary monitor (Settings → System → Display → Scale), run `$env:AVALIO_UPDATE_BASELINES = "1"; cargo test --test desktop -- --ignored --test-threads=1`, open and review every new image for clipped, wrapped or cut-off text (Galician at 1000 × 600 above all), then run the suite again without the variable.
  - After a deliberate change to the look, rewrite the baselines with `$env:AVALIO_UPDATE_BASELINES = "1"`. Open every changed image and check nothing is clipped, wrapped or cut off before committing.
  - A failure saves the screen and a marked diff in `target\desktop-failures\`. For a screen that never stops changing, it saves the last two frames and what still changes.
- **Golden files** change only when a format does: `$env:AVALIO_UPDATE_GOLDEN = "1"; cargo test --test golden` rewrites the current-format samples. After a baseline the README shows changes, `cargo test --test readme` fails until its SVG is written again with the same variable. Samples of older formats are never rewritten.
- **Coverage** needs `cargo install cargo-llvm-cov` and `rustup component add llvm-tools-preview`.
  - Raise the gates in `scripts\coverage.ps1` (`$MinLines`, and `$MinBranches` for `-Branches`) as coverage grows; never lower them.
  - Branch data needs the nightly toolchain (`rustup toolchain install nightly --component llvm-tools-preview`). Without it, `-Branches` fails with that hint instead of skipping the check. The line gate is measured with the pinned toolchain only, because which lines count as code (closing braces, lines ending in `?`) changes from one compiler to the next; `release-check.ps1` runs both.
  - The script lists the few lines left uncovered on purpose, with the reason for each: the real exe and its Windows dialogs on screen (a desktop test covers them instead), checks that can't fail, and double faults. Any other uncovered line needs a test.
  - If the coverage build kept stale code from an earlier build (a file then reports more executable lines than it has), the script says so, cleans it and measures again. `-Branches` builds in `target\llvm-cov-branches`, apart from `target\llvm-cov-target`, so one mode's test programs are never counted in the other's report.
  - The script turns off the nightly cargo's new build-folder layout (`__CARGO_TEMPORARY_BUILD_DIR_NEW_LAYOUT_OPT_OUT`): with it, rustc's command line is too long for Windows, the coverage tool instruments nothing and the report fails with "no coverage data found", or keeps describing an older build. After a toolchain change, delete both folders first and check the totals are of the current code.
- **Mutation testing** (`cargo install cargo-mutants`) changes the code one small edit at a time and checks that some test fails. Run it now and then on the logic modules (not the UI wiring: `src/app/`, `src/window/` and each feature's `ui`; `src/state/` is included), with `--in-place` so it reuses the already built `target\` instead of rebuilding OpenSSL in a copy. Don't edit sources while it runs. Every mutant it reports as missed needs a test, unless the mutant can't change behaviour; those are listed with the reason in `.cargo/mutants.toml`. A *timeout* is a mutant the tests caught by hanging (cargo-mutants exits with 3 then), not a missed one. The last runs, over `compute.rs`, `columns.rs`, `cells.rs`, `migrations.rs`, `storage.rs`, `lock`, `i18n.rs` and `src/state/` (505 mutants, about 13 hours), caught every one but the equivalent ones listed in `.cargo/mutants.toml`. A run over the lines changed in `src/state/` and `src/spreadsheet/` when they were split into smaller files (133 mutants, 28 of which don't compile) found 8 more without a test; all are caught now. Name every test program that covers the code with `-C=--test -C=<name>`: a mutant in `src/state/` can be caught only by `encryption`, for example.

## Layout

Why it is laid out like this: [design decisions](design-decisions.md).

```
src/
  gradebook/    classes, pupils, columns, cells (SQL) and grade calculation (compute.rs); `columns/` holds the settings each kind takes, the kinds' defaults and storage, and the order (`settings.rs`, `kinds.rs`, `order.rs`); `model/names.rs` the name rules; names new columns through `ColumnTitles`, so it never imports `i18n`; `ui` wires it to the window; `pictures.rs` makes, stores and reads pupils' pictures (D13), drawn by `ui/avatars.rs` and set in `ui/pupil_panel.rs`
  state/        the app's core: settings, the open database and its PIN (the file swap of a PIN change in `protection/swap.rs`), data moves, backups; no UI code
  app/          wires the features (setup and settings screens); `lifecycle.rs` is the process around the window (logging, `--version`, start and clean shutdown); where the window opens is `placement.rs`, with the sizes and fitting maths in `placement/fit.rs`, the Win32 screen query in `placement/work_area.rs` and what is kept for the next start in `placement/remember.rs`
  window/       the UI kit every feature UI uses: callback wrappers, shared handles, dialogs, status and error line, theme, language, and the worker that runs work off the UI thread (D7)
  backup/       .avalio backup files: export, open and check, add classes; `ui` wires the Backup card
  spreadsheet/  Excel export (writing, safe names, the columns tab in `export/settings.rs`) and import (cell text in `import/cell.rs`, read, plan, apply) of a class sheet; `ui` wires it to the window
  error.rs      the app's error type
  lock/         PIN rules and guess limiting; `ui` wires the lock screen
  licence/      the licence check through Clavren (clavren-client): server, product, trusted token keys, and what the licence screen says for each answer; `ui` wires the screen
  update/       the updater, one module per concern: which build this is and the asset names (target.rs), the forges' answers (release.rs), GitHub and GitLab and the one HTTP client (source.rs), the check and its schedule (check.rs), the release keys and signatures (signature.rs: minisign's format in signature/minisign.rs, the two hashes in signature/hashes.rs, the 0.10.0 format in signature/bridge.rs), the verified download (download.rs), replacing and relaunching the program (install.rs), the uninstall entry (registration.rs, windows.rs); `ui.rs` re-exports the public paths of `ui/`, one file per concern: the state (`updater.rs`), the callbacks (`wiring.rs`), the timers and checks (`schedule.rs`), the download and install (`installing.rs`) and what is shown (`view.rs`), which wire the banner and the Updates card
  bin/          the release tools: avalio-lite-sign (signs and checks release files) and avalio-lite-asset-name (prints an asset's name for the release workflow)
  shortcuts/    keyboard shortcuts: the actions, their keys and the rules for config.toml, with no Slint code; `ui/` wires them to the window and reads key events (`ui/key_events.rs`)
  i18n/         interface language: Rust-side texts by topic (`messages/` for the enums, `english/` and `galician/` for the texts, one file per topic, errors in `errors.rs`)
  pupil_view.rs  what one pupil is shown of a class (their grades, never the notes); `pupil_view/ui.rs` wires the list, the grades page and the PIN between them, `ui/pages.rs` fills the pages (D12)
  platform/     plumbing every feature uses
    config/     config.toml: data locations and view preferences (`preferences.rs`), read and written by `store.rs`, its folders in `place.rs`, the 0.10.0 settings move in `legacy.rs`
    db/         SQLite connection, copy/verify, schema migrations
    logging.rs  file logger whose folder can change at runtime
    storage.rs  safe cross-drive file moves
ui/             Slint UI with its own light and dark theme (theme.slint); `components/` the shared widgets, one file per family (buttons, fields, menu, navigation, icons, surfaces, focus); one folder per page (`sheet/`, `settings/`, `lock/`, `licence/`, `column-panel/`, `pupil-view/`) holding the page and its parts, with the folding sidebar in sidebar.slint and the update bar across the window's top in update-bar.slint; gradebook.slint holds the sheet's data and actions, shortcuts.slint the keyboard shortcuts, backup.slint the Backup card's actions
lang/           translations of the UI strings, lang/<code>/LC_MESSAGES/avalio-lite.po, bundled into the exe
assets/         logo (SVG source, generated .ico/.png; regenerate with cargo run --example render_logo -- <preview folder>); breren-logo.svg, Breren's original mark for the README footer, copied unchanged (it switches colours for light and dark by itself); screenshots/, the README's screenshots: each SVG holds a screen's light and dark baseline and shows the one for the reader's mode (GitLab and Codeberg strip `<picture>`), written by `tests/readme.rs`
examples/       demo_data: builds a sample database; app_driver/: runs the app for the desktop tests (main, startup, commands, releases, state); render_logo: the icon files
migrations/     numbered SQL schema files, embedded in the exe
packaging/      windows/: the Inno Setup script (avalio-lite.iss), build-installer.ps1 and test-installer.ps1
scripts/        release-check (tests before tagging), release (after CI), release-common (shared helpers), coverage gate, Perl setup
.github/        workflows/release.yml: the release build on a tag
tests/          integration tests, one file per module; the binaries too big for one file are folders (see Tests)
  ui/, update/, spreadsheet/, gradebook/, db/, backup/, state/, i18n/  one folder per test binary: `main.rs` and one module per feature
  common/       fixtures shared by the tests and the app driver; licence.rs is a fake Clavren in memory (tests and the driver start activated; the driver's `unlicensed` argument starts on the licence screen); http.rs a small local server for the download tests
  golden/       saved-format samples
  desktop/      desktop tests: `main.rs` and one module per feature, `fixtures.rs` and `screens.rs` shared; `baselines/` holds the reviewed images, which are also the README's screenshots
  desktop_support/  desktop-test harness: `App` (the driver client, `app.rs`, with its setup, input, locate, capture and page moves in sibling files), the Win32 window (`window.rs`), pixels, theme colours, the built binaries, and the real app's own dialogs (`real_app.rs`)
```

## Translations and schema changes

Any text added to the UI with `@tr("…")` needs an entry in every `lang/*/…/avalio-lite.po`, and any text built in Rust needs a translation in every `src/i18n/<language>/<topic>.rs`, where the topic is the one whose enum in `src/i18n/messages/<topic>.rs` holds the text (`sheet`, `data`, `spreadsheet`, `shortcuts`, `licence`, `update`, `app`). Error texts are in `src/i18n/<language>/errors.rs`. `tests/i18n/` (its `main.rs` and one module per topic in the same folder) fails until both are complete. The golden copies `tests/golden/messages-en.txt` and `messages-gl.txt` hold every message and error in both languages: after a deliberate change of wording, run `AVALIO_UPDATE_GOLDEN=1 cargo test --test i18n` and review the diff.

Schema changes go in a new `migrations/NNN_*.sql` file, added to the list in `src/platform/db/migrations.rs`. Shipped migrations are never edited. Migrations run with foreign keys off (a table rebuild would otherwise cascade-delete the rows pointing at it), each in its own transaction, which is committed only once `PRAGMA foreign_key_check` finds no dangling reference; the runner turns foreign keys back on afterwards, also when a migration fails. Before rewriting the golden samples for a new migration, copy `tests/golden/db-current.db` to `db-v<previous version>.db` and add an upgrade test for that sample in `tests/golden.rs`, so files users already have stay covered.

## Releases

CI builds every release on a Windows runner and makes a draft. The maintainer signs the exes on their own machine, from the tagged checkout, and publishes. The signing key never goes to CI, a forge or a file.

### Assets

Each release has these files on GitHub, GitLab and Codeberg. The names come only from `avalio-lite-asset-name` (`cargo run --release --bin avalio-lite-asset-name -- windows-x86_64 portable|setup [--bridge]`), and CI, `release.ps1` and the docs follow it.

| File | What it is |
|---|---|
| `avalio-lite-<version>-windows-x86_64-portable.exe` | The app in one file, for Windows 64-bit. It runs from any folder. |
| `avalio-lite-<version>-windows-x86_64-setup.exe` | The installer, per user: a Start-menu entry, an optional desktop icon, an uninstaller. |
| `avalio-lite-<version>-portable.exe` | The bridge: a byte-identical copy of the portable file under the name the 0.6.0 to 0.10.0 updaters look for. Kept until 2027-05-15. |
| `avalio-lite-<version>-setup.exe` | The bridge: a byte-identical copy of the setup under the name the same updaters look for. Kept until 2027-05-15. |

Next to each exe is `<file>.b3`, one line with the BLAKE3 hash, two spaces and the file name, and its signature: `<file>.minisig` (minisign's format, which the app checks before and after it downloads an update) for the two new names, and `<file>.sig` (the 0.10.0 format) for the two bridge names. That is twelve files: four exes, four `.b3`, two `.minisig` and two `.sig`. CI makes the exes and the `.b3` files, and the maintainer's script adds the signatures. Anyone can check a file by hand with `minisign -Vm <file> -P <key>`, using a key from the README.

- **Runner:** `windows-latest`. **Target:** `windows-x86_64`. CI builds with `AVALIO_LITE_RELEASE_TARGET=windows-x86_64`; a build without that variable is "from source" and never updates itself.
- **Bridge:** the 0.6.0 to 0.10.0 updaters need the bridge names and the old signature format, and `avalio-lite-sign` writes that format for them. After 2027-05-15, remove the bridge steps from `.github/workflows/release.yml` and `scripts/release.ps1`, and the bridge `[Run]` entry from `packaging/windows/avalio-lite.iss`.

### Installer and its test

```powershell
.\packaging\windows\build-installer.ps1 [-Binary target\release\avalio-lite.exe] [-OutputDirectory dist]
.\packaging\windows\test-installer.ps1 -Installer dist\<setup file> [-BuildDirectory target]
```

`build-installer.ps1` builds the setup with Inno Setup 6 (`packaging/windows/avalio-lite.iss`). The setup installs per user, into `%LOCALAPPDATA%\Programs\avalio-lite`, with no administrator rights. It closes a running copy before it replaces it. Windows' installed apps and the Start menu list it as **avalio lite** (`DISPLAY_NAME` in `src/about.rs`; a test ties the installer's `AppName` and `UninstallDisplayName` to it); the install folder and the exe keep the program name `avalio-lite`. An upgrade removes the `avalio-lite` shortcuts earlier setups made (`[InstallDelete]`). Uninstalling keeps the database, logs and settings.

`test-installer.ps1` tests the setup silently, in a scratch folder under the build directory. It installs, checks the install (the uninstall record's name and version, the two shortcuts, `avalio-lite.exe --version`), installs again over itself, uninstalls, and checks that nothing is left. It refuses to run while a real avalio-lite installation exists on the machine, because the test uses the same uninstall record and shortcuts. CI runs it on every release build. Run it locally only on a machine without a real installation.

### Signing keys

- **The signing key** signs every release. It lives in `~/.release-keys/avalio-lite-release-signing.key.gpg`, encrypted to the maintainer's GPG key (the fingerprint on the `GPG` line of `CONTRIBUTORS`), so `scripts/release.ps1` signs without asking for anything: `gpg --batch -q --pinentry-mode error -d <file> | avalio-lite-sign sign - <assets>`. A copy is in the maintainer's KeePass (`avalio-lite release key`), as its recovery copy. Another maintainer without the `.gpg` file pipes the key from their password manager instead: `keepassxc-cli show -s -a Password <db> "<entry>" | avalio-lite-sign sign - <assets>`. Never decrypt the key to disk, and never delete the `.gpg` file.
- **The backup key** is a second key, not a copy of the first. Its secret exists only in the maintainer's KeePass, in a database kept apart from the signing key's, and is used only if the signing key is lost or stolen. No file of it stays on any computer.
- The app trusts the keys in `BUILT_IN_KEYS` (`src/update/signature.rs`), in hex. The first entry is the signing key, which the 0.6.0 to 0.10.0 copies trust too, so it stays first. The second is the backup key's public half. A test checks both are real, distinct keys, and another that the README lists them as `avalio-lite-sign keys` prints them.
- A plain key file, such as one `keygen` just wrote, is wiped with `shred -u -z -n 3 <file>` (Git Bash) once it is safely stored, never with `rm`, `del` or the Recycle Bin.
- `avalio-lite-sign public -` prints a key's public half, in hex (what `BUILT_IN_KEYS` holds) and then in minisign's form; `avalio-lite-sign keys` prints the two built-in keys in minisign's form, signing key first, as the README publishes them (a test keeps the two equal). `avalio-lite-sign keygen <file>` makes a new key and never overwrites a file. `avalio-lite-sign sign - <assets>` signs (the key on standard input): a `.minisig` for each new name, the old `.sig` for each bridge name, and nothing unless every file can be signed. `verify <assets>` checks each in its own format.

If the release key is lost or stolen:

1. Sign the next release with the backup key.
2. In that release, replace `BUILT_IN_KEYS` with the backup key and a newly generated key, and store the new key's backup in a separate database.
3. Ship that release at once.

Copies that don't trust the backup key can't check that release, so their users install it by hand once.

### Steps

1. Run `.\scripts\release-check.ps1` on the commit to release. It runs the default suite under the line and branch coverage gates, then the desktop tests, which open the real window.
2. Bump the version in `Cargo.toml` (the level comes from the changes), move the `[Unreleased]` notes in `CHANGELOG.md` under `## [X.Y.Z] - date`, and commit. Tag the commit `vX.Y.Z`, and push the commit and the tag to `github`, `gitlab` and `codeberg`.
3. CI (`.github/workflows/release.yml`) checks that the tag is the version in `Cargo.toml`, builds the exe and the setup, tests the setup, makes the bridge copies, writes and checks the `.b3` files, attests the build of every file, and creates a **draft** GitHub release with the eight files and the version's CHANGELOG section as its notes. Look over the draft on GitHub.
4. Run `.\scripts\release.ps1` in the tagged checkout. It downloads the draft's files and checks the `.b3` files and the build attestations of the four exes. Then it signs the four exes with the signing key, which gpg decrypts from `~/.release-keys` straight into `avalio-lite-sign` (nothing is asked for), verifies the signatures, uploads the two `.minisig` and two `.sig` files to the draft, creates the GitLab and Codeberg releases with all twelve files (if Codeberg refuses a file because the account's quota is full, it deletes this repository's oldest release files there, one at a time and only as many as the rest needs, never the new release's, and adds a line to each stripped release saying GitHub has every file; if even all of them wouldn't make room it deletes nothing and stops: `scripts/codeberg-quota.ps1`, tested by `tests/release_scripts.rs`), publishes the GitHub draft last, and checks that each forge lists every file at its size. `-DryRun` runs the read-only steps and prints the rest without doing it.

The script needs `gh` and `glab`, both signed in; `b3sum` 1.8.5 (`cargo install b3sum --version 1.8.5 --locked`); `gpg` holding the maintainer's GPG key with no passphrase prompt, and Git Bash (the decrypt runs in a Git Bash pipe); and the environment variable `CODEBERG_TOKEN`, a Codeberg token that can create releases. It stops at the first failed check, and signs nothing that failed one. It doesn't roll back: if a step fails after a forge was changed, delete that forge's release by hand before running the script again.
