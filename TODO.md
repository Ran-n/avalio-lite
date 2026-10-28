[//]: # ( ---------------------------------------------------------------------- )
[//]: # (+ Authors: 	Ran# <ran.hash@proton.me> )
[//]: # (+ Created: 	2026/10/28 10:26:46.388247 )
[//]: # (+ Revised: 	2026/10/28 22:47:24.992520 )
[//]: # ( ---------------------------------------------------------------------- )

# TODO

The backlog. Each item is self-contained: reading it alone is enough to
implement it fully, without asking anything. Work top to bottom unless an item
says otherwise.

Next item number: 16

## Index

1. [Create a logo and use it everywhere the app shows itself](#1-create-a-logo-and-use-it-everywhere-the-app-shows-itself)
2. [List the keyboard shortcuts in Settings and read them from config.toml](#2-list-the-keyboard-shortcuts-in-settings-and-read-them-from-configtoml)
3. [Change keyboard shortcuts from Settings](#3-change-keyboard-shortcuts-from-settings)
4. [Share one helper for "run this callback while the window is alive"](#4-share-one-helper-for-run-this-callback-while-the-window-is-alive)
5. [Cover the rest of the code and raise the coverage gate](#5-cover-the-rest-of-the-code-and-raise-the-coverage-gate)
6. [Gate branch coverage too](#6-gate-branch-coverage-too)
7. [Run mutation testing on the core logic and strengthen weak tests](#7-run-mutation-testing-on-the-core-logic-and-strengthen-weak-tests)
8. [Make the desktop baselines work on other displays](#8-make-the-desktop-baselines-work-on-other-displays)
9. [Turn on the hook that loads matching memory rules into each prompt](#9-turn-on-the-hook-that-loads-matching-memory-rules-into-each-prompt)
10. [Export a class sheet to an Excel file](#10-export-a-class-sheet-to-an-excel-file)
11. [Import pupils and marks from an Excel file](#11-import-pupils-and-marks-from-an-excel-file)
12. [Export everything to an .avalio backup file](#12-export-everything-to-an-avalio-backup-file)
13. [Restore everything from an .avalio backup](#13-restore-everything-from-an-avalio-backup)
14. [Add the classes of an .avalio backup to the app](#14-add-the-classes-of-an-avalio-backup-to-the-app)
15. [Duplicate a class, with or without its pupils and marks](#15-duplicate-a-class-with-or-without-its-pupils-and-marks)

## Adding an item

- Write for someone who has never seen the conversation: what happens today,
  what should happen instead, and why each non-obvious decision was made.
- Be concrete: real file paths, names, sizes, colours, keys. No open "maybe";
  if the implementer must choose, say how to choose and how to check.
- Use the template below. Implementation is a numbered list of steps in the
  order to do them; Done when is a checklist.
- Number it with "Next item number" above, then increase that number. Numbers
  are IDs: never reused or renumbered.
- File order is priority: the first item is the next one to do.
- Keep the Index in file order: one `<n>. [title](#anchor)` line per item;
  update it whenever an item is added, retitled, moved or deleted.
- Screenshots go in `docs/todo/<n>-<slug>.png`, linked from the item.
- Split anything bigger than one sitting into items that ship alone.

## Implementing an item

1. Read the whole item and every file it names; check `git status` first.
2. If the code has drifted from the item, adapt and note it; if the item
   contradicts a newer decision, ask before building.
3. Do the Implementation steps in order.
4. Write the listed tests, plus any others the change needs.
5. Lint, format, update file headers and run the full test suite until clean.
6. For visible changes, run the app and check it in the states the item names.
7. Update the listed docs; write the changelog line when committing.
8. Tick every Done when line, with evidence.
9. Delete the item, its Index line and its screenshots; add follow-up work as
   new items.

## Template

```markdown
## <n>. <short imperative title>

**Why:** what happens today and what is wrong with it.
**Goal:** the result, as the user will see it.

### Behaviour
- One bullet per case, edge cases included.

### Out of scope
- What this item deliberately doesn't change.

### Implementation
1. `<path>`: what changes and why.

### Tests
- `<test file>`: each case and what it proves.

### Docs
- `<doc file>`: what to change. Changelog: "<line>".

### Done when
- [ ] Checkable result.
```

---

## 1. Create a logo and use it everywhere the app shows itself

**Why:** the app has no logo. Inside the app, `BrandMark` (`ui/components.slint`, around line 224) draws a 30 × 30 px rounded tile in the theme accent (`Theme.accent`: `#2f54d6` light, `#8ea6ff` dark) with a bold lowercase "a" typed in Segoe UI on top. It appears in the sidebar (`ui/shell.slint`), on the first-run screen (`ui/setup-page.slint`) and on the lock screen (`ui/lock-page.slint`). Everywhere outside the app's own pages there is nothing: the window's title bar and the Windows taskbar show the generic default icon, `avalio-lite.exe` has no icon in Explorer, the installer (`installer/avalio-lite.iss`) uses Inno Setup's default icon, and `README.md` has no image. The user asked to "create and use a logo".
**Goal:** one logo, drawn as vector art, shown in the sidebar, first-run and lock screens, the window title bar, the taskbar, Explorer (for the `.exe`), the installer and its Start-menu shortcut, the uninstall entry, and at the top of the README. It stays legible at 16 px and works on both the light and the dark theme.

### Behaviour
- **The design:** a rounded square tile with a white tick (✓) inside, the stroke widening slightly into a small underline on the right, like a teacher's marking pen ticking a line in a mark book. That ties the logo to what the app does (marking), and a single bold stroke stays readable at 16 px, where letters and thin details blur. It replaces the typed "a", which depends on the installed font and turns into a grey blob at taskbar size.
  - Drawn on a 64 × 64 grid. Tile: `rect x=0 y=0 width=64 height=64 rx=15`.
  - Tick: one stroke path `M 17 34 L 27 44 L 47 22`, stroke width 7, round caps and joins.
  - Underline: `M 34 50 L 47 50`, same stroke style.
  - **Light** (`assets/logo.svg`): tile `#2f54d6` (the light theme's accent), strokes `#ffffff`.
  - **Dark** (`assets/logo-dark.svg`): tile `#8ea6ff`, strokes `#10162b` (the dark theme's `accent` and `on-accent`), so the in-app mark matches the theme.
- **Approval first:** before wiring the logo in, render it at 256, 32 and 16 px on white and on `#13161c`, and send the images to the user. If they want changes, adjust the paths and repeat. Don't start the "use it" steps until they approve.
- **In-app** (sidebar, first-run, lock screen): shown at 30 × 30 px, as now. Light theme uses `logo.svg`, dark theme uses `logo-dark.svg`, and switching themes swaps it immediately.
- **Window icon** (title bar and taskbar): always `logo.svg`, whatever the in-app theme, because Windows draws the title bar and taskbar itself.
- **`.exe` icon in Explorer, Start menu, installer and uninstall entry:** `assets/logo.ico` with 16, 24, 32, 48, 64 and 256 px images rendered from `logo.svg`, because Windows picks the size it needs from the `.ico`.
- **README:** the logo at 96 px, centred above the one-line description.
- **The portable `.exe`** gets the same icon as the installed one, since it's the same file.

### Out of scope
- Renaming the app or changing the wordmark text "avalio lite" next to the logo in the sidebar.
- Other icons (lock, sun, moon, gear, ⋯ in `Icon`) and the app's colours.
- A splash screen.

### Implementation
1. `assets/logo.svg` and `assets/logo-dark.svg`: create both from the shapes and colours in Behaviour. Keep them hand-readable: no editor metadata, `viewBox="0 0 64 64"`, no fixed `width`/`height`, so every size renders sharp.
2. `examples/render_logo.rs`: a small program that renders `assets/logo.svg` with the `resvg` crate (Slint already compiles it; add `resvg` and `ico` under `[dev-dependencies]` in `Cargo.toml`, pinned to the versions `cargo add` picks). It writes `assets/logo.ico` (16, 24, 32, 48, 64, 256 px) and `assets/logo-256.png`, plus preview PNGs on white and on `#13161c` into a folder given as the first argument. Same style as `examples/demo_data.rs`: header comment with usage, `ExitCode`, errors go to stderr, nothing panics. Because the `.ico` is generated, a changed SVG can be turned back into icons with one command.
3. Run `cargo run --example render_logo -- <scratch folder>` and send the previews to the user for approval (see Behaviour). Commit the generated `assets/logo.ico` and `assets/logo-256.png`, so the build doesn't need to render them.
4. `ui/components.slint`, `BrandMark`: replace the `Rectangle` + `Text` with an `Image` whose `source` is `Theme.dark ? @image-url("../assets/logo-dark.svg") : @image-url("../assets/logo.svg")`, 30 × 30 px, and keep the component name, so `shell.slint`, `setup-page.slint` and `lock-page.slint` need no change (grep `BrandMark` to confirm those are the only three users). Add `accessible-role: image` and `accessible-label: "avalio lite"`.
5. `ui/app-window.slint`, `AppWindow`: set `icon: @image-url("../assets/logo.svg");` for the title bar and taskbar.
6. `Cargo.toml` + `build.rs`: add `winresource` under `[build-dependencies]`. In `build.rs`, after the Slint compile, when `CARGO_CFG_TARGET_OS` is `windows`, embed `assets/logo.ico` with `winresource::WindowsResource::new().set_icon("assets/logo.ico").compile()`. If that call fails, panic with a message naming the file: a build without its icon is a broken build, not something to ignore. Add `cargo:rerun-if-changed=assets/logo.ico`.
7. `installer/avalio-lite.iss`, `[Setup]`: add `SetupIconFile=..\assets\logo.ico`. Keep `UninstallDisplayIcon={app}\avalio-lite.exe`, which now carries the embedded icon.
8. `README.md`: add `<p align="center"><img src="assets/logo.svg" alt="avalio lite" width="96"></p>` between the title and the description, as in the user's other projects (timtapper's README).
9. Run `metadata -c` on `examples/render_logo.rs` and `metadata -m` on every edited `.rs` and `.md` file. `metadata` only handles `.py .sh .rs .md`, so skip the `.svg`, `.slint` and `.iss` files.

### Tests
- `tests/logo.rs`, `both_logo_variants_parse_and_are_square`: `usvg` parses `assets/logo.svg` and `assets/logo-dark.svg`, and each view box is 64 × 64. Proves the files the UI and installer load are valid.
- `tests/logo.rs`, `ico_has_every_windows_size`: decode `assets/logo.ico` with the `ico` crate and assert exactly the sizes 16, 24, 32, 48, 64 and 256. Proves Explorer and the taskbar get a sharp image at each size.
- `tests/logo.rs`, `ico_is_up_to_date_with_the_svg`: render `logo.svg` at 32 px with the same function the example uses (move it into a small `pub fn` the example and the test share, so the code isn't duplicated) and compare it with the 32 px image in `logo.ico`, allowing a per-pixel difference of 2. Fails when someone edits the SVG and forgets to regenerate the icon.
- `tests/logo.rs`, `logo_stays_legible_at_16px`: render at 16 px and assert the tick's centre pixel (around x 8, y 10) is closer to white than to the tile colour. Guards against a redesign that disappears at taskbar size.
- `examples/render_logo.rs` exits non-zero with a message when the output folder argument is missing (run it once by hand; note the result in the reply).
- Manual, light and dark: run the app and take screenshots of the sidebar, the first-run screen (start with no `config.toml`) and the lock screen in both themes. Check the logo is sharp and swaps with the theme, and that the title bar and taskbar show the logo. Run `scripts/release.ps1` and check the `.exe` icon in Explorer and the installer's own icon.

### Docs
- `README.md`: the logo image (step 8). In the Layout block, add a line `assets/        logo (SVG source, generated .ico/.png; regenerate with cargo run --example render_logo)`.
- `CHANGELOG.md` (when committing): "Added the avalio-lite logo to the window, taskbar, program file, installer and README".

### Done when
- [ ] The user approved the rendered logo (their reply quoted in the hand-off).
- [ ] `assets/logo.svg`, `assets/logo-dark.svg`, `assets/logo.ico` and `assets/logo-256.png` exist and are committed.
- [ ] Sidebar, first-run and lock screens show the logo in both themes (screenshots attached), and it swaps when the theme is toggled.
- [ ] The title bar and the taskbar show the logo (screenshot).
- [ ] `dist\avalio-lite-<version>-portable.exe` and the installer show the logo in Explorer, and the installed app's Start-menu entry and Settings → Apps entry show it.
- [ ] README shows the logo.
- [ ] `cargo test` (including `tests/logo.rs`), `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` all pass.

## 2. List the keyboard shortcuts in Settings and read them from config.toml

**Why:** the app has exactly one keyboard shortcut, Ctrl+L to lock, and it is hard-coded in three places: the key check in `ui/app-window.slint` (the `FocusScope`'s `capture-key-pressed`, around line 67), the sidebar label `@tr("Lock (Ctrl+L)")` in `ui/shell.slint` (around line 92) and the PIN card text in `ui/settings-page.slint` (around line 46, "…lock it any time with Ctrl+L or the Lock button."). Nothing in the app lists shortcuts, and they can't be changed. The user asked for "a config part that lists and allows to edit keybinds". This item is the first half: a set of shortcuts, a Settings card that lists them, and storage in `config.toml`. Item 3 adds editing in the app; it builds on this one, so do this first.
**Goal:** Settings has a "Keyboard shortcuts" card listing every shortcut (what it does, and its keys, such as "Ctrl+L"). Six actions have shortcuts, all working from anywhere in the main screen, even while typing in a cell. A `[shortcuts]` table in `config.toml` can change any of them, and the card, the sidebar label and the PIN card text all show the keys actually in use.

### Behaviour
- **The actions and their default keys**, in this order (the card lists them in the same order):
  - Lock: **Ctrl+L** (unchanged).
  - Switch between light and dark: **Ctrl+Shift+D**. Not Ctrl+D, so that it's never hit by accident while typing.
  - Show or hide weights and scales: **Ctrl+H**.
  - Open Settings: **Ctrl+,** (the comma key). This is the usual Windows shortcut for settings (VS Code, Teams, Edge).
  - Next class: **Ctrl+PageDown**.
  - Previous class: **Ctrl+PageUp**. Excel uses these two keys to switch between sheets, and here each class is a sheet.
- **Where they work:** only on the main screen (sheet or Settings), including while a cell or text field has focus. The key is caught before the focused field sees it, as Ctrl+L is today. On the first-run screen and on the lock screen, no shortcut does anything and the key goes on to the focused field, so typing a PIN is never hijacked.
- **What each action does:**
  - Lock: does exactly what the Lock button does today. Without a PIN it shows the existing `LockNeedsPin` message.
  - Switch between light and dark: same as the sidebar's theme button, saved the same way.
  - Show or hide weights and scales: same as the "⋯" menu entry (`Gradebook.toggle-column-settings`).
  - Open Settings: switches the main area to Settings. Pressed again while Settings is open, it goes back to the sheet, so one shortcut both opens and closes Settings.
  - Next and Previous class: switch the sheet to the class below or above the current one in the sidebar list, and show the sheet if Settings was open. At the last class, Next goes round to the first; at the first, Previous goes to the last. With no classes they do nothing. With one class they keep it selected.
- **Key format** in `config.toml` and on screen:
  - `Ctrl`, `Shift` and `Alt` joined with `+`, always in that order, then the key.
  - Keys allowed: A–Z, 0–9, `,`, `.`, F1–F12, PageUp, PageDown, Home, End.
  - Every shortcut needs Ctrl or Alt, except F1–F12, which may stand alone. Plain letters would break typing in cells.
  - **Ctrl+Alt together is refused:** on Spanish and Galician keyboards AltGr sends Ctrl+Alt, and AltGr+2 types "@".
  - Letters are case-insensitive when read (`ctrl+l` = `Ctrl+L`) and are always written and shown in upper case.
- **On-screen names in Galician:** Shift is shown as "Maiús", as on Galician Windows keyboards. Ctrl, Alt and the key names stay the same in both languages. `config.toml` always uses the English names, so the file doesn't depend on the language.
- **`config.toml`:** an optional table, holding only the actions changed from their defaults:
  ```toml
  [shortcuts]
  lock = "Ctrl+K"
  next-class = "F8"
  ```
  The action names are `lock`, `toggle-theme`, `toggle-column-settings`, `open-settings`, `next-class` and `previous-class`. A file without the table, or one written before this change, uses all the defaults.
- **A bad entry is never silent:**
  - Bad entries are: an unknown action name, a key that doesn't parse or isn't allowed, or two actions on the same keys.
  - The app still starts, logs a warning naming the entry, and uses the default for that action.
  - When the main screen opens, an error banner reads: "Some keyboard shortcuts in config.toml weren't valid and use their defaults: lock = "Ctrl+"." It lists each bad entry.
  - **For a clash**, every action involved goes back to its default. This repeats until no two actions share keys. It always ends, because the defaults never clash with each other (a test checks this).
- **Labels that mention Ctrl+L:**
  - The sidebar Lock item reads "Lock (Ctrl+K)", with the actual keys.
  - The PIN card text reads "…lock it any time with the Lock button or Ctrl+K."
  - Both follow the language and the current shortcut.
- **The Settings card:** placed after Language and before PIN and encryption. Each action is one row: the action name on the left in `Theme.text`, the keys on the right as a key-cap pill. The pill is a `Tag`-style rounded rectangle in `Theme.raised` with `Theme.muted` text and one pill per key ("Ctrl", "Shift", "L"), so the keys read like a keyboard. The card description says: "Shortcuts work anywhere on this screen, even while you type in a cell. You can change them in config.toml." Item 3 replaces the second sentence.

### Out of scope
- Changing shortcuts inside the app (item 3).
- Shortcuts for "+ Add pupil", "+ Add column" or anything that opens a popup in `ui/sheet-page.slint`. Their popups are local to `SheetPage` and can't be opened from the window today; doing that is a separate change.
- Moving around the sheet with the keyboard (arrow keys, Tab, Enter in cells).
- Shortcuts on the lock and first-run screens.

### Implementation
1. **`src/shortcuts.rs` (new, declared `pub mod shortcuts;` in `src/lib.rs`):** the shortcut model, with no Slint or file access, so it can be tested on its own.
   - `pub enum Action { Lock, ToggleTheme, ToggleColumnSettings, OpenSettings, NextClass, PreviousClass }` with `ALL: [Action; 6]` in the order above, `config_name()` and `from_config_name()`.
   - `pub struct KeyCombo { ctrl: bool, shift: bool, alt: bool, key: Key }`. `Key` is an enum covering exactly the keys allowed in Behaviour, with `FromStr` and `Display` in the canonical English form ("Ctrl+Shift+D").
   - The validity rules from Behaviour: Ctrl or Alt needed except for F-keys, Ctrl+Alt refused. Put them in one `KeyCombo::new(...) -> Result<KeyCombo, ShortcutError>`, used by both parsing and key presses, so the rules exist once.
   - `pub struct Shortcuts` holds one `KeyCombo` per action. It has:
     - `Shortcuts::defaults()`;
     - `Shortcuts::from_config(&BTreeMap<String, String>) -> (Shortcuts, Vec<ShortcutProblem>)`, which applies the bad-entry and clash rules from Behaviour;
     - `combo(Action)`;
     - `action_for(&KeyCombo) -> Option<Action>`;
     - `KeyCombo::from_key_press(text: &str, ctrl, shift, alt) -> Option<KeyCombo>`, which maps Slint's `event.text`. A letter arrives as typed: "l" or "L" with Shift, upper-cased here. `Key.PageDown` and the F-keys arrive as Slint's private-use characters; map them using Slint's `slint::platform::Key` values (`Key::PageDown`, `Key::F1`…), never hard-coded code points.
   - `ShortcutProblem` holds the entry text and the reason (`UnknownAction`, `BadKeys`, `Clash`), for logging and the banner.
2. **`src/error.rs`:** add `AppError::InvalidShortcuts(Vec<ShortcutProblem>)` for the banner, so it goes through the existing `error_message` path. Add both arms in `src/i18n/errors.rs`, English and Galician. The compiler enforces both, because each language is one `match`.
3. **`src/i18n/shortcuts.rs` (new, `mod shortcuts;` in `src/i18n/mod.rs`):**
   - `pub fn action_label(Language, Action) -> &'static str` (English: "Lock", "Switch between light and dark", "Show or hide weights and scales", "Open Settings", "Next class", "Previous class"; Galician: "Bloquear", "Cambiar entre claro e escuro", "Amosar ou agochar pesos e escalas", "Abrir os axustes", "Clase seguinte", "Clase anterior").
   - `pub fn key_names(Language, &KeyCombo) -> Vec<String>`: the pill texts, with "Maiús" for Shift in Galician.
   - `pub fn display(Language, &KeyCombo) -> String`: those names joined with "+", for the labels.
   - One `match` per language, the same pattern as `errors.rs`.
4. **`src/config.rs`:** add `#[serde(default, skip_serializing_if = "BTreeMap::is_empty")] pub shortcuts: BTreeMap<String, String>` to `AppConfig`, holding only the changes from the defaults. Keep it outside `Preferences`, so `Preferences` stays `Copy` (`AppState::preferences()` at `src/app/state/mod.rs:155` and `src/app/gradebook/mod.rs:58` copy it). Add `shortcuts: BTreeMap::new()` to the two `AppConfig` literals in `src/app/state/mod.rs` (lines 122 and 135, in `create_database` and `open_existing`); grep `AppConfig {` in `src` and `tests` for any others.
5. **`src/app/state/mod.rs`:**
   - `AppState` gets `shortcuts: Shortcuts` and `shortcut_problems: Vec<ShortcutProblem>`. They are built from the config in `start()` and in `choose()` (the same places `config` is set), using `Shortcuts::from_config`.
   - Log each problem with `log::warn!` naming the entry.
   - Add `pub fn shortcuts(&self) -> &Shortcuts` and `pub fn take_shortcut_problems(&mut self) -> Vec<ShortcutProblem>`, which empties the list so the banner shows once.
6. **`ui/shortcuts.slint` (new):**
   - `export enum ShortcutAction { none, lock, toggle-theme, toggle-column-settings, open-settings, next-class, previous-class }`.
   - `export struct ShortcutItem { label: string, keys: [string] }`.
   - `export global Shortcuts { in property <[ShortcutItem]> items; in property <string> lock-keys; pure callback find(string, bool, bool, bool) -> ShortcutAction; }`. This is a global so the settings card, the sidebar and the window read it without passing properties through `Shell`.
   - Export all three from `ui/app-window.slint` next to the existing exports.
7. **`ui/app-window.slint`:**
   - Add `in-out property <Section> section` and bind it to `Shell`'s `section`, so the window can switch to Settings and back.
   - Keep the `shortcuts` FocusScope taking focus back (the `changed locked` / `changed setup-required` handlers and Shell's `navigated`): keys only arrive while something has focus, and `tests/ui.rs` `regression_ctrl_l_*` check it.
   - Replace the Ctrl+L check in `capture-key-pressed` with: reject when `setup-required` or `locked`; otherwise `action = Shortcuts.find(event.text, event.modifiers.control, event.modifiers.shift, event.modifiers.alt)`; return `reject` for `none`; run the action and return `accept`.
   - The six actions:
     - lock calls `root.lock-now()`;
     - toggle-theme calls `root.choose-theme(!Theme.dark)`;
     - toggle-column-settings calls `Gradebook.toggle-column-settings()`;
     - open-settings flips `root.section` between `settings` and `sheet`;
     - next-class and previous-class set `root.section = Section.sheet` and call `Gradebook.select-adjacent-class(1)` or `(-1)`.
   - Update the comment above it.
8. **`ui/gradebook.slint`:** add `callback select-adjacent-class(int);`. In `src/app/gradebook/classes.rs`, wire it next to `on_select_class`, as follows:
   - read the class list with `classes::list_classes`, find `current-class-id` and step with wrap-around;
   - call `run(…, "switch class", …)` with `Refresh::All { select: Some(id) }`;
   - do nothing when the list is empty.

   Put the index arithmetic in a small pure function, `adjacent_index(len, current, delta) -> Option<usize>` in `src/gradebook/classes.rs`, so it can be unit-tested.
9. **`src/app/shortcuts.rs` (new, `mod shortcuts;` in `src/app/mod.rs`):**
   - `wire(window, state)`: sets `Shortcuts.find`, which builds a `KeyCombo` with `KeyCombo::from_key_press` and looks it up in `state.borrow().shortcuts()`.
   - `show(window, state)`: fills `items` and `lock-keys` in the current language.
   - Call `wire` from `open` in `src/app/mod.rs`, next to the other `wire` calls. Call `show` from `gradebook::show_database` and after a language change in `wire_language` (`src/app/callbacks.rs`), so the labels follow the language. Show `take_shortcut_problems()` as an error banner with `view::show_error` when the main screen opens.
10. **`ui/settings-page.slint`:** add the "Keyboard shortcuts" `Card` described in Behaviour. Put the key pill as `KeyCap` in `ui/components.slint`, next to `Tag`, so item 3 can reuse it.
11. **The two labels:**
    - In `ui/shell.slint`, the Lock item becomes `@tr("Lock ({})", Shortcuts.lock-keys)`.
    - In `ui/settings-page.slint`, the PIN text becomes `@tr("The database file is encrypted with your PIN. The app asks for it when it opens; lock it any time with the Lock button or {}.", Shortcuts.lock-keys)`.
    - Remove the two old msgids from `lang/gl/LC_MESSAGES/avalio-lite.po`.
12. **`lang/gl/LC_MESSAGES/avalio-lite.po`:** add Galician for every new `@tr` string: the card title "Atallos de teclado", the card description, the two changed labels. `tests/i18n.rs` fails on missing or stale entries, so run it.
13. **File headers:** run `metadata -c` on the new `.rs` files and `metadata -m` on every edited `.rs` and `.md` file. Skip `.slint`, `.po` and `.toml`, which `metadata` doesn't support.

### Tests
- **`tests/shortcuts.rs` (new):**
  - `defaults_never_clash`: every pair of default combos differs. The clash rule depends on this.
  - `combos_round_trip_through_text`: proptest over every allowed `KeyCombo`: `Display` then `FromStr` returns the same combo.
  - `parsing_is_case_insensitive_and_canonical`: `"ctrl+shift+d"` parses and displays as `"Ctrl+Shift+D"`; `"Shift+Ctrl+D"` parses to the same combo.
  - `plain_letters_and_altgr_are_refused`: `"L"`, `"Shift+L"` and `"Ctrl+Alt+2"` are errors, while `"F5"` and `"Alt+L"` are accepted.
  - `unknown_keys_are_refused`: `"Ctrl+"`, `"Ctrl+Space"`, `"Ctrl+F13"` and `""` are errors.
  - `config_overrides_replace_defaults`: `{lock = "Ctrl+K"}` gives Ctrl+K for Lock, the defaults for the rest, and no problems.
  - `bad_entries_fall_back_and_are_reported`: an unknown action and a bad key each produce one problem, and the default stays for the action.
  - `clashing_overrides_both_revert`: setting `toggle-theme = "Ctrl+L"` with `lock` unchanged reverts toggle-theme to Ctrl+Shift+D and reports a clash; two overrides on the same keys both revert.
  - `key_presses_map_to_actions`: `from_key_press("l", true, false, false)` and `("L", true, true, false)` give Ctrl+L and Ctrl+Shift+L; Slint's PageDown text with Ctrl gives NextClass; `("2", true, false, true)` (AltGr) gives `None`.
- **`tests/config.rs`:**
  - `shortcuts_round_trip`: save then load keeps a `[shortcuts]` table.
  - `settings_files_without_shortcuts_load_with_defaults`: extend the existing older-file test to also check that `shortcuts` is empty.
  - `no_shortcuts_table_is_written_when_nothing_changed`: the saved text has no `[shortcuts]` line.
- **`tests/state.rs`:** `bad_shortcuts_are_reported_once`: a config with one bad entry gives one problem from `take_shortcut_problems()`, then none.
- **`tests/gradebook.rs`:** `adjacent_class_wraps_around`: `adjacent_index` for lengths 0, 1 and 3, both directions, at both ends.
- **`tests/i18n.rs`:** `every_action_has_a_label_in_every_language` loops over `Action::ALL` and `Language` and checks no label is empty; `shift_is_maius_in_galician`.
- **Manual, in the release build, light and dark, English and Galician:**
  - screenshot the Settings card (Galician at the 1000 px minimum width, as it's the longer language);
  - check that each of the six shortcuts works from a focused cell;
  - check that Ctrl+L on the lock screen does nothing and the PIN field still takes digits;
  - put `lock = "Ctrl+"` in `config.toml`, start the app and screenshot the banner;
  - delete the test `config.toml` afterwards.

### Docs
- `README.md`, section "Using it": add a short "Keyboard shortcuts" list of the six defaults, and say they can be changed in `config.toml` under `[shortcuts]`, with the action names.
- `README.md`, line 62 (PIN and encryption): "**Ctrl+L** (or your own shortcut) or **Lock** in the sidebar…".
- `README.md`, Layout block: add `src/shortcuts.rs` and `ui/shortcuts.slint`.
- `project_spec.md` (project memory): add a line on shortcuts. Only after the user confirms, per the spec rule.
- `CHANGELOG.md` (when committing): "Added keyboard shortcuts for the theme, weights and scales, Settings and switching classes, listed in Settings and changeable in config.toml".

### Done when
- [ ] The six shortcuts work from a focused cell, and do nothing on the lock and first-run screens (noted per shortcut in the hand-off).
- [ ] Settings lists them with key pills in both languages and both themes (screenshots).
- [ ] The Lock label and PIN text show the configured keys (screenshot after setting `lock = "Ctrl+K"`).
- [ ] A bad `config.toml` entry shows the banner and falls back (screenshot), and the log has the warning.
- [ ] `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass.

## 3. Change keyboard shortcuts from Settings

**Why:** after item 2, Settings lists the six keyboard shortcuts, but changing one means editing `config.toml` by hand while the app is closed. The user asked to be able to edit them in the app. Do this after item 2: it uses item 2's `Shortcuts` model, the `[shortcuts]` table and the `KeyCap` pill.
**Goal:** each row of the "Keyboard shortcuts" card has a "Change" button. You press it, press the new keys, and the shortcut is saved at once. Each row has its own reset, and there's one "Reset all" for every row.

### Behaviour
- **Changing one:**
  - Pressing "Change" puts that row into recording: the pills are replaced by "Press the new keys…" in `Theme.accent`, and the button becomes "Cancel".
  - Only one row records at a time. Pressing "Change" on another row moves the recording there.
- **While recording:**
  - Every key press goes to the recorder, not to the shortcuts or the focused field.
  - A press of only Ctrl, Shift or Alt waits for the actual key.
  - Esc cancels and leaves the shortcut unchanged.
  - Any other combination is checked with item 2's rules.
- **A valid new combination:**
  - It is saved to `config.toml` immediately, as with other settings, with no Save button (inline editing).
  - The row shows the new pills and the recording ends.
  - The status banner reads "Lock is now Ctrl+K."
- **An invalid combination:** the row keeps recording, and an error banner says why, then the user tries again or presses Esc. Why the combination is refused:
  - a plain letter: "Add Ctrl or Alt: a plain letter would type into the cell.";
  - Ctrl+Alt: "Ctrl+Alt is the AltGr key on many keyboards, so it can't be used.";
  - a key that isn't allowed: "That key can't be used for a shortcut.".
- **A clash:**
  - Refused with "Ctrl+H is already used by Show or hide weights and scales.", and the row keeps recording.
  - Refused rather than swapped, so a shortcut never moves silently from another action.
- **Text-editing keys are refused:** Ctrl+A, Ctrl+C, Ctrl+V, Ctrl+X, Ctrl+Y and Ctrl+Z, with "Ctrl+C is used for editing text." Text fields need these keys, and item 2's capture would take them before the field. Add this rule to `KeyCombo` validation, so a hand-edited `config.toml` can't set them either.
- **Resetting:**
  - A row whose shortcut differs from its default shows a small "Reset" ghost button. It restores the default, unless that would clash, in which case the clash message names the action that has those keys now.
  - "Reset all" at the bottom of the card removes the whole `[shortcuts]` table and restores all six defaults. It's greyed out (disabled style, per the UI rule) when nothing differs from the defaults.
- **Leaving Settings or locking while recording** cancels the recording.
- **What gets saved:** only differences from the defaults are written (a shortcut set back to its default removes its line), the same format as item 2.

### Out of scope
- Several shortcuts for one action, or shortcuts for new actions.
- Chord shortcuts (two key presses in a row, such as Ctrl+K then Ctrl+S).

### Implementation
1. **`src/shortcuts.rs`:**
   - Add the text-editing refusal to `KeyCombo::new`, with its own `ShortcutError` variant.
   - Add `Shortcuts::with_change(Action, KeyCombo) -> Result<Shortcuts, ShortcutError>`, which returns `Clash(other_action)` when another action has the combo.
   - Add `Shortcuts::to_config() -> BTreeMap<String, String>`, holding only the non-defaults.
   - Add `is_default(Action)`.

   Keeping these pure keeps the rules testable without the UI.
2. **`src/error.rs` and `src/i18n/errors.rs`:** add `AppError::Shortcut(ShortcutError)`, with English and Galician text for every variant (wording in Behaviour; Galician e.g. "Ctrl+H xa se usa para Amosar ou agochar pesos e escalas.").
3. **`src/i18n/messages.rs`:** add `Message::ShortcutChanged { action, keys }` ("Lock is now Ctrl+K." / "Bloquear agora é Ctrl+K.") and `Message::ShortcutsReset` ("All keyboard shortcuts are back to their defaults." / "Todos os atallos de teclado volven ser os de orixe.").
4. **`src/app/state/mod.rs`:**
   - Add `set_shortcut(Action, KeyCombo) -> Result<()>`, `reset_shortcut(Action) -> Result<()>` and `reset_all_shortcuts() -> Result<()>`.
   - Each builds the new `Shortcuts` with `with_change`, writes `to_config()` into a cloned config, saves it with `save_config` and only then replaces `self.shortcuts`, so a failed save changes nothing. This is the same pattern as `set_theme`.
5. **`ui/shortcuts.slint`:**
   - Add `in-out property <int> recording: -1` (the row index) to the global.
   - Add `callback record(int, string, bool, bool, bool)`, `callback reset(int)` and `callback reset-all()`.
   - Add `is-default: bool` to `ShortcutItem`.
6. **`ui/app-window.slint`, `capture-key-pressed`:**
   - Before the shortcut lookup, when `Shortcuts.recording >= 0`: Esc sets `recording = -1`; a press of only modifiers is ignored; anything else calls `Shortcuts.record(recording, event.text, …)`. Always return `accept`, so the focused field never gets the keys.
   - Set `recording = -1` when `section` changes away from Settings and in `lock-now`.
7. **`ui/settings-page.slint`:** add the Change/Cancel button, the recording text, the per-row Reset ghost button (only when `!is-default`) and "Reset all" (disabled style when every row `is-default`), all as described in Behaviour.
8. **`src/app/shortcuts.rs`:**
   - Wire `record`: build the combo, call `set_shortcut`. On success, end recording, refresh `items` and `lock-keys` and show `ShortcutChanged`. On error, keep recording and show the error banner with `view::show_error`, logging at `warn`.
   - Wire `reset` and `reset-all` the same way.
9. **`lang/gl/LC_MESSAGES/avalio-lite.po`:** add Galician for "Change" (Cambiar), "Cancel" (Cancelar), "Reset" (Restablecer), "Reset all" (Restablecer todo) and "Press the new keys…" (Preme as novas teclas…). Change the card description's second sentence to "Press Change and then the new keys." Run `tests/i18n.rs`.
10. **File headers:** run `metadata -m` on every edited `.rs` and `.md` file.

### Tests
- **`tests/shortcuts.rs`:**
  - `editing_keys_are_refused`: each of Ctrl+A/C/V/X/Y/Z is an error, also when read from config (`from_config` reports it and keeps the default).
  - `a_change_that_clashes_names_the_other_action`: `with_change(ToggleTheme, Ctrl+L)` returns `Clash(Lock)`.
  - `only_differences_are_saved`: change one shortcut and change it back, and `to_config()` is empty.
  - `reset_restores_the_default`.
- **`tests/state.rs`:**
  - `a_changed_shortcut_survives_a_restart`: set Lock to Ctrl+K, restart the state from the same config folder, and `shortcuts().combo(Lock)` is Ctrl+K.
  - `a_failed_change_leaves_everything_as_it_was`: a clash returns the error, and both the config file and `shortcuts()` are unchanged.
  - `reset_all_removes_the_table`: the saved file has no `[shortcuts]` line.
- **`tests/i18n.rs`:** every `ShortcutError` variant and both new messages have text in both languages. This comes free with `match`, but check that none is empty.
- **Manual, in the release build, light and dark, Galician at 1000 px width:**
  - screenshot a row while recording, after a refused clash (banner), after a successful change, and with "Reset all" both enabled and greyed;
  - check that while recording, typing in no field happens and Ctrl+L doesn't lock;
  - check that Esc cancels;
  - check that leaving Settings cancels.

### Docs
- `README.md`, "Keyboard shortcuts" (added by item 2): say they're changed in Settings → Keyboard shortcuts; keep the `config.toml` note as the alternative; list the keys that can't be used (plain letters, Ctrl+Alt, Ctrl+A/C/V/X/Y/Z).
- `CHANGELOG.md` (when committing): "Keyboard shortcuts can be changed and reset in Settings".

### Done when
- [ ] Changing, cancelling (Esc and leaving Settings), resetting one and resetting all work in the app (screenshots of each state).
- [ ] Clashes, plain letters, Ctrl+Alt and editing keys are refused with their message, and the row keeps recording.
- [ ] A change survives restarting the app.
- [ ] `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass.

## 4. Share one helper for "run this callback while the window is alive"

**Why:** every UI callback in `src/app` starts with `let Some(window) = weak.upgrade() else { return };`. That line is repeated 34 times (`callbacks.rs` 9, `gradebook/columns.rs` 9, `lock_screen.rs` 5, `gradebook/classes.rs` 4, `gradebook/pupils.rs` 4, `gradebook/cells.rs` 1, `gradebook/mod.rs` 1, plus an `if let` form in `gradebook/panel.rs:35`). Its `else { return }` branch only runs if a callback fires after the window is dropped, which never happens while the app runs. So it is 34 uncovered regions in the coverage report (`.\scripts\coverage.ps1`) and 34 copies of the same rule, against the DRY rule. Do this before item 5, which raises the coverage gate.
**Goal:** one helper owns the rule. The coverage report no longer lists the `else` regions, and each callback reads as just what it does.

### Behaviour
- Every button, field and menu behaves exactly as before; the UI tests (`tests/ui.rs`, 69 tests) and the desktop tests prove it.
- If a callback ever does run after the window is gone, the helper logs it at `debug` (`log::debug!("callback after the window closed")`) and does nothing, instead of returning silently.

### Out of scope
- What any callback does. This is a pure refactor.

### Implementation
1. `src/app/mod.rs`: add `pub(crate) fn with_window<F: Fn(&AppWindow) + 'static>(window: &AppWindow, action: F) -> impl Fn() + 'static`. It captures `window.as_weak()` and calls `action` with the upgraded window, logging at `debug` when it can't upgrade. Add variants for the argument shapes that are actually used: grep `window.on_` and `.on_` on `Gradebook` in `src/app` to list them (`()`, `(SharedString)`, `(i32)`, `(bool)`, `(i32, SharedString)`, `(i32, i32)`, `(i32, i32, SharedString)`, `(ColumnType)`, `(i32, bool)`). Prefer one generic helper over many if it type-checks cleanly with Slint's `on_*` signatures; check by compiling.
2. `src/app/callbacks.rs`, `src/app/lock_screen.rs` and each file in `src/app/gradebook/`: replace each `let (weak, shared) = (window.as_weak(), state.clone()); x.on_y(move |…| { let Some(window) = weak.upgrade() else { return }; … })` with the helper. `on_erase_last` in `lock_screen.rs` returns a value and doesn't need the window, so leave it as it is.
3. Run `cargo fmt` and `cargo clippy --all-targets -- -D warnings`, and update file headers with `metadata -m` on every edited file.

### Tests
- `tests/ui.rs`: all 69 tests must pass unchanged; together they click every callback.
- `tests/desktop.rs` (`cargo test --test desktop -- --ignored --test-threads=1`): passes against the existing baselines.
- `tests/error_logging.rs`: add `a_callback_after_the_window_closed_is_logged_at_debug`. Keep a `Weak` to a `Ui` window, drop the `Ui`, invoke the callback through the stored closure, and assert the `debug` line appears. Raise the logger's level for that test with `FileLogger::new(LevelFilter::Debug)`.

### Docs
- None besides the changelog: "Callbacks share one window-lifetime helper."

### Done when
- [ ] `grep -rn "weak.upgrade() else" src/app` finds nothing.
- [ ] `.\scripts\coverage.ps1` passes and no longer lists those lines.
- [ ] The default and desktop suites pass.

## 5. Cover the rest of the code and raise the coverage gate

**Why:** since the toolchain was pinned to Rust 1.99.0 (`rust-toolchain.toml`, 2026-10-28), line coverage is 97.0 % and the gate in `scripts/coverage.ps1` is `$MinLines = 96`. (The earlier 84.8 % came from Rust 1.95, whose coverage data cargo-llvm-cov 0.9 misreads.) Apart from the callback `else` branches (item 4, do it first), the uncovered lines are:
- SQL `?` error arms in `src/gradebook/*.rs` and `src/db/mod.rs`: for example `columns.rs` 28, 44, 47–52, 69, 80, 91, 105, 117; `classes.rs` 18, 56; `pupils.rs` 19, 30; `sheet.rs` 35; `cells.rs` 27; `db/mod.rs` 31, 70, 75, 126.
- `NativeDialogs` in `src/app/dialogs.rs`: the real Windows dialogs, which every test replaces with `FakeDialogs`.
- `app::run` (`src/app/mod.rs` 38–40) and `main` (`src/main.rs`).
- Logs that only run when a cleanup fails after an earlier failure: `src/app/state/mod.rs` 205, 252, 272, 279; `src/app/state/protection.rs` 98–99, 110, 123, 137–138, 151.
- Small leftovers: `src/config.rs` 95, 104; `src/db/migrations.rs` 28–29; `src/i18n/errors.rs` 33, 73; `src/i18n/messages.rs` 37, 42, 63; `src/storage.rs` 23; `src/app/gradebook/view.rs` 122, 142, 193, 202; `src/app/language.rs` 15; `src/app/theme.rs` 14.

Line numbers are from before the toolchain pin, and some of these lines no longer count; rerun `.\scripts\coverage.ps1` for the current list. The testing rules allow only the `main` guard and dev-only fallbacks to stay uncovered.
**Goal:** every uncovered line is either covered by a test or deleted because nothing calls it. The few that can't be reached from a test are listed, with the reason, at the top of `scripts/coverage.ps1`. The gate goes up to the new whole-number total.

### Behaviour
- No visible change to the app.

### Out of scope
- Branch coverage (item 6) and mutation testing (item 7).

### Implementation
1. Run `.\scripts\coverage.ps1` and copy the uncovered lines into a scratch list.
2. SQL error arms: make them fail for real. Close the connection's underlying tables (`DROP TABLE` in a test connection), open the database read-only (`OpenFlags::SQLITE_OPEN_READ_ONLY`) for write paths, or bypass CHECK constraints with `PRAGMA ignore_check_constraints = ON` (as `tests/gradebook.rs` `an_unknown_column_kind_in_the_file_is_reported` does). Add one test per function in that module's test file. Each asserts the error is `AppError::Database` and that nothing was half-written.
3. Double-fault logs: drive them with the Windows file locks in `tests/common/mod.rs` (`hold_exclusively`, `hold_against_removal`) at the moment of cleanup. For example, hold the `.rekey` copy against removal while `export` fails, then assert the ERROR line with the `logged` helper in `tests/error_logging.rs`. If a path truly can't be reached, delete it when nothing calls it (grep first); otherwise list it in step 5.
4. `app::run` and `NativeDialogs`: `run` only wires in `NativeDialogs`, and every rfd call opens a modal OS dialog. Cover `run` with a desktop test (`tests/desktop.rs`) that starts the release-style exe (`target\debug\avalio-lite.exe`) with `AVALIO_LITE_CONFIG_DIR` set to a scratch folder, clicks "Browse…" for real, finds the native folder dialog by its owner window with Win32 (`EnumWindows` plus `GetWindow(GW_OWNER)`) and closes it with `WM_CLOSE`. That desktop run isn't part of line coverage, so list `dialogs.rs` and `main.rs` in step 5 as covered by desktop tests only.
5. `scripts/coverage.ps1`: add a comment block listing the lines left uncovered on purpose, each with its reason. Set `$MinLines` to the new whole-number total.

### Tests
- One test per covered error path, in the module's own test file (`tests/gradebook.rs`, `tests/db.rs`, `tests/state.rs`, `tests/encryption.rs`). Each names the failure and asserts both the returned error and the state left behind.
- `tests/desktop.rs`: `the_real_dialogs_open_and_close`, from step 4.

### Docs
- `README.md`, *Tests* section: mention that `scripts/coverage.ps1` lists the lines left uncovered on purpose. Changelog: "Test coverage raised to <n> %; the gate follows."

### Done when
- [ ] `.\scripts\coverage.ps1` passes with the raised `$MinLines`.
- [ ] Every line it still reports is listed with a reason in `scripts/coverage.ps1`.
- [ ] The default and desktop suites, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass.

## 6. Gate branch coverage too

**Why:** the testing rules want branch coverage measured and gated. `scripts/coverage.ps1 -Branches` already reports it through the nightly toolchain (`cargo +nightly llvm-cov --branch`), but cargo-llvm-cov 0.9 can only fail on lines, functions and regions (`--fail-under-lines`, `--fail-under-functions`, `--fail-under-regions`), not on branches. So a drop in branch coverage goes unnoticed. Do this after item 5.
**Goal:** `.\scripts\coverage.ps1` fails when either line or branch coverage drops below its gate.

### Behaviour
- Without `-Branches`, the script still checks lines only, on stable Rust.
- With `-Branches`, or always if the nightly toolchain is installed (decide by checking `rustup toolchain list` for `nightly`), it also reads the branch total and fails below `$MinBranches`.
- With no nightly toolchain, it prints how to install one (`rustup toolchain install nightly --component llvm-tools-preview`) and fails, rather than skipping the check silently.

### Out of scope
- Raising branch coverage itself; set the gate to whatever the total is now.

### Implementation
1. `scripts/coverage.ps1`: after the line run, run `cargo +nightly llvm-cov report --branch --json --summary-only` and read `data[0].totals.branches.percent` with `ConvertFrom-Json`. Compare it with a new `$MinBranches` (the current whole number). `throw` with both numbers when it's below.
2. `scripts/release.ps1`: call `coverage.ps1 -Branches`, so a release checks both.

### Tests
- Run the script once with `$MinBranches` set one above the current total and confirm it fails with a message naming both numbers; then set it back.

### Docs
- `README.md`, *Tests* section: the branch gate and the nightly requirement. `CONTRIBUTING.md` step 4: add `-Branches`. Changelog: "Coverage gate checks branches as well as lines."

### Done when
- [ ] A branch total under `$MinBranches` fails the script (shown once on purpose).
- [ ] `.\scripts\release.ps1` runs the branch gate.

## 7. Run mutation testing on the core logic and strengthen weak tests

**Why:** coverage only shows that code ran, not that a test would notice it being wrong. The testing rules ask for occasional mutation testing of the core logic to find tests that execute code without really checking it. It has never been run on this project, and `cargo-mutants` isn't installed (`cargo install cargo-mutants`).
**Goal:** every mutant that cargo-mutants makes in the core logic is caught by a test, or is listed as equivalent (a change that can't alter behaviour), with the reason.

### Behaviour
- No change to the app.

### Out of scope
- `src/app/` (UI wiring, covered by `tests/ui.rs`) and `src/main.rs`.

### Implementation
1. Install with `cargo install cargo-mutants`.
2. Run it on the pure logic first, since each mutant rebuilds the crate: `cargo mutants --file src/gradebook/compute.rs --file src/lock.rs --file src/i18n/mod.rs -- --test compute --test lock --test i18n`. Strawberry Perl must be on `PATH` (dot-source `scripts/use-perl.ps1`). The first build compiles SQLCipher's OpenSSL in cargo-mutants' copy of the tree (about 10 minutes); later mutants rebuild incrementally.
3. For each survivor in `mutants.out/missed.txt`, add an assertion or a property test to that module's test file that fails on the mutant. Re-run until nothing is missed or every survivor is equivalent.
4. Then run it on `src/gradebook/columns.rs`, `src/gradebook/cells.rs`, `src/db/migrations.rs` and `src/storage.rs` the same way.
5. Add `mutants.out*/` to `.gitignore`.
6. Add a `.cargo/mutants.toml` with `exclude_globs = ["src/app/**", "src/main.rs"]` and record each equivalent mutant there with a comment, so later runs don't report them again.

### Tests
- New assertions and property tests in `tests/compute.rs`, `tests/lock.rs`, `tests/i18n.rs`, `tests/gradebook.rs`, `tests/db.rs` and `tests/storage.rs`, each killing a named survivor (say which in a comment).

### Docs
- `README.md`, *Tests* section: one line on running `cargo mutants` now and then, and where its config lives. Changelog: "Tests strengthened after mutation testing."

### Done when
- [ ] `cargo mutants` on the listed files reports no missed mutants apart from those in `.cargo/mutants.toml`.
- [ ] The default suite passes.

## 8. Make the desktop baselines work on other displays

**Why:** the 40 baselines in `tests/desktop/baselines/` were captured on one machine, at 100 % display scaling with its installed Segoe UI. `App::snapshot` (`tests/desktop_support/mod.rs`) names each file `<screen>-<look>@<scale>.png`. On a display at 125 % or 150 % there is no matching baseline, so every snapshot fails with "no baseline". On another machine, different font hinting would fail them too. The desktop suite only passes where it was recorded.
**Goal:** the desktop suite passes at 100 %, 125 % and 150 % scaling on this machine, and the README says exactly what to do on a new machine.

### Behaviour
- At a scale that has baselines, snapshots compare as today.
- At a scale without baselines, the snapshot tests fail with a message naming the scale and the exact command to record them (`$env:AVALIO_UPDATE_BASELINES = "1"; cargo test --test desktop -- --ignored --test-threads=1`). The pixel and position assertions still run.

### Out of scope
- Rendering in a fixed scale independent of Windows: the point is to test what users see.

### Implementation
1. `tests/desktop_support/mod.rs`, `App::snapshot`: when the baseline for the current scale is missing, fail with the message above. Keep it a failure, not a skip, per the no-silent-failure rule.
2. Record baselines at 125 % and 150 %: set Windows' display scaling (Settings → System → Display → Scale), run the suite with `AVALIO_UPDATE_BASELINES=1`, and open and review every new image for clipped, wrapped or cut-off text before keeping it. Watch Galician at 1000 × 600 logical pixels in particular.
3. The assertions in `tests/desktop.rs` use physical pixels from `App::locate`, which already scales by the driver's `scale`. Check that the blob-size thresholds in `image.blobs(…, min_area)` (200 and 300 px) still find the pills and buttons at 150 %, and scale them by `app.scale * app.scale` if not.

### Tests
- The full desktop suite at 100 %, 125 % and 150 %.

### Docs
- `README.md`, *Tests* section: baselines exist per display scale; how to record them for a new scale or machine, and to review every image first. Changelog: "Desktop test baselines for 125 % and 150 % display scaling."

### Done when
- [ ] `cargo test --test desktop -- --ignored --test-threads=1` passes at 100 %, 125 % and 150 %.
- [ ] Every new baseline was opened and reviewed.

## 9. Turn on the hook that loads matching memory rules into each prompt

**Why:** this one is outside the repo, in Claude Code's own setup. On 2026-10-28 Claude answered "what's the testing situation?" and "what's the coverage?" without opening `feedback_tests.md`, because `MEMORY.md` only shows one-line summaries of each rule. The hook `C:\Users\anxol\.claude\hooks\memory_inject.py` exists and works: given a prompt on stdin, it prints the full text of every memory file whose topic matches, and it logs to `memory_inject.log` next to it. But it isn't registered, and Claude was refused permission to edit `~/.claude/settings.json` itself (self-modification), so only the user can turn it on.
**Goal:** every prompt about tests, commits, releases, logging, i18n, the UI and so on arrives with the matching rules' full text, so they can't be skipped.

### Behaviour
- A prompt matching one of the hook's patterns gets the files listed for it (the `TOPICS` table in the script) added to Claude's context.
- A prompt matching nothing adds nothing.
- If the hook can't read its input or a memory file, it says so in the injected text and in `memory_inject.log`; it never fails the prompt.

### Out of scope
- Changing the topic table; extend `TOPICS` in the script when a new memory file is added.

### Implementation
1. `C:\Users\anxol\.claude\settings.json` (by the user, or by Claude with explicit permission): inside `"hooks"`, next to `"Stop"`, add `"UserPromptSubmit": [{ "hooks": [{ "type": "command", "command": "python \"$HOME/.claude/hooks/memory_inject.py\"" }] }]`.
2. Restart Claude Code so the hook loads.

### Tests
- Send "what's the coverage?" and check that `memory_inject.log` records `injected feedback_tests.md, feedback_regression_safety.md, feedback_gap_free.md`.
- Send "hello" and check that nothing new is logged.

### Docs
- None in this repo. `C:\Users\anxol\.claude\memory\feedback_apply_global_memory_proactively.md` already describes the hook.

### Done when
- [ ] `settings.json` has the `UserPromptSubmit` entry.
- [ ] The log shows an injection for a testing prompt and none for "hello".

## 10. Export a class sheet to an Excel file

**Why:** the user asked for "a way to export and import data". Today the only way to get marks out of the app is to copy them by hand: a sheet can't be saved as a spreadsheet to hand in, print or open in Excel. The user chose `.xlsx` for single sheets (and a custom `.avalio` file for the whole database, items 12–14). This is the first of five items; item 11 (import from Excel) builds on the file dialog and the spreadsheet module added here, so do this first.
**Goal:** the sheet's "⋯" menu has **Export to Excel…**. It opens a Save dialog, and the file it writes opens in Excel showing exactly the sheet on screen: pupils down the side, every column across the top, marks as real numbers (Excel shows them with the computer's decimal separator), plus a second tab listing each column's settings.

### Behaviour
- **Where:** a new `MenuOption` "Export to Excel…" in the sheet's "⋯" menu (`more-menu` in `ui/sheet-page.slint`, around line 300), placed after "Hide weights & scales" and before "Delete class", so the destructive entry stays last.
- **The Save dialog:**
  - opens in the folder of the last export in this session, and otherwise in the Windows Documents folder (`dirs::document_dir()`);
  - proposes `<class name>.xlsx`, with the characters Windows refuses in file names (`\ / : * ? " < > |`) replaced by `-`;
  - filters on "Excel workbook (*.xlsx)" / "Libro de Excel (*.xlsx)".
  - Picking an existing file: the Windows save dialog asks before replacing it (rfd turns `FOS_OVERWRITEPROMPT` on); the app doesn't ask a second time. The manual test checks this.
  - Cancel: nothing happens, no message.
- **First tab, named after the class:**
  - Excel limits tab names to 31 characters and refuses `[ ] : * ? / \`: those are replaced by `-` and the name is cut to 31 characters; an empty result becomes "Sheet" / "Folla".
  - Row 1 is the header, in bold with a bottom border: "Pupil" / "Alumno" in A1, then every column's title in sheet order.
  - One row per pupil, in sheet order. The pupil's name is text in column A.
  - **Info** cells: the text as typed.
  - **Test** cells: a valid score (what `compute::evaluate_row` doesn't mark `is_invalid`) is written as a number; an invalid one is written as the text that was typed, so nothing typed is lost; an empty cell stays empty.
  - **Final grade, average and rounded** cells: the number shown on screen (the `Evaluated.text` of `evaluate_row`, parsed back with `parse_number`), written as a number; blank where the screen is blank. Numbers are written in Excel's own format, so Excel shows "8,5" on a Galician Windows and "8.5" on an English one, whatever the app's language.
  - Row 1 and column A are frozen, so names and titles stay visible when scrolling. Column widths fit their contents (`Worksheet::autofit`).
  - When the normal tests' weights don't add up to 100 %, one line in italics two rows below the last pupil reads "Test weights add up to 90%, not 100%: final grades are provisional." / "Os pesos das probas suman 90%, non 100%: as notas finais son provisionais." It mirrors the warning on screen, so the file doesn't pass provisional grades off as final.
- **Second tab, "Columns" / "Columnas":** one row per sheet column, in sheet order, with a bold header: Column, Kind, Weight %, Maximum, Bonus, Rounds, Decimals (Galician: Columna, Tipo, Peso %, Máximo, Extra, Redondea, Decimais).
  - Kind names: Test / Info / Final grade / Average / Rounded (Galician: Proba / Información / Nota final / Media / Redondeo).
  - Weight % only for tests; Maximum for tests, final grades and averages; Bonus "Yes" / "Si" for bonus tests and blank otherwise; Rounds (the source column's title) and Decimals only for rounded columns. Numbers are numbers.
  - Without this tab, the file says "7" without saying out of what.
- **A class with no pupils or no columns** still exports: the header row alone, or the names alone.
- **When it works:** a green status banner on the sheet reads "Saved 1A to D:\…\1A.xlsx" / "Gardouse 1A en D:\…\1A.xlsx" (the class name and the full path). It clears on the next sheet action.
- **When it fails** (folder not writable, file open in Excel and locked): the red error banner on the sheet reads "Couldn't save D:\…\1A.xlsx: <reason>" / "Non se puido gardar D:\…\1A.xlsx: <motivo>", the log has a warning, and no half-written file is left (rust_xlsxwriter writes the whole file at `save`).

### Out of scope
- Importing (item 11), the whole database (items 12–14), CSV (the user chose xlsx only), exporting several classes into one workbook, cell colours for pass/fail.

### Implementation
1. **`Cargo.toml`:** `cargo add rust_xlsxwriter` (writes .xlsx; no C dependencies) and `cargo add --dev calamine` (reads it back in tests; item 11 moves it to `[dependencies]`). Both pin the current version.
2. **`src/app/dialogs.rs`:** replace `pick_database_file` with a general picker, so the database, spreadsheet and backup pickers (items 11–14) share one method instead of one per file type:
   - `pub enum FileKind { Database, Spreadsheet, Backup }` with `extensions()` (`["db"]`, `["xlsx"]` for now, `["avalio"]`) and `filter_name(Language)` built from `Message::DatabaseFileType` and two new messages (step 6).
   - `fn pick_file(&self, kind: FileKind) -> Option<PathBuf>` and `fn save_file(&self, kind: FileKind, start: &Path, file_name: &str) -> Option<PathBuf>` on the trait; `NativeDialogs` uses `FileDialog::add_filter`, `set_directory` (only when `start.is_dir()`, as `pick_folder` does), `set_file_name` and `save_file()`.
   - Update every caller (grep `pick_database_file`): `src/app/callbacks.rs` (two calls → `pick_file(FileKind::Database)`), `tests/common/mod.rs` `FakeDialogs` (also add `saves: RefCell<VecDeque<Option<PathBuf>>>`, `save_file_next()` and record the proposed name in `save_requests: RefCell<Vec<(PathBuf, String)>>`), `examples/app_driver.rs`, and the eight uses in `tests/ui.rs`.
3. **`src/spreadsheet/mod.rs` and `src/spreadsheet/export.rs` (new, `pub mod spreadsheet;` in `src/lib.rs`):** no Slint code, so it can be tested on its own.
   - `pub fn sheet_workbook(class_name: &str, sheet: &Sheet, language: Language) -> Result<Workbook>` builds both tabs from `Sheet` and `evaluate_row` / `weights_add_up` / `weight_total` from `src/gradebook/compute.rs`, so the numbers come from the same code as the screen (no second calculation to drift).
   - `pub fn export_sheet(connection, class_id, language, path) -> Result<()>` loads `classes::class_name` and `sheet::load_sheet`, builds the workbook and saves it; logs `log::info!("exported class {class_id} to {}", path.display())`.
   - `pub fn tab_name(text: &str, fallback: &str) -> String` and `pub fn file_name(text: &str) -> String` hold the two character rules from Behaviour, as pure functions.
4. **`src/error.rs`:** add `AppError::SpreadsheetWrite { path: PathBuf, reason: String }` (`#[error("could not write {path}: {reason}")]`), built from `rust_xlsxwriter::XlsxError` with its `Display` text. Add its English and Galician arms in `src/i18n/errors.rs` (wording in Behaviour).
5. **`src/i18n/messages.rs`:** add `SheetExported { class, path }`, `SpreadsheetFileType` ("Excel workbook" / "Libro de Excel"), `BackupFileType` ("avalio-lite backup" / "Copia de seguranza de avalio-lite", used from item 12), `ExportWeightsWarning { total }`, and `pub fn export_labels(Language) -> ExportLabels` returning the header and tab words from Behaviour plus `pub fn column_type_name(Language, ColumnType)`. Add every new message to `every_message()` in `tests/i18n.rs`.
6. **`ui/gradebook.slint`:** add `in property <string> status-message;` and `callback export-sheet();`. **`ui/sheet-page.slint`:** a `MessageBanner { message: Gradebook.status-message; is-error: false; }` right after the error banner at line 132, and the new `MenuOption` calling `Gradebook.export-sheet()`.
7. **`src/app/gradebook/mod.rs`:** make `run` clear `status-message` before the task and return `bool` (true on success), so a status from an earlier action never sits next to a new error; the existing callers ignore the result. Add `fn show_status(window, text)`.
8. **`src/app/gradebook/classes.rs`:** wire `on_export_sheet`: proposed name from `spreadsheet::export::file_name(&class_name)` + ".xlsx", start folder from a `Rc<RefCell<Option<PathBuf>>>` holding the last export folder (Documents when empty), `dialogs.save_file(FileKind::Spreadsheet, …)`, then `run(…, "export sheet", …)` with `Refresh::Nothing`, then the status on success.
9. **`lang/gl/LC_MESSAGES/avalio-lite.po`:** add "Export to Excel…" → "Exportar a Excel…".
10. **File headers:** `metadata -c` on the new `.rs` files, `metadata -m` on every edited `.rs`/`.md` file. Then `cargo fmt`, `cargo clippy --all-targets -- -D warnings`.

### Tests
- **`tests/spreadsheet.rs` (new)**, each test writes to a `tempfile` folder and reads the file back with calamine:
  - `exported_sheet_matches_the_screen`: a class with an info, two tests, a final grade, an average and a rounded column; every cell equals what `evaluate_row` shows, numbers are numeric cells (`Data::Float`), names and info are strings.
  - `invalid_scores_are_kept_as_text`: a score of "12" out of 10 and "abc" are exported as those strings.
  - `the_columns_tab_lists_every_setting`: weights, maxima, bonus, rounding source and decimals, per column.
  - `incomplete_weights_add_the_provisional_note`: weights at 90 % add the note row; at 100 % there is none.
  - `empty_class_exports_the_header_only` and `class_without_columns_exports_the_names`.
  - `galician_labels_are_used`: header "Alumno", tab "Columnas".
  - `tab_and_file_names_are_cleaned`: proptest over arbitrary strings: `tab_name` is 1–31 chars with none of `[]:*?/\`; `file_name` has none of `\/:*?"<>|`.
  - `a_locked_destination_is_reported`: with the destination held open without sharing (`OpenOptionsExt::share_mode(0)`, as Excel does with an open workbook), the export gives `SpreadsheetWrite`, not a panic.
- **`tests/golden.rs`:** `the_sheet_export_matches_the_golden_dump`: the export of `fill_current`'s class, read back and dumped as tab-separated text, equals `tests/golden/sheet-export.tsv` (rewritten with `AVALIO_UPDATE_GOLDEN=1`). The .xlsx bytes contain a timestamp, so the dump is compared, not the bytes.
- **`tests/ui.rs`:** `export_to_excel_saves_the_sheet_and_says_where` (scripted save path → file exists, green banner text, proposed name "1A.xlsx"); `a_cancelled_export_does_nothing`; `a_failed_export_shows_the_error`; `regression_the_export_status_clears_on_the_next_action`.
- **`tests/i18n.rs`:** the new messages in `every_message()`; the `.po` test covers the menu entry.
- **Manual (release build):** export a demo class (`cargo run --example demo_data -- <path> gl`), open it in Excel or LibreOffice, screenshot both tabs; export over an existing file and check Windows asks before replacing; screenshot the "⋯" menu and the green banner in light and dark, and in Galician at 1000 px wide.

### Docs
- `README.md`, "Using it": a line "**⋯ → Export to Excel…** saves the sheet as an .xlsx file: the marks as numbers, and a second tab with each column's settings."
- `README.md`, Layout block: add `spreadsheet/  Excel export and import of a class sheet`.
- Changelog: "Added Export to Excel… in the sheet's ⋯ menu: saves the sheet as an .xlsx workbook with each column's settings".

### Done when
- [ ] The exported file opens in Excel or LibreOffice with both tabs as described (screenshots).
- [ ] The menu entry and green banner look right in light, dark and Galician at 1000 px (screenshots).
- [ ] `cargo test`, `.\scripts\coverage.ps1`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass; desktop baselines that changed were rewritten and each changed image reviewed.

## 11. Import pupils and marks from an Excel file

**Why:** after item 10 a sheet can be exported, but nothing can come in: a class list from the school's software or a sheet of marks kept in Excel has to be typed in again by hand, pupil by pupil. Do this after item 10, which adds the file picker, the spreadsheet module and the sheet's status banner this item uses.
**Goal:** the sheet's "⋯" menu has **Import from Excel…**. Picking a workbook adds its pupils to the current class and fills in its marks under the columns with the same titles, then says what it did.

### Behaviour
- **Where:** "Import from Excel…" in the "⋯" menu, right after "Export to Excel…". The Open dialog filters on "Excel workbook" with `.xlsx`, `.xls` and `.ods` (calamine reads all three at no extra cost; LibreOffice users save `.ods`). Cancel does nothing.
- **What is read:** the first tab of the workbook only. Row 1 is the header row; every later row is one pupil. Column A holds the pupil names, whatever its header says. Rows whose name cell is empty are skipped. An exported file (item 10) imports back unchanged: its "Columns" tab is the second tab and is ignored.
- **Matching columns:** each header from column B on is matched to a column of the current class by title, ignoring case and spaces at both ends.
  - It matches the first info or test column with that title, in sheet order.
  - A header that matches a final grade, average or rounded column is ignored: those are calculated.
  - A header that matches nothing, an empty header, or a header repeated in the file (the second time) is ignored. Ignored headers are named in the result message. Creating columns from the file is out of scope.
- **Matching pupils**, per name (trimmed, compared ignoring case):
  - a pupil of this class with that name → that row;
  - otherwise exactly one pupil elsewhere in the database with that name → that pupil is added to this class (pupils are shared between classes; this is what "+ Add from another class" does);
  - otherwise (no such pupil, or several with the same name, where the app can't know which one is meant) → a new pupil, added at the end of the sheet in file order.
  - The same name twice in the file is refused before anything changes: "The file has "Ana Pérez" twice; each pupil can only be in it once." / "O ficheiro ten «Ana Pérez» dúas veces; cada alumno só pode estar nel unha vez." Merging the two rows silently could lose a mark.
- **Values:**
  - Excel numbers are stored as typed text without a locale (`8`, `7.5`, `7.125`: Rust's shortest `f64` form); text cells are stored trimmed; true/false as "TRUE"/"FALSE"; dates as `YYYY-MM-DD`.
  - A value that isn't a valid score for a test column is still stored, exactly as typing it does: the cell is then shaded red on the sheet. Refusing the whole file for one typo would be worse.
  - An empty cell in the file leaves the sheet's cell as it is, so a file with only some marks never erases others.
  - Excel error cells (`#DIV/0!`, `#N/A`) are treated as empty and counted in the result message.
- **Overwriting:** when the import would replace marks or notes that already differ from the file's, one confirmation lists how many first: "Importing replaces 12 marks or notes already in 1A. Continue?" / "Ao importar substitúense 12 notas ou observacións que xa hai en 1A. Continuar?". No → nothing changes. With nothing to replace, there is no question.
- **All or nothing:** the whole import runs in one transaction; any error rolls everything back and shows the red banner.
- **Errors**, each in the red banner, each logged as a warning, nothing changed:
  - the file can't be read or isn't a workbook: "Couldn't read D:\…\x.xlsx: <reason>" / "Non se puido ler D:\…\x.xlsx: <motivo>";
  - no pupils (only a header, or an empty first tab): "D:\…\x.xlsx has no pupils: names go in column A, from row 2." / "D:\…\x.xlsx non ten alumnos: os nomes van na columna A, desde a fila 2.";
  - the duplicate name above.
- **When it works**, the green banner reads, for example: "Imported 24 pupils (3 new) and 72 marks or notes into 1A. Left out: Phone, Final grade." / "Importáronse 24 alumnos (3 novos) e 72 notas ou observacións en 1A. Quedaron fóra: Teléfono, Nota final.". The "Left out" sentence only appears when headers were ignored; "2 cells with Excel errors were skipped." / "Saltáronse 2 celas con erros de Excel." only when there were some. The sheet and the class list (pupil counts) redraw.

### Out of scope
- Creating columns from unknown headers, choosing which tab to import, CSV, importing into several classes at once, removing pupils that aren't in the file.

### Implementation
1. **`Cargo.toml`:** move `calamine` from `[dev-dependencies]` to `[dependencies]`. **`src/app/dialogs.rs`:** `FileKind::Spreadsheet.extensions()` becomes `["xlsx", "xls", "ods"]` for opening; the save dialog keeps `["xlsx"]`. Add `FileKind::open_extensions()` / `save_extensions()` so the rule is in one place.
2. **`src/gradebook/pupils.rs`:**
   - Split `add_new_pupil` into `insert_new_pupil(connection, class_id, name) -> Result<i64>` (no transaction of its own) and `add_new_pupil`, which wraps it in its transaction as now. The import runs inside its own transaction, and rusqlite refuses a transaction inside another.
   - Add `pub fn pupils_named(connection, name) -> Result<Vec<Pupil>>` (`WHERE name = ?1 COLLATE NOCASE` after trimming). Item 14 uses it too.
3. **`src/spreadsheet/import.rs` (new):**
   - `pub struct ImportTable { headers: Vec<String>, rows: Vec<ImportRow>, error_cells: usize }`, `pub struct ImportRow { name: String, values: Vec<Option<String>> }`.
   - `pub fn read_table(path) -> Result<ImportTable>` with `calamine::open_workbook_auto`, the first worksheet, and the value rules from Behaviour (a private `cell_text(&Data) -> Option<String>`).
   - `pub fn plan_import(connection, class_id, &ImportTable) -> Result<ImportPlan>`: column matching, pupil matching, the duplicate check, and the counts (pupils, new, marks, overwrites, left-out headers). No writes, so the confirmation can be asked before anything changes.
   - `pub fn apply_import(connection, class_id, &ImportPlan) -> Result<ImportSummary>`: one `unchecked_transaction`, `insert_new_pupil` / `add_existing_pupil` / `cells::set_cell` (reusing them keeps every rule and trigger of normal typing), commit, `log::info!` with the counts.
4. **`src/error.rs` + `src/i18n/errors.rs`:** `SpreadsheetRead { path, reason }`, `NoPupilsInFile(PathBuf)`, `DuplicatePupilInFile(String)`, English and Galician as in Behaviour.
5. **`src/i18n/messages.rs`:** `ConfirmImportOverwrite { count, class }`, `ImportDone { pupils, new_pupils, marks, class }`, `ImportLeftOut { headers }`, `ImportErrorCells { count }`; plural forms per language with a `match` on the count (1 → "1 pupil" / "1 alumno"). Add them to `every_message()` in `tests/i18n.rs`.
6. **`ui/gradebook.slint`:** `callback import-sheet();`. **`ui/sheet-page.slint`:** the menu entry. **`lang/gl/…/avalio-lite.po`:** "Import from Excel…" → "Importar de Excel…".
7. **`src/app/gradebook/classes.rs`:** wire `on_import_sheet`: `dialogs.pick_file(FileKind::Spreadsheet)` → `read_table` → `plan_import` → `dialogs.confirm` when `overwrites > 0` → `run(…, "import sheet", …)` with `Refresh::All { select: Some(class_id) }` → the status text joined from the messages above. Errors from `read_table`/`plan_import` go through `run` too, so they reach the banner and the log the same way.
8. Headers, fmt, clippy as in item 10.

### Tests
- **`tests/spreadsheet.rs`:** fixtures are written with rust_xlsxwriter in the test, so they're readable in the diff.
  - `an_exported_sheet_imports_back_unchanged`: export (item 10) → clear the marks → import → the sheet equals the original.
  - `columns_match_by_title_ignoring_case_and_spaces`; `calculated_and_unknown_headers_are_left_out_and_named`; `a_repeated_header_uses_the_first`.
  - `pupils_match_in_class_then_elsewhere_then_new`: one in the class, one unique in another class (enrolled, same id), two same-named elsewhere (a new pupil), one unknown (new).
  - `a_name_twice_in_the_file_is_refused_and_changes_nothing`.
  - `empty_file_cells_keep_existing_values`; `excel_numbers_dates_and_booleans_become_text`; `error_cells_are_counted_and_skipped`.
  - `invalid_scores_are_stored_like_typing`: "12" into a test out of 10 is stored and evaluates `is_invalid`.
  - `a_header_only_file_has_no_pupils`; `a_text_file_renamed_xlsx_is_a_read_error`.
  - `a_failure_mid_import_rolls_everything_back`: a plan whose last write fails (column deleted between plan and apply) leaves the database as before.
  - `ods_files_import`: a small `.ods` fixture in `tests/golden/import.ods`, made once in LibreOffice and never rewritten.
  - proptest `plan_counts_match_what_apply_writes`: random tables against a seeded class; the summary's counts equal the rows and cells actually changed.
- **`tests/ui.rs`:** `import_from_excel_fills_the_sheet_and_reports`; `overwriting_asks_first_and_no_changes_nothing`; `a_cancelled_import_does_nothing`; `an_unreadable_file_shows_the_error`.
- **Manual:** import a list saved from Excel with accents ("Uxía Núñez") and a Galician decimal comma typed as text ("7,5"); screenshot the green banner in light, dark and Galician at 1000 px.

### Docs
- `README.md`, "Using it": "**⋯ → Import from Excel…** adds the pupils in column A of a workbook (.xlsx, .xls or .ods) and fills the marks under columns with the same titles; it asks before replacing marks already there."
- Changelog: "Added Import from Excel… in the sheet's ⋯ menu: adds pupils and fills marks from an .xlsx, .xls or .ods file".

### Done when
- [ ] Round trip export → import gives the same sheet (test output).
- [ ] A real Excel-made list imports with accents intact (screenshot of the sheet and banner).
- [ ] The overwrite question appears only when values would change (UI test names).
- [ ] `cargo test`, coverage gate, clippy and fmt pass.

## 12. Export everything to an .avalio backup file

**Why:** the only backup today is "copy `avalio-lite.db` while the app is closed" (README "Your data", and the Database card's description in `ui/settings-page.slint` line 167). That needs the app closed, a trip to Explorer, and knowing where the file is. The user asked for "a custom file type for the whole thing". This item adds the export; items 13 (restore) and 14 (add classes from a backup) read it, so do it before them.
**Goal:** Settings has a **Backup** card with **Export everything…**: one click saves every class, pupil, column and mark into a single `.avalio` file, while the app stays open.

### Behaviour
- **The Backup card:** in `ui/settings-page.slint`, right after the Database card. Title "Backup" / "Copia de seguranza". Description: "Save everything (every class, pupil, column and mark) in one .avalio file, to keep it safe or take it to another computer. With a PIN, the file is encrypted with it." / "Garda todo (cada clase, alumno, columna e nota) nun ficheiro .avalio, para telo a salvo ou levalo a outro ordenador. Cun PIN, o ficheiro cífrase con el.". One primary `ActionButton` "Export everything…" / "Exportar todo…". Items 13 and 14 add two ghost buttons to the same row.
- **The Database card description** becomes "Every class, pupil and mark is in this file." / "Cada clase, alumno e nota está neste ficheiro.", because backups now have their own card.
- **The Save dialog:** filter "avalio-lite backup (*.avalio)", proposed name `avalio-lite-backup-YYYY-MM-DD.avalio` (today's local date, so successive backups don't overwrite each other by default), opened in the last backup folder of the session, else Documents. Windows asks before replacing an existing file. Cancel does nothing.
- **What the file is:** a complete copy of the database, in the same SQLite format as `avalio-lite.db`, with its schema version. Reusing the format means the existing, tested `db::export` and `db::verify` do the work and the migrations upgrade old backups on restore, instead of a second format to keep in step with the schema. The `.avalio` extension says "backup, not the live database", so it isn't confused with `avalio-lite.db` or opened as the working file by mistake.
- **Encryption:** when the database has a PIN, the backup is encrypted with the same PIN (otherwise the export would leak exactly what the PIN protects); without a PIN it isn't encrypted. The card description says so. Restoring it later needs the PIN it was made with (item 13).
- **Safe writing:** the copy is written next to the destination as `<name>.avalio.partial`, verified (opens with the key, passes `PRAGMA integrity_check`), and only then renamed onto the chosen name. A failure leaves no `.partial` file and never touches an existing file of the chosen name.
- **When it works:** the green banner on Settings reads "Backup saved to D:\…\avalio-lite-backup-2026-10-28.avalio" / "Copia gardada en D:\…". The log has an info line with the path.
- **When it fails:** the red banner on Settings with the existing I/O or database error text; the log has a warning.

### Out of scope
- Restoring (item 13), adding classes from a backup (item 14), automatic or scheduled backups, a different password for the backup than the PIN.

### Implementation
1. **`src/backup.rs` (new, `pub mod backup;` in `src/lib.rs`):**
   - `pub const EXTENSION: &str = "avalio";`
   - `pub fn default_file_name(date: chrono::NaiveDate) -> String`.
   - `pub fn export(connection: &Connection, destination: &Path, key: Option<&str>) -> Result<()>`: the `.partial` path, `db::remove_files` on any leftover, `db::export`, `db::verify`, `fs::rename` onto `destination` (replaces an existing file on Windows), and on any error remove the `.partial` file (log, don't hide, a failure to remove it, as `protection::discard` does).
2. **`src/app/state/mod.rs`:** `pub fn export_backup(&self, destination: &Path) -> Result<()>` calling `backup::export` with `self.key`. The key stays inside `AppState`, never handed to the UI layer.
3. **`src/app/dialogs.rs`:** `FileKind::Backup` with `save_extensions()` `["avalio"]` and the `BackupFileType` filter name (added in item 10).
4. **`src/i18n/messages.rs`:** `BackupExported { path }`; add it to `every_message()` in `tests/i18n.rs`.
5. **`ui/app-window.slint`:** `callback export-backup();`, passed through `Shell` to `SettingsPage` as `move-database` is. **`ui/settings-page.slint`:** the Backup card and the new Database description.
6. **`src/app/callbacks.rs`:** in `wire_settings`, wire `on_export_backup`: `dialogs.save_file(FileKind::Backup, start, &backup::default_file_name(Local::now().date_naive()))` → `state.borrow().export_backup(&path)` → `view::show_status` or log + `view::show_error`.
7. **`lang/gl/…/avalio-lite.po`:** the card title, description and button; replace the old Database description msgid (the i18n test fails on the stale one).
8. Headers (`metadata -c src/backup.rs`), fmt, clippy.

### Tests
- **`tests/backup.rs` (new):**
  - `a_backup_holds_everything`: export a filled database, open the backup with `db::open_existing`, every class, pupil, column and cell matches.
  - `a_backup_of_an_encrypted_database_needs_its_pin`: opens with the PIN, `WrongPin` without it or with another.
  - `a_backup_replaces_an_existing_file_only_once_complete`: with a directory of the chosen name at the destination, the final rename fails: the error is returned, the directory is unchanged and no `.partial` remains; with an existing backup file at the destination, a successful export replaces it.
  - `the_default_name_carries_the_date`: `2026-10-28` → `avalio-lite-backup-2026-10-28.avalio`.
  - `a_backup_keeps_the_schema_version`: `user_version` of the file equals `migrations::latest_version()`.
- **`tests/ui.rs`:** `export_everything_saves_a_backup_and_says_where`; `a_cancelled_backup_does_nothing`; `a_failed_backup_shows_the_error`.
- **`tests/i18n.rs`:** the new message and `.po` entries.
- **Desktop baselines:** the Settings screens change; rewrite `settings-*` and `settings-folders-*` with `AVALIO_UPDATE_BASELINES=1` and open every changed image to check nothing is clipped, especially Galician at 1000 px.

### Docs
- `README.md`, "Your data": replace "To back up, copy `avalio-lite.db` while the app is closed." with "To back up, use **Settings → Backup → Export everything…**: it saves one `.avalio` file (encrypted with your PIN, if you set one) while the app stays open."
- `README.md`, Layout: `backup.rs     .avalio backup files`.
- `project_spec.md` (project memory): a line on backups, only after the user confirms (spec rule).
- Changelog: "Added Settings → Backup → Export everything…: saves the whole database to one .avalio file, encrypted with the PIN when there is one".

### Done when
- [ ] A backup made with the app open restores the same data (test) and is encrypted when a PIN is set (test).
- [ ] The Backup card in light, dark and Galician at 1000 px (screenshots), baselines rewritten and reviewed.
- [ ] `cargo test`, coverage gate, clippy and fmt pass.

## 13. Restore everything from an .avalio backup

**Why:** after item 12 a backup can be made, but bringing it back means knowing that a backup is a database file and using "Open another database…", which switches the app to working in the backup file itself. Do this after item 12; item 14 reuses the PIN prompt and the backup opening added here.
**Goal:** **Settings → Backup → Restore from a backup…** replaces everything in the app with the backup's contents, after one clear warning, and keeps the current PIN.

### Behaviour
- **Where:** a ghost `ActionButton` "Restore from a backup…" / "Restaurar unha copia…" after "Export everything…" in the Backup card.
- **Picking:** an Open dialog filtered on "avalio-lite backup" with `.avalio` and `.db` (a plain database file, such as a copied `avalio-lite.db`, is the same format and works too). Cancel does nothing.
- **Opening the backup:** the chosen file is never changed. It is copied to `<database path>.restore` and the copy is opened, so the migrations can upgrade an older backup without touching the original.
  - The app tries the current PIN first (the common case: a backup of this same database), then no PIN.
  - If neither works, the Backup card shows, inline (no dialog, per the inline-editing preference): "This backup is encrypted. Type the PIN it was made with." / "Esta copia está cifrada. Escribe o PIN co que se fixo.", a `PinField` "Backup PIN" / "PIN da copia", a primary "Continue" / "Continuar" and a ghost "Cancel" / "Cancelar". Enter in the field is Continue.
  - A wrong PIN shows "Wrong PIN" under the field (the existing `WrongPin` text) and keeps the prompt open. There is no wait after several tries, unlike the lock screen: the backup file can be attacked offline anyway, so a wait would protect nothing.
  - Cancel, leaving Settings, or locking the app hides the prompt and deletes the `.restore` copy.
- **Errors before anything changes:** a file that isn't a database ("Couldn't read …" with the database error), a backup from a newer version (the existing `NewerSchema` text), or one that fails its integrity check (the existing `IntegrityCheckFailed` text): red banner, log warning, `.restore` copy deleted, the app's data untouched.
- **The warning:** once the backup opens, one confirmation: "Replace everything in the app with the backup avalio-lite-backup-2026-10-28.avalio (4 classes, 96 pupils)?\n\nEvery class, pupil and mark in the app now is replaced. This can't be undone. Export a backup first if you might need them." / Galician equivalent in step 4. No → nothing changes. The counts tell the user they picked the right file.
- **Replacing:** the backup's contents are written as a fresh file next to the database, encrypted with the **current** PIN (or not, if the app has none), verified, then swapped in for the database exactly as changing the PIN does today (`protection::swap_in`). The PIN belongs to this computer's copy, so a restore never turns encryption on, off or onto another PIN behind the user's back. A failure at any step leaves the current database complete and in use.
- **After:** the sheet shows the restored classes (the first class selected), the green banner on Settings reads "Restored the backup avalio-lite-backup-2026-10-28.avalio." / "Restaurouse a copia …", the `.restore` copy is deleted, and the log has an info line.

### Out of scope
- Adding a backup's classes without replacing (item 14), choosing which classes to restore, keeping the backup's own PIN.

### Implementation
1. **`src/app/state/protection.rs`:** pull the body of `rekey` into `fn replace_with(&mut self, source: &Connection, key: Option<&str>) -> Result<()>` (export `source` to the fresh file with `key`, verify, close, `swap_in`, reopen with `key`). `rekey(new_key)` becomes `replace_with(own connection, new_key)`, so restoring and re-keying share one tested path. Borrowing note: `rekey` passes `self.connection` while `self` is mutably borrowed, so take the connection out with `Option::take` for the export step, or export before the mutable part; keep the "a failure leaves one complete database" guarantee and its existing tests green.
2. **`src/backup.rs`:**
   - `pub fn open_copy(file: &Path, scratch: &Path, keys: &[Option<&str>]) -> Result<Connection>`: `fs::copy` to `scratch` (after `db::remove_files`), then `db::verify` + `db::open` with each key in turn; `WrongPin` only when all fail; delete the copy on any error.
   - `pub fn contents(connection) -> Result<BackupContents { classes: usize, pupils: usize }>` for the question.
3. **`src/app/state/mod.rs`:** `pub fn open_backup(&self, file, pin: Option<&str>) -> Result<OpenedBackup>` (keys: `[pin]` when a PIN was typed, else `[self.key, None]`; scratch `<database>.restore`) and `pub fn restore_backup(&mut self, backup: OpenedBackup) -> Result<()>` (calls `replace_with` with `self.key`, then closes and deletes the copy, also on failure). `OpenedBackup` owns the connection and the scratch path and deletes the scratch file in `Drop`, so no path can leave it behind.
4. **`src/i18n/messages.rs`:** `ConfirmRestore { file, classes, pupils }` (Galician: "Substituír todo o que hai na aplicación pola copia {file} ({classes} clases, {pupils} alumnos)?\n\nSubstitúense todas as clases, alumnos e notas que hai agora. Non se pode desfacer. Exporta antes unha copia se as podes necesitar.") and `BackupRestored { file }`; add both to `every_message()`.
5. **`ui/app-window.slint` / `ui/shell.slint` / `ui/settings-page.slint`:** `callback restore-backup()`, `in property <bool> backup-pin-needed`, `in-out property <string> backup-pin`, `in property <string> backup-pin-error`, `callback backup-pin-entered()`, `callback backup-pin-cancelled()`, and the inline prompt in the Backup card, reusing `PinField` from the PIN card (`ui/settings-page.slint` line 52).
6. **`src/app/backup.rs` (new, `mod backup;` in `src/app/mod.rs`, wired from `open` next to the other `wire` calls):** holds the pending action in `Rc<RefCell<Option<Pending>>>` (`Pending::Restore(PathBuf)`; item 14 adds `AddClasses`). `on_restore_backup` picks, opens (or shows the prompt), confirms, restores, then `gradebook::show_database` and `view::show_status`. Clear the prompt and drop the pending action in `lock_screen` when locking and when the section changes. Moving this out of `callbacks.rs` keeps that file to setup and folder actions.
7. **`lang/gl/…/avalio-lite.po`:** every new `@tr` string. Headers, fmt, clippy.

### Tests
- **`tests/backup.rs`:**
  - `restoring_replaces_everything`: database A, backup of B, restore → A's connection shows B's data.
  - `restoring_keeps_the_current_pin`: encrypted A (PIN 1111), unencrypted backup → after restore the database opens with 1111 only; and the reverse (plain A, backup with 2222 → plain after restore).
  - `an_older_backup_is_upgraded_and_the_file_is_untouched`: restore `tests/golden/db-v1.db`; the golden file's bytes (`fs::read` before and after) are unchanged and the restored data is complete.
  - `a_newer_backup_is_refused` (set `user_version` past `latest_version()` on a copy).
  - `a_failed_swap_keeps_the_current_database` (reuse the forced-failure setup of the existing re-key tests in `tests/encryption.rs`).
  - `the_restore_copy_never_stays_behind`: after success, wrong PIN, cancel and every error above, `<database>.restore` doesn't exist.
- **`tests/ui.rs`:** `restore_asks_once_and_replaces_the_sheet`; `no_to_the_restore_question_changes_nothing`; `an_encrypted_backup_asks_for_its_pin_inline`; `a_wrong_backup_pin_keeps_the_prompt`; `locking_closes_the_backup_prompt`.
- **Manual:** restore a backup made with a different PIN; screenshot the inline prompt and the green banner in light, dark and Galician at 1000 px; rewrite and review the Settings desktop baselines.

### Docs
- `README.md`, "Your data": "**Restore from a backup…** replaces everything in the app with a backup (it asks first, and keeps your current PIN)."
- Changelog: "Added Settings → Backup → Restore from a backup…: replaces the app's data with an .avalio backup, keeping the current PIN".

### Done when
- [ ] Restore works from a plain backup, one with the same PIN and one with a different PIN (UI tests + screenshots of the prompt).
- [ ] The current database survives every failure path, and no `.restore` file is left (test names).
- [ ] `cargo test`, coverage gate, clippy and fmt pass; Settings baselines reviewed.

## 14. Add the classes of an .avalio backup to the app

**Why:** after item 13 a backup can only replace everything. Bringing a colleague's classes, or last year's classes from another computer, into the current data isn't possible without losing what's there. Do this after items 11 (pupil matching, `insert_new_pupil`) and 13 (opening a backup and its PIN prompt).
**Goal:** **Settings → Backup → Add classes from a backup…** copies every class of a backup, with its pupils, columns and marks, into the app, keeping everything already there.

### Behaviour
- **Where:** a ghost `ActionButton` "Add classes from a backup…" / "Engadir clases dunha copia…" after "Restore from a backup…". The same Open dialog, opening and inline PIN prompt as item 13. No confirmation: nothing is deleted or replaced.
- **Classes:** every class of the backup is added as a new class with all its columns (titles, kinds, weights, maxima, bonus, rounding source and decimals, in the same order) and every typed cell. A name already used here gets " (2)", or the lowest free number from 2 up (" (3)"…), since class names are unique (`DuplicateClassName`).
- **Pupils:** matched by the same rule as the Excel import (item 11): exactly one pupil here with that name (ignoring case and outer spaces) → that pupil joins the new class; none, or several of the same name → a new pupil. A pupil shared by several classes in the backup stays one pupil in all of them here. Row order follows the backup.
- **All or nothing:** one transaction; any error rolls back and shows the red banner; nothing added.
- **An empty backup** (no classes): the green banner says "The backup has no classes; nothing was added." / "A copia non ten clases; non se engadiu nada." It's not an error.
- **When it works:** the class list shows the new classes, the first added class is selected, and the green banner reads "Added 3 classes and 58 pupils (12 were already here) from avalio-lite-backup-2026-10-28.avalio." / "Engadíronse 3 clases e 58 alumnos (12 xa estaban) desde …". Renamed classes are named: "Renamed because the name was taken: 1A → 1A (2)." / "Renomeadas porque o nome xa existía: 1A → 1A (2).".
- The current PIN and encryption are unchanged.

### Out of scope
- Choosing which classes to add, merging a backup class into an existing class of the same name, updating marks of classes already here.

### Implementation
1. **`src/gradebook/columns.rs`:** extract the `INSERT INTO sheet_columns` of `add_column` into `pub fn insert_column(connection, class_id, title, kind: ColumnKind) -> Result<i64>` (position at the end, settings via the existing `kind_fields`); `add_column` calls it. Rounded columns are inserted with `source: None` first and get their source afterwards through the existing `set_round_source`, once every column id is mapped, since a rounded column may round a column to its right.
2. **`src/gradebook/classes.rs`:** `pub fn free_class_name(connection, name) -> Result<String>` (the " (n)" rule).
3. **`src/backup.rs`:** `pub fn add_classes(target: &Connection, backup: &Connection) -> Result<AddSummary { classes, pupils, existing_pupils, renamed: Vec<(String, String)> }>`. Read with the existing `list_classes`, `load_sheet` (columns, pupils in order, typed values); write in one `unchecked_transaction` with `create_class`, `insert_column`, `pupils_named` + `insert_new_pupil` / `add_existing_pupil` (a `HashMap<i64, i64>` from backup pupil id to target id, filled once per pupil), and `cells::set_cell` for every non-empty typed value. Reading through the gradebook functions means older backups are already migrated (item 13 opens the copy with `db::open`).
4. **`src/app/state/mod.rs`:** `pub fn add_backup_classes(&self, backup: OpenedBackup) -> Result<AddSummary>`.
5. **`src/i18n/messages.rs`:** `BackupClassesAdded { classes, pupils, existing, file }`, `BackupClassesRenamed { pairs }`, `BackupHasNoClasses`, with plurals; add them to `every_message()`.
6. **`ui/…`:** `callback add-backup-classes()` through `AppWindow` → `Shell` → `SettingsPage`, and the button. **`src/app/backup.rs`:** `Pending::AddClasses(PathBuf)` and its handler; after success, `gradebook::show_database` then select the first added class. **`.po`:** the new strings.
7. Headers, fmt, clippy.

### Tests
- **`tests/backup.rs`:**
  - `added_classes_carry_columns_pupils_and_marks`: every column setting, row order and typed value matches the backup's class; calculated cells evaluate the same.
  - `taken_names_get_the_lowest_free_number`: "1A" and "1A (2)" exist → "1A (3)".
  - `pupils_match_by_unique_name_and_shared_pupils_stay_shared`.
  - `rounded_columns_keep_their_source_even_to_the_right`.
  - `a_failure_adds_nothing`: the test adds `CREATE TEMP TRIGGER … BEFORE INSERT ON cells WHEN (SELECT COUNT(*) FROM cells) >= <n> BEGIN SELECT RAISE(ABORT, 'test'); END` on the target, so the add fails half-way; afterwards the class, pupil and cell counts equal those before.
  - `an_empty_backup_adds_nothing_and_says_so`.
  - `an_older_backup_adds_its_classes`: `tests/golden/db-v1.db`.
  - proptest `adding_a_backup_of_itself_doubles_the_classes`: for a random database, adding its own backup gives every class twice (with " (2)") and no new pupils.
- **`tests/ui.rs`:** `add_classes_from_a_backup_lists_them_and_reports`; `adding_from_an_encrypted_backup_asks_for_its_pin`.
- **Manual:** add a demo backup (`demo_data`) into a database with an overlapping class name; screenshot the class list and banner in light, dark and Galician at 1000 px.

### Docs
- `README.md`, "Your data": "**Add classes from a backup…** copies a backup's classes, pupils and marks into the app without changing what's already there; pupils with the same name are joined."
- Changelog: "Added Settings → Backup → Add classes from a backup…: copies another backup's classes into the app".

### Done when
- [ ] Adding a backup keeps every existing class and adds the backup's ones with all their data (test names).
- [ ] Name clashes and shared pupils behave as described (test names).
- [ ] Screenshots in light, dark and Galician at 1000 px; Settings baselines reviewed.
- [ ] `cargo test`, coverage gate, clippy and fmt pass.

## 15. Duplicate a class, with or without its pupils and marks

**Why:** a teacher who starts a new term, or teaches the same subject to another group, has to rebuild a class's sheet column by column: titles, weights, maxima, bonus tests, final grade, roundings. Nothing copies a class today; the class "⋯" menu (`ui/sheet-page.slint`, `more-menu`) only has "Add from another class…", "Hide/Show weights & scales" and "Delete class". The user asked (2026-10-28) to choose each time whether the copy brings the pupils, and whether it brings their marks. This item shares two helpers with item 14 (`columns::insert_column`, `classes::free_class_name`): whichever item is done first adds them as described in item 14's steps 1–2, and the other reuses them.
**Goal:** the class "⋯" menu has three entries, **Duplicate class**, **Duplicate with pupils** and **Duplicate with pupils and marks**; choosing one creates the copy, selects it and shows its sheet.

### Behaviour
- **Where:** three `MenuOption`s in the "⋯" (More actions) menu of a class, right after "Add from another class…" (or first, when that entry is hidden) and before "Hide weights & scales". Each has a hint line, like the add-column menu:
  - "Duplicate class" — "Same columns and settings, no pupils" / gl "Duplicar clase" — "As mesmas columnas e axustes, sen alumnos"
  - "Duplicate with pupils" — "Same columns and pupils, empty marks" / gl "Duplicar con alumnos" — "As mesmas columnas e alumnos, sen notas"
  - "Duplicate with pupils and marks" — "An exact copy" / gl "Duplicar con alumnos e notas" — "Unha copia exacta"
  No dialog and no confirmation (the user prefers in-place UI, and nothing is deleted); the copy can be renamed in place and deleted from the same menu.
- **Name:** the original name plus " (2)", or the lowest free number from 2 up (" (3)"…), because class names are unique (`AppError::DuplicateClassName`). "1A" → "1A (2)"; if "1A (2)" exists → "1A (3)". The name is not translated, so it reads the same in both languages.
- **Columns (all three entries):** every column in the same order with the same title, kind, weight, maximum, bonus flag, decimals and rounding source. A rounded column's source points at the copy's own column, never back at the original class, including a source to its right.
- **Pupils (second and third entries):** the same pupils, in the same row order. Pupils are shared between classes (one pupil, many classes; see `src/gradebook/pupils.rs`), so no pupil is created: renaming one later renames them in both classes, and removing them from the copy leaves the original untouched.
- **Marks (third entry only):** every typed info and test cell is copied. Calculated columns store nothing, so the copy's final grades, averages and roundings are recalculated and match the original's.
- **All or nothing:** the copy is written in one transaction; on any error nothing is added and the red banner shows the error (`run` in `src/app/gradebook/mod.rs` already does this).
- **After it works:** the class list shows the copy (sorted by name, as always) and the copy is selected, with its sheet shown. The original is unchanged.
- An empty class (no columns, no pupils) duplicates to an empty class; the entries are always enabled.

### Out of scope
- Copying into an existing class, choosing which columns to copy, and duplicating several classes at once.

### Implementation
1. **`src/gradebook/columns.rs`:** if item 14 isn't done yet, extract `insert_column` exactly as item 14 step 1 says (`add_column` calls it).
2. **`src/gradebook/classes.rs`:** if item 14 isn't done yet, add `free_class_name` as item 14 step 2 says.
3. **`src/gradebook/model.rs`:** `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum DuplicateContents { Columns, ColumnsAndPupils, Everything }`, with a doc line per variant. An enum rather than two booleans, so "marks without pupils" can't be asked for.
4. **`src/gradebook/classes.rs`:** `pub fn duplicate_class(connection: &Connection, class_id: i64, contents: DuplicateContents) -> Result<i64>`. In one `unchecked_transaction`:
   1. `free_class_name(class_name(..))`, then `create_class`.
   2. For each column of `columns::list_columns` (in order): `insert_column` into the new class, with a rounded column's source set to `None` for now; keep a `HashMap<i64, i64>` from old to new column id.
   3. For each rounded column with a source: `columns::set_round_source(new_id, map[source])`. This comes after step 2 because the source may sit to the right.
   4. Unless `Columns`: `pupils::add_existing_pupil` for every pupil of `pupils::list_class_pupils`, in that order (this keeps the row order, since `enroll` appends).
   5. If `Everything`: `sheet::load_sheet` the original, and `cells::set_cell(map[column], pupil, value)` for every non-empty value of an info or test column. This works with migration 003's trigger, because the pupils were enrolled in step 4.
   6. Commit; `log::info!("duplicated class {class_id} as {new_id} ({contents:?})")`. Return the new id.
   An error at any step returns before the commit, so the transaction rolls back.
5. **`ui/gradebook.slint`:** `export enum DuplicateContents { columns, columns-and-pupils, everything }` and `callback duplicate-class(DuplicateContents);`. Re-export the enum in `ui/app-window.slint`'s `export { … }` list, as `ColumnType` is.
6. **`ui/sheet-page.slint`, `more-menu`:** the three `MenuOption`s with the titles and hints above; each `clicked => { Gradebook.duplicate-class(DuplicateContents.…); }`.
7. **`src/app/gradebook/view.rs`:** `pub fn from_ui_contents(ui::DuplicateContents) -> DuplicateContents`, beside `from_ui_type`.
8. **`src/app/gradebook/classes.rs`:** `gradebook.on_duplicate_class(…)` → `run(&window, &shared, "duplicate class", |connection| { let copy = classes::duplicate_class(connection, current_class_id(&window), from_ui_contents(contents))?; Ok(Refresh::All { select: Some(copy) }) })`. `run` already logs failures as WARN and shows the banner, so there's no silent failure.
9. **`lang/gl/LC_MESSAGES/avalio-lite.po`:** the six strings above. `tests/i18n.rs` fails until all are present.
10. `metadata -m` on every edited `.rs`/`.md` file (`.slint` and `.po` aren't supported), `cargo fmt`, `cargo clippy --all-targets -- -D warnings`.

### Tests
- **`tests/gradebook.rs`:**
  - `duplicating_copies_every_column_setting_in_order`: a class with info, test (weight, max, bonus), final grade, average and a rounded column (decimals 1); the copy's `list_columns` equals the original's in everything but ids.
  - `a_duplicated_rounding_points_at_the_copys_own_column`: including a source to the right of the rounded column.
  - `duplicating_without_pupils_leaves_the_sheet_empty`.
  - `duplicating_with_pupils_shares_them_in_order_with_empty_marks`: same pupil ids, same order, no cells; renaming one renames them in both classes.
  - `duplicating_with_marks_gives_the_same_calculated_sheet`: `evaluate_row` of every row is equal for original and copy.
  - `copies_take_the_lowest_free_number`: "1A" → "1A (2)"; with "1A (2)" present → "1A (3)".
  - `a_failed_duplicate_adds_nothing`: a `CREATE TEMP TRIGGER … BEFORE INSERT ON cells BEGIN SELECT RAISE(ABORT, 'test'); END` makes the `Everything` copy fail; class, column, enrolment and cell counts are unchanged and the error is `AppError::Database`.
  - `the_original_is_unchanged`: its sheet before equals its sheet after each kind of copy.
  - proptest `duplicating_everything_reproduces_the_sheet`: random columns, pupils and values; the copy's `load_sheet` equals the original's apart from ids.
- **`tests/ui.rs`:** `each_duplicate_option_creates_and_selects_the_copy`: through "More actions" and `click_menu_option`, one class per option; the class list grows, the copy is selected, and the row count is 0 or the original's. `a_failed_duplicate_shows_the_banner` (the same temp trigger).
- **`tests/desktop.rs`**, `the_column_panel_and_menus`: open "More actions" in Galician at 1000×600 and `assert_on_screen` the three entries (the longest labels in the least room); review the screenshot.

### Docs
- `README.md`, *Using it*, after the **Columns** bullet: "**⋯ → Duplicate class** copies a class's columns and settings; **Duplicate with pupils** brings the same pupils with empty marks (for a new term), and **Duplicate with pupils and marks** makes an exact copy. The copy is called e.g. *1A (2)*; rename it in place."
- Changelog: "Added ⋯ → Duplicate class, with or without its pupils and marks".

### Done when
- [ ] The three entries work as described, in English and Galician (test names above pass).
- [ ] Rounded columns in a copy point at the copy's columns; a failed copy adds nothing.
- [ ] Desktop screenshot of the Galician menu at 1000×600 reviewed; nothing clipped.
- [ ] `cargo test`, the coverage gate, the desktop suite, clippy and fmt pass.
