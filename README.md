[//]: # ( ---------------------------------------------------------------------- )
[//]: # (+ Authors: 	Ran# <ran.hash@proton.me> )
[//]: # (+ Created: 	2026/10/27 19:34:10.260648 )
[//]: # (+ Revised: 	2026/10/29 01:00:46.961481 )
[//]: # ( ---------------------------------------------------------------------- )

# avalio-lite

Lightweight, local edition of Avalio — a teacher-focused student evaluation app.

A single native Windows app: no server, no accounts, no internet. Each class gets its own spreadsheet-style sheet of pupils and marks, all kept in one SQLite file in a folder you choose.

## Using it

- **Classes** are listed on the left; type a name in **+ New class** and press Enter.
- **Pupils** are the rows of a class's sheet: type a name in the last row (or click **+ Add pupil**) and press Enter. A pupil can be in several classes (**⋯ → Add from another class**); renaming them renames them everywhere, and removing them (× next to the name) from one class only deletes their marks in that class.
- **Columns** are added with **+ Add column**. Click a column's header to open its settings on the right: title, weight, maximum, bonus, rounding, move left / right and delete. The header itself shows the title and small tags such as `50%` and `/10`.
- Under the class name: the number of pupils, the class average of the final grade, and how many pass and fail.

| Column | Holds |
|---|---|
| Test | A score you type, out of the test's maximum (e.g. 10 or 5), counting a set % towards the final grade. Tick **Bonus** for extra credit: its % counts on top of the normal tests' 100 % |
| Info | Any text, e.g. notes |
| Final grade | Sum of each test's weight × score / maximum (bonus tests included), shown out of its own maximum and never above it. Ungraded tests count as 0; blank until a test is marked. While the normal tests' weights don't add up to 100 %, a warning shows under the class name and final grades turn grey with a "!" |
| Average | Mean of the graded normal tests (each as score / maximum), shown out of its own maximum; bonus tests are left out |
| Rounded | Another numeric column rounded to 0–3 decimals (half up: 7.45 → 7.5) |

**⋯ → Hide weights & scales** hides the tags in the column headers; the choice is remembered.

**Light and dark:** the moon / sun button at the bottom of the sidebar (or **Settings → Appearance**). The app follows the Windows setting until you pick one.

The app speaks **Galician** and **English**. It follows the Windows language until you pick one in **Settings → Language**; in Galician, calculated marks use a decimal comma (8,5).

**Keyboard:** Tab moves between fields and buttons, Enter or Space presses the focused button, and Ctrl+L locks.

Everything saves as you type. Scores accept `7.5` or `7,5`; a score that isn't a number within the test's range is shaded red and left out of the calculations. Calculated marks show as green pills at half their maximum or above (5 out of 10) and rose below it; failing test scores are shown in rose too.

## Install

Each release ships two files; use either:

| File | What it is |
|---|---|
| `avalio-lite-<version>-setup.exe` | Installer: Start-menu entry, optional desktop icon, uninstall from Windows Settings. Installs for the current user without an administrator prompt. |
| `avalio-lite-<version>-portable.exe` | The whole app in one file. Put it anywhere (including a USB stick) and run it. |

The files are not code-signed. If Windows shows "Windows protected your PC" after a browser download, choose **More info → Run anyway**, or right-click the file → **Properties** → tick **Unblock**.

## Your data

On first launch the app asks where to keep:

- **the database** — `avalio-lite.db`, everything you enter
- **the logs** — `avalio-lite.log`, for diagnosing problems

Both default to `Documents\avalio-lite`. Either can be moved later from **Settings**; the app copies, verifies, switches over, and only then deletes the original. **Open another database…** switches to an existing file (a backup, or one from another PC).

The only thing stored in a fixed place is `%APPDATA%\avalio-lite\config.toml`, which holds those two locations. Uninstalling never deletes your database or logs.

To back up, copy `avalio-lite.db` while the app is closed.

### PIN and encryption

In **Settings → PIN and encryption** you can set a PIN of 4 to 128 characters: digits, letters, spaces or symbols. The database file is then encrypted with it ([SQLCipher](https://www.zetetic.net/sqlcipher/): every page of the file is encrypted with AES-256 and checked for tampering), so the file can't be read without the PIN. The app asks for the PIN when it opens: type it, or click it on the on-screen keys (**ABC** switches them to letters, with ⇧ for capitals and a space bar; **#+=** to two pages of symbols, switched with **1/2**; **123** goes back to numbers); **Ctrl+L** or **Lock** in the sidebar closes the database and shows the lock screen. After five wrong PINs, each further try waits 30 seconds.

How strong the PIN is, is your call. The 30-second wait only applies inside the app: someone with a copy of the file can try guesses as fast as their computer allows (each guess is deliberately slow, but not slow enough for short PINs). A 4-digit PIN can be found in minutes, 8 digits in about a day on a gaming graphics card; a password of 12 or more characters with letters, or four random words, would take far longer than anyone can wait.

While the app is open, the PIN is held in memory only as long as the database is unlocked, and wiped (overwritten with zeros) when it locks, closes or the PIN changes; SQLCipher wipes its own copies too. The PIN is never written to the settings file or the log.

**If you forget the PIN, the data can't be recovered** — by anyone. Keep the PIN somewhere safe. **Forgot the PIN?** on the lock screen leaves it for the first-run screen, where you can open another database or start a new one in a different folder; the locked file stays where it is, untouched, and **Back to the lock screen** returns to it (a running 30-second wait keeps running). Removing the PIN (it asks for the current one) turns the encryption off again.

## Development

Requirements: Rust with the MSVC toolchain (the version is pinned in `rust-toolchain.toml`; rustup installs it on first build); [Strawberry Perl](https://strawberryperl.com/) (`winget install StrawberryPerl.StrawberryPerl`), because SQLCipher's OpenSSL is compiled from source — Git's bundled Perl lacks modules it needs, and Perl is only used at build time; and, for the installer, [Inno Setup 6](https://jrsoftware.org/isinfo.php). Keep the checkout in a short path: OpenSSL's build fails on paths over Windows' 260-character limit.

```powershell
cargo run                  # debug build, keeps a console for log output
cargo clippy --all-targets -- -D warnings
cargo run --example demo_data -- D:\somewhere\avalio-lite.db gl   # sample classes to try the app with (en or gl)
```

Set `LOG_LEVEL` (`error`, `warn`, `info`, `debug`, `trace`) to change log verbosity; the default is `info`. Set `AVALIO_LITE_CONFIG_DIR` to keep the settings file somewhere other than `%APPDATA%\avalio-lite`, e.g. to try the app on scratch data; the first-run screen then suggests that folder for the data too.

### Tests

```powershell
cargo test                       # the default suite: logic, database, files, the UI headless
.\scripts\coverage.ps1           # the same under coverage; fails below the line-coverage gate
.\scripts\coverage.ps1 -Branches # plus branch coverage (nightly toolchain)
cargo test --test desktop -- --ignored --test-threads=1   # the real app in its real window
cargo test regression            # only the regression tests, one per fixed bug
```

- **Default suite** (`tests/`, one file per module): unit and property tests (proptest) for the logic; integration tests on real databases and folders; golden files for every saved format (`tests/golden/`); a step-by-step migration walk; the entry point and quit path; error logging; the version; and the UI (`tests/ui.rs`), the real window on Slint's headless backend, clicked and typed into through its controls' accessible labels, with dialogs answered by a fake.
- **Desktop tests** (`tests/desktop.rs`) open real windows, so they are left out of `cargo test`. They start the app through `examples/app_driver.rs` on scratch data, click and type through Windows messages, check colours and positions pixel by pixel, and compare every screen in light and dark, at the normal and the minimum window size, and in Galician at the minimum size, with reviewed baselines in `tests/desktop/baselines/`. After a deliberate change to the look, rewrite them with `$env:AVALIO_UPDATE_BASELINES = "1"`, open every changed image and check nothing is clipped, wrapped or cut off before committing; a failure saves the screen and a marked diff in `target\desktop-failures\` (for a screen that never stops changing, its last two frames and what still changes).
- **Golden files** change only with a format: `$env:AVALIO_UPDATE_GOLDEN = "1"; cargo test --test golden` rewrites the current-format samples. Samples of older formats are never rewritten.
- **Coverage** needs `cargo install cargo-llvm-cov` and `rustup component add llvm-tools-preview`. Raise the gate in `scripts\coverage.ps1` as coverage grows; never lower it. If the coverage build kept stale code from an earlier build (a file then reports more executable lines than it has), the script says so, cleans it and measures again.

### Layout

```
src/
  gradebook/    classes, pupils, columns, cells (SQL) and grade calculation (compute.rs); no UI code
  app/          window startup, UI callbacks, lock screen, theme and language switching
    state/      data locations, the open database and its PIN (protection.rs)
    gradebook/  connects the sheet UI to src/gradebook
  db/           SQLite connection, copy/verify, schema migrations
  config.rs     config.toml: data locations and view preferences
  error.rs      the app's error type
  lock.rs       PIN rules and guess limiting
  logging.rs    file logger whose folder can change at runtime
  storage.rs    safe cross-drive file moves
  i18n/         interface language: Rust-side texts and error messages per language
ui/             Slint UI with its own light and dark theme (theme.slint); gradebook.slint holds the sheet's data and actions
lang/           translations of the UI strings, lang/<code>/LC_MESSAGES/avalio-lite.po, bundled into the exe
examples/       demo_data: builds a sample database; app_driver: runs the app for the desktop tests
migrations/     numbered SQL schema files, embedded in the exe
installer/      Inno Setup script
scripts/        release, coverage gate, Perl setup
tests/          integration tests, one file per module
  common/       fixtures shared by the tests and the app driver
  golden/       saved-format samples
  desktop/      desktop-test baselines
  desktop_support/  desktop-test harness: Win32 window, pixels, driver client
```

Any text added to the UI with `@tr("…")` needs an entry in every `lang/*/…/avalio-lite.po`, and any text built in Rust needs a translation in `src/i18n/`; `tests/i18n.rs` fails until both are complete.

Schema changes go in a new `migrations/NNN_*.sql` file added to the list in `src/db/migrations.rs`; shipped migrations are never edited. Before rewriting the golden samples for it, copy `tests/golden/db-current.db` to `db-v<previous version>.db` and add an upgrade test for that sample in `tests/golden.rs`, so files users already have stay covered.

### Release

```powershell
.\scripts\release.ps1
```

Runs the default suite under its coverage gate and the desktop tests, builds the optimized exe, and writes both versioned files to `dist\`. The version comes from `Cargo.toml`.

## License

[PayBack License (PBL)](LICENSE). Contributions welcome: see [CONTRIBUTING.md](CONTRIBUTING.md).
