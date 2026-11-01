[//]: # ( ---------------------------------------------------------------------- )
[//]: # (+ Authors: 	Ran# <ran.hash@proton.me> )
[//]: # (+ Created: 	2026/10/28 10:26:46.388247 )
[//]: # (+ Revised: 	2026/10/29 15:46:50.932685 )
[//]: # ( ---------------------------------------------------------------------- )

# TODO

The backlog. Each item is self-contained: reading it alone is enough to
implement it fully, without asking anything. Work top to bottom unless an item
says otherwise.

Next item number: 19

## Index

7. [Run mutation testing on the core logic and strengthen weak tests](#7-run-mutation-testing-on-the-core-logic-and-strengthen-weak-tests)
8. [Make the desktop baselines work on other displays](#8-make-the-desktop-baselines-work-on-other-displays)
15. [Duplicate a class, with or without its pupils and marks](#15-duplicate-a-class-with-or-without-its-pupils-and-marks)
16. [Get the user's sign-off on the logo, and redraw it if they ask](#16-get-the-users-sign-off-on-the-logo-and-redraw-it-if-they-ask)
17. [Add a .gitattributes so checkouts and patches keep test files intact](#17-add-a-gitattributes-so-checkouts-and-patches-keep-test-files-intact)
18. [Close the sheet's menus when the window is resized](#18-close-the-sheets-menus-when-the-window-is-resized)

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

## 7. Run mutation testing on the core logic and strengthen weak tests

**Why:** coverage only shows that code ran, not that a test would notice it being wrong. The testing rules ask for occasional mutation testing of the core logic to find tests that execute code without really checking it. It has never been run on this project, and `cargo-mutants` isn't installed (`cargo install cargo-mutants`).
**Goal:** every mutant that cargo-mutants makes in the core logic is caught by a test, or is listed as equivalent (a change that can't alter behaviour), with the reason.
**Status (paused 2026-10-29):** done in the main tree, uncommitted: tests killing the 9 survivors in `compute.rs`, `lock.rs` and `i18n/mod.rs` (`tests/i18n.rs`, `tests/lock.rs`); `.cargo/mutants.toml` (`profile`, `exclude_globs`, the equivalent `weights_add_up` mutant); `[profile.mutants]` in `Cargo.toml`; `mutants.out*/` and `/.claude/` in `.gitignore`; the README and CHANGELOG lines. Not yet confirmed: a re-run of those 9 mutants showing they are now caught. Step 4 (241 mutants: `columns.rs` 220, `migrations.rs` 11, `storage.rs` 8, `cells.rs` 2) hasn't run; worktree `.claude/worktrees/agent-a29b7b27f3bbc65ef` holds an extended, not yet compiled `new_columns_get_sensible_defaults` in `tests/gradebook.rs` aimed at the 192 `kind_fields` mutants (bring it over, then delete the worktree). How to run fast, in one PowerShell command: remove `MAKEFLAGS`, `MFLAGS` and `CARGO_MAKEFLAGS` (nmake, building OpenSSL, can't read them); set `$env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER = 'rust-lld'`; point `TEMP`/`TMP` at a folder under `.claude/` (excluded from Defender); dot-source `scripts/use-perl.ps1`; warm the build with `cargo test --profile mutants --no-run --test …`; then `cargo mutants --gitignore true --jobserver false -j 3 --copy-target true --baseline skip --file <file> -- --test …`.

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
**Status (paused 2026-10-29):** steps 1 and 3 done in the main tree, uncommitted, not yet compiled: the missing-baseline message and the `App::px`/`App::area` helpers in `tests/desktop_support/mod.rs`, the blob sizes and gaps in `tests/desktop.rs` scaled by them, and the README paragraph. Left: run the desktop suite at 100 % (the first build after the `Cargo.toml` profile changes rebuilds everything), then step 2 (switch Windows to 125 % and 150 %, record, review every image) and the changelog line.

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

## 15. Duplicate a class, with or without its pupils and marks

**Why:** a teacher who starts a new term, or teaches the same subject to another group, has to rebuild a class's sheet column by column: titles, weights, maxima, bonus tests, final grade, roundings. Nothing copies a class today; the class "⋯" menu (`ui/sheet-page.slint`, `more-menu`) has "Add from another class…", "Hide/Show weights & scales", the two Excel entries and "Delete class". The user asked (2026-10-28) to choose each time whether the copy brings the pupils, and whether it brings their marks. The backup work (2026-10-29) already added the two helpers this item needs: `columns::insert_column(connection, class_id, title, kind) -> Result<i64>` in `src/gradebook/columns.rs` and `classes::free_class_name(connection, name) -> Result<String>` in `src/gradebook/classes.rs`; reuse them. The "⋯" menu now also has "Export to Excel…" and "Import from Excel…" before "Delete class".
**Goal:** the class "⋯" menu has three entries, **Duplicate class**, **Duplicate with pupils** and **Duplicate with pupils and marks**; choosing one creates the copy, selects it and shows its sheet.
**Status (paused 2026-10-29):** implemented in worktree `.claude/worktrees/agent-aad6089da6876f4a9`, uncommitted; its tests passed (`duplicate` 11, `gradebook` 27, `i18n` 11, `ui` 100). Left: clippy, the coverage gate, the desktop screenshot of the Galician menu, the commit, and the merge into `main`.

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
1. **`src/gradebook/columns.rs`:** nothing to add: `insert_column` exists (`add_column` and the backup's add-classes call it).
2. **`src/gradebook/classes.rs`:** nothing to add: `free_class_name` exists.
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

## 16. Get the user's sign-off on the logo, and redraw it if they ask

**Why:** item 1 (the logo) was built while the user was away, so its "approval first" step was replaced by sending them the previews (2026-10-29) and carrying on. The logo is in use everywhere: the sidebar, first-run and lock screens (`BrandMark` in `ui/components.slint`), the window and taskbar (`icon` in `ui/app-window.slint`), the exe (`build.rs` embeds `assets/logo.ico`), the installer (`SetupIconFile` in `installer/avalio-lite.iss`) and the README.
**Goal:** the user has seen the logo and either approved it or had it changed to what they want.

### Behaviour
- Ask the user (with AskUserQuestion) whether the tick-on-tile logo is approved, showing `assets/logo-256.png` and the 16 px rendering. Options: approve; change colours; change the shape; something else.
- On approval: nothing else changes; note the approval and their words in the hand-off.
- On a change: edit only `assets/logo.svg` and `assets/logo-dark.svg` (same 64 × 64 view box; the dark variant keeps the dark theme's `accent` `#8ea6ff` tile and `on-accent` `#0f1530` strokes unless told otherwise), then regenerate the icon files.

### Out of scope
- Where the logo is used; that is done.

### Implementation
1. Ask as described; wait for the answer.
2. If a change is wanted, edit the two SVGs, run `cargo run --example render_logo -- <scratch folder>`, send the new previews, and repeat until approved.
3. Rewrite the desktop baselines (`$env:AVALIO_UPDATE_BASELINES = "1"; cargo test --test desktop -- --ignored --test-threads=1`) and review every changed image.

### Tests
- `tests/logo.rs` already checks the files are valid, up to date with the SVG and legible at 16 px; it must still pass.

### Docs
- `CHANGELOG.md` (when committing, only if the logo changed): "Changed the logo to …".

### Done when
- [ ] The user's approval (quoted) or the redrawn logo, approved.
- [ ] `cargo test`, the desktop suite, clippy and fmt pass.

## 17. Add a .gitattributes so checkouts and patches keep test files intact

**Why:** the repo has no `.gitattributes`, so Git for Windows' defaults decide how files are stored. Two things went wrong on 2026-10-29 because of that:
- A fresh worktree checkout (with `core.autocrlf=true`, the Git for Windows default) turned `migrations/*.sql`, `tests/golden/schema.sql` and `tests/golden/config-*.toml` into CRLF. The golden tests compare those byte for byte, so two of them failed before any code changed.
- Git for Windows' system attributes send `.ods` files (and other office formats) through the `astextplain` text converter in `git diff`. A `git diff --binary` patch of the new fixtures `tests/golden/import.ods` and `import-types.ods` held their extracted text (84 and 74 bytes) instead of the zip. Applied elsewhere, it wrote broken files, and `ods_files_import` failed with "Could not find EOCD".

**Goal:** any fresh checkout on Windows passes the whole test suite, and diffs and patches of binary fixtures carry the real bytes.

### Behaviour
- Text files are stored with LF in the repo, as today.
- Golden files and migrations are LF in the working copy too, whatever `core.autocrlf` says.
- Binary fixtures (`.ods`, `.xls`, `.xlsx`, `.png`, `.ico`, `.db`, `.avalio`) are never converted or turned into text, in checkouts or in `git diff`.

### Out of scope
- Changing the line endings of source files in the working copy (`.rs`, `.slint`, `.md` stay as `core.autocrlf` makes them).

### Implementation
1. `.gitattributes` (new, repo root):
   ```
   * text=auto
   migrations/*.sql text eol=lf
   tests/golden/*.sql text eol=lf
   tests/golden/*.toml text eol=lf
   tests/golden/*.tsv text eol=lf
   *.ods binary
   *.xls binary
   *.xlsx binary
   *.png binary
   *.ico binary
   *.db binary
   *.avalio binary
   ```
   `binary` turns off both line-ending conversion and the text converter, so it fixes both problems for those files.
2. Run `git add --renormalize .` and check that `git status` lists no content changes beyond `.gitattributes` itself. Stored files are already LF, so nothing else should change; if something does, look at it before going on.
3. `README.md`, "Building it" (or the section that lists build steps): one sentence: "`.gitattributes` keeps the golden files and migrations in LF; don't remove it, or the golden tests fail on a Windows checkout."

### Tests
- In a fresh clone (`git clone <repo> D:\tmp\avalio-check`, with `core.autocrlf=true`): `cargo test --test golden --test spreadsheet --test db` passes. This proves the checkout keeps golden files byte-identical.
- `git diff --binary` of a changed `.ods` fixture (change one cell of a copy in LibreOffice, then diff it) shows `GIT binary patch`, not text.

### Docs
- `README.md`: the sentence in step 3. Changelog: "Added a .gitattributes so a fresh Windows checkout passes the golden tests".

### Done when
- [ ] `.gitattributes` exists with the lines in step 1.
- [ ] The fresh-clone test run passes (output).
- [ ] `git diff --binary` of an `.ods` change is a binary patch (output).

## 18. Close the sheet's menus when the window is resized

**Why:** an open popup in Slint keeps the position it opened at. Open the sheet's "⋯" menu (`more-menu` in `ui/sheet-page.slint`) with the window at 1280 px, then shrink the window to 1000 px: the menu stays where it was and ends partly outside the window, cut off. The same happens to `column-menu` and `pupil-menu` in that file. The desktop test `the_sheet_menu_and_an_import` (`tests/desktop.rs`) works around it by reopening the menu at each size (the comment "an open popup keeps its place when the window is resized").
**Goal:** resizing the window closes any open sheet menu, as Windows' own menus do, so a menu is never left cut off.

### Behaviour
- Any change of the window's width or height closes `more-menu`, `column-menu` and `pupil-menu` if open.
- Nothing else happens: no option is chosen, no message is shown. Reopening the menu places it correctly for the new size.
- Moving the window without resizing it leaves an open menu open.

### Out of scope
- Popups outside the sheet page (Settings has none today).

### Implementation
1. `ui/sheet-page.slint`: add `changed width => { more-menu.close(); column-menu.close(); pupil-menu.close(); }` and the same for `height` on the page's root element. Closing a popup that isn't open does nothing, so no check is needed first.
2. `tests/desktop.rs`, `the_sheet_menu_and_an_import`: remove the reopen-at-each-size workaround and its comment. Open the menu once at 1280×800, resize to 1000×600, and check the menu is gone (`locate_in_popup("Export to Excel…")` returns nothing).

### Tests
- `tests/desktop.rs`: `resizing_closes_the_sheet_menus` opens each of the three menus in turn, resizes, and checks the popup is gone, in light and dark. This is a regression test for the cut-off menu.
- The existing desktop baselines of the menus at 1000×600 (`sheet-menu-*`) still match after the workaround is removed (the menu is opened again at that size).

### Docs
- Changelog: "Fixed the sheet's menus staying open, partly cut off, when the window was resized".

### Done when
- [ ] Resizing with any of the three menus open closes it (desktop test output).
- [ ] The workaround in `the_sheet_menu_and_an_import` is gone.
- [ ] `cargo test`, the desktop suite, clippy and fmt pass.
