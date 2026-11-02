[//]: # ( ---------------------------------------------------------------------- )
[//]: # (+ Authors: 	Ran# <ran.hash@proton.me> )
[//]: # (+ Created: 	2026/10/28 10:26:46.388247 )
[//]: # (+ Revised: 	2026/11/02 08:45:07.399686 )
[//]: # ( ---------------------------------------------------------------------- )

# TODO

The backlog. Each item is self-contained: reading it alone is enough to
implement it fully, without asking anything. Work top to bottom unless an item
says otherwise.

Next item number: 37

## Index

24. [Open the window at a size and place that fit the screen](#24-open-the-window-at-a-size-and-place-that-fit-the-screen)
7. [Run mutation testing on the core logic and strengthen weak tests](#7-run-mutation-testing-on-the-core-logic-and-strengthen-weak-tests)
8. [Make the desktop baselines work on other displays](#8-make-the-desktop-baselines-work-on-other-displays)
15. [Duplicate a class, with or without its pupils and marks](#15-duplicate-a-class-with-or-without-its-pupils-and-marks)
16. [Get the user's sign-off on the logo, and redraw it if they ask](#16-get-the-users-sign-off-on-the-logo-and-redraw-it-if-they-ask)
17. [Add a .gitattributes so checkouts and patches keep test files intact](#17-add-a-gitattributes-so-checkouts-and-patches-keep-test-files-intact)
18. [Close the sheet's menus when the window is resized](#18-close-the-sheets-menus-when-the-window-is-resized)
19. [Check that exporting over an existing file asks before replacing it](#19-check-that-exporting-over-an-existing-file-asks-before-replacing-it)
20. [Collapse the sidebar to a narrow icon rail](#20-collapse-the-sidebar-to-a-narrow-icon-rail)
21. [Make Escape close the column panel wherever the focus is in it](#21-make-escape-close-the-column-panel-wherever-the-focus-is-in-it)
22. [Step the column panel to the previous or next column](#22-step-the-column-panel-to-the-previous-or-next-column)
23. [Move the column with Ctrl+Left and Ctrl+Right in the column panel](#23-move-the-column-with-ctrlleft-and-ctrlright-in-the-column-panel)
30. [Write docs/design-decisions.md with the structure decisions](#30-write-docsdesign-decisionsmd-with-the-structure-decisions)
25. [Move the app state out of the UI layer and split it by concern](#25-move-the-app-state-out-of-the-ui-layer-and-split-it-by-concern)
31. [Group each feature's logic and UI wiring into one folder](#31-group-each-features-logic-and-ui-wiring-into-one-folder)
32. [Group the shared infrastructure under src/platform/](#32-group-the-shared-infrastructure-under-srcplatform)
26. [Stop the gradebook logic from depending on the translations](#26-stop-the-gradebook-logic-from-depending-on-the-translations)
27. [Keep Slint's key events out of the shortcut rules](#27-keep-slints-key-events-out-of-the-shortcut-rules)
33. [Remove the repeated code in the app state and the UI error reporting](#33-remove-the-repeated-code-in-the-app-state-and-the-ui-error-reporting)
34. [Split the shortcut rules, the translations and the sheet view into smaller files](#34-split-the-shortcut-rules-the-translations-and-the-sheet-view-into-smaller-files)
35. [Split the spreadsheet import, export, backup and grade-calculation files](#35-split-the-spreadsheet-import-export-backup-and-grade-calculation-files)
28. [Split ui/components.slint into one file per component family](#28-split-uicomponentsslint-into-one-file-per-component-family)
36. [Split the large Slint pages into their parts](#36-split-the-large-slint-pages-into-their-parts)
29. [Guard the module dependency rules with a test](#29-guard-the-module-dependency-rules-with-a-test)

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

## 24. Open the window at a size and place that fit the screen

**Why:** `ui/app-window.slint` gives the window `preferred-width: 1280px; preferred-height: 800px` and `min-width: 1000px; min-height: 600px`, and nothing sets where it opens: `src/app/mod.rs` `open` builds the window and `launch` calls `run()`, so Windows picks the place (its default cascade position). On a screen smaller than 1280×800 logical pixels (a 1366×768 laptop at 125 % scaling is about 1093×580 logical once the taskbar is out; 1920×1080 at 150 % is 1280×690), the window is larger than the screen. The user installed the app on another computer and it opened in the bottom-right corner, with parts of the window off-screen until they moved it. The user called this unacceptable; it is first in the backlog for that reason.
**Goal:** on any Windows computer, at any display scaling, the app opens fully visible, centred on the screen the user is working on, as large as is comfortable up to 1280×800, and maximised when even the minimum size doesn't fit.

### Behaviour
- **Which screen:** the monitor under the mouse pointer when the app starts (where the user just clicked the shortcut or exe); if that can't be read, the primary monitor. Only the **work area** counts (the screen minus the taskbar and docked bars), so nothing is ever hidden under the taskbar.
- **Size:** in logical pixels (physical ÷ that monitor's scale), `width = min(1280, work width − 48)`, `height = min(800, work height − 48)`: 24 px of margin on each side, so the window's edges and shadow are clearly visible and it looks placed on purpose.
- **Too small for the minimum** (the result is under 1000×600): the window opens **maximised** to the work area, and the window's minimum size is lowered to the work area's logical size, so a maximised window never extends beyond the screen. The layouts must still be usable at that size: the sheet already scrolls; check every screen (below).
- **Place:** centred in the work area: `x = work.left + (work width − window width) / 2`, the same for y. The window's top-left is never left of or above the work area, so the title bar can always be grabbed.
- **Multiple monitors with different scales:** the size is computed with the scale of the chosen monitor, not the primary one's.
- **Failure to read the monitor** (Win32 call fails): log `warn` "couldn't read the screen's work area, using the primary screen: {error}"; if the primary also fails, log `warn` and keep Slint's defaults (the app must still start). Never silent.
- The first-run, lock and main screens all open in the same window, so all get the same placement.

### Out of scope
- Remembering the size and position from the last run (a follow-up item: it must use this item's fit check, since the saved place may be on a monitor that is gone).
- Moving the window when the user changes monitors or scaling while it is open (Windows handles that).

### Implementation
1. `Cargo.toml`: add `windows-sys` (0.61, the version the tests already use) as a normal dependency under `[target.'cfg(windows)'.dependencies]` with features `Win32_Foundation`, `Win32_Graphics_Gdi` (`MonitorFromPoint`, `GetMonitorInfoW`, `MONITOR_DEFAULTTOPRIMARY`), `Win32_UI_HiDpi` (`GetDpiForMonitor`, `MDT_EFFECTIVE_DPI`) and `Win32_UI_WindowsAndMessaging` (`GetCursorPos`). Keep the dev-dependency entry's features in step.
2. New `src/app/placement.rs`, two parts, so the rules are testable without a screen:
   - Pure: `pub struct WorkArea { left: i32, top: i32, width: u32, height: u32, scale: f32 }` (physical pixels + scale) and `pub enum Placement { Window { x: i32, y: i32, width: f32, height: f32 } /* x, y physical; size logical */, Maximized { min_width: f32, min_height: f32 } }`, with `pub fn place(area: &WorkArea) -> Placement` implementing the rules above. The numbers 1280, 800, 1000, 600 and 24 are named constants with a comment saying they must match `ui/app-window.slint`.
   - Win32: `pub fn work_area() -> Result<WorkArea, String>`: `GetCursorPos` → `MonitorFromPoint(point, MONITOR_DEFAULTTOPRIMARY)` → `GetMonitorInfoW` (`rcWork`) → `GetDpiForMonitor(.., MDT_EFFECTIVE_DPI, ..)` → `scale = dpi / 96`. Each failing call returns an `Err` naming the call.
   - `pub fn apply(window: &AppWindow)`: `work_area()` → `place()` → for `Window`, `window.window().set_size(slint::LogicalSize::new(w, h))` and `set_position(slint::PhysicalPosition::new(x, y))`; for `Maximized`, set the new min sizes (step 3) and `window.window().set_maximized(true)`. Log `info` "window placed at {x},{y}, {w}×{h} on a {W}×{H} work area at {scale}" (or "maximised …"). Errors logged as above.
3. `ui/app-window.slint`: turn the fixed minimums into `in property <length> fit-min-width: 1000px;` / `fit-min-height: 600px;` with `min-width: root.fit-min-width;` etc., so step 2 can lower them on small screens. The generated setters are `set_fit_min_width` / `set_fit_min_height`.
4. `src/app/mod.rs` `open`: call `placement::apply(&window)` after `show_startup` and before returning, so it applies before `run()` shows the window. Register the module in `src/app/mod.rs`. Check by running the real exe that Slint 1.18's winit backend honours `set_position` given before the window is shown; if it doesn't, call `apply` from a `slint::Timer::single_shot(Duration::ZERO, …)` started before `run()` (it fires once the event loop runs and the window exists) and note that in a comment.
5. `docs/development.md` source tree (line ~84): add "placement.rs (where the window opens)" to the `app/` line.

### Tests
- Unit tests of `place` in `src/app/placement.rs` (or `tests/placement.rs` if `place` is made `pub` through the crate's test exports, following how other `src/app` code is tested): 1920×1040 work area at 100 % → 1280×800 centred at (320, 120); 1920×1040 at 150 % → 1232×645.3 logical (work 1280×693.3, minus 48 each way) centred; 1366×728 at 125 % → maximised with min 1092.8×582.4; 1280×720 at 100 % → 1232×672 at (24, 24); a secondary monitor at negative coordinates (left −1920, top 0) → centred within it, x stays negative; a work area offset by a top taskbar (top 40) → y ≥ 40; exactly 1000+48 × 600+48 → a window of 1000×600, not maximised (boundary).
- Property test (proptest, if already a dev-dependency; otherwise a loop over a grid of sizes 800–3840 × 600–2160 and scales 1.0/1.25/1.5/1.75/2.0): the result is always fully inside the work area, never larger than 1280×800 logical, and maximised exactly when 1000×600 doesn't fit.
- `tests/desktop.rs`: `the_window_opens_inside_the_work_area`: start the real exe, read its rectangle with `GetWindowRect` and the monitor's `rcWork`, and check the window is fully inside and centred (±2 px). Also start it with the mouse moved to a second monitor when one exists (skip that part, with a printed note, on single-monitor machines).
- `tests/desktop.rs`: `small_screens_stay_usable`: resize to the smallest real-world case (`app.resize(1092, 582)`) and check, on the sheet, the column panel, Settings and the lock screen, that every button and field `assert_on_screen` checks today is reachable (visible, or visible after `scroll_main`). Record baselines `*-1092x582@100.png` for the sheet and Settings in light, review each by eye.
- `tests/entry_point.rs`: the app still starts when `placement::work_area` fails (inject the failure through the same seam the other startup tests use) and the warning is logged.

### Docs
- `docs/user-guide.md`, first-run section: "The window opens centred on the screen you started it from, sized to fit; on small screens it opens maximised."
- Changelog: "Fixed the window opening partly off-screen on smaller or scaled displays: it now opens centred on the current screen, sized to fit, and maximised when the screen is too small".

### Done when
- [ ] The real exe opens fully inside the work area, centred (desktop test output, with the measured rectangles).
- [ ] At 1092×582 every screen is usable (desktop test output + reviewed baselines).
- [ ] The unit and property tests of `place` pass, boundary cases included.
- [ ] A failing monitor query is logged and the app still starts (test output).
- [ ] Checked by hand at 125 % and 150 % Windows scaling, if this machine allows changing it (say which were checked).
- [ ] `cargo test`, the desktop suite, the coverage gate, clippy and fmt pass; `metadata -m` run on every edited file.

## 7. Run mutation testing on the core logic and strengthen weak tests

**Why:** coverage only shows that code ran, not that a test would notice it being wrong. The testing rules ask for occasional mutation testing of the core logic to find tests that execute code without really checking it. It has never been run on this project, and `cargo-mutants` isn't installed (`cargo install cargo-mutants`).
**Goal:** every mutant that cargo-mutants makes in the core logic is caught by a test, or is listed as equivalent (a change that can't alter behaviour), with the reason.
**Status (paused; updated 2026-11-01):** committed in `b6a1f27` and `288c9ee` (released in 0.3.0): tests killing the 9 survivors in `compute.rs`, `lock.rs` and `i18n/mod.rs` (`tests/i18n.rs`, `tests/lock.rs`); `.cargo/mutants.toml` (`profile`, `exclude_globs`, the equivalent `weights_add_up` mutant); `[profile.mutants]` in `Cargo.toml`; `mutants.out*/` and `/.claude/` in `.gitignore`; the README and CHANGELOG lines. `columns.rs` has changed since the mutant counts below (the backup work split `insert_column` out of `add_column`), so list the mutants again before step 4. Not yet confirmed: a re-run of those 9 mutants showing they are now caught. Step 4 (241 mutants: `columns.rs` 220, `migrations.rs` 11, `storage.rs` 8, `cells.rs` 2) hasn't run; worktree `.claude/worktrees/agent-a29b7b27f3bbc65ef` holds an extended, not yet compiled `new_columns_get_sensible_defaults` in `tests/gradebook.rs` aimed at the 192 `kind_fields` mutants (bring it over, then delete the worktree). How to run fast, in one PowerShell command: remove `MAKEFLAGS`, `MFLAGS` and `CARGO_MAKEFLAGS` (nmake, building OpenSSL, can't read them); set `$env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER = 'rust-lld'`; point `TEMP`/`TMP` at a folder under `.claude/` (excluded from Defender); dot-source `scripts/use-perl.ps1`; warm the build with `cargo test --profile mutants --no-run --test …`; then `cargo mutants --gitignore true --jobserver false -j 3 --copy-target true --baseline skip --file <file> -- --test …`.

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
- `docs/development.md`, *Tests* section: one line on running `cargo mutants` now and then, and where its config lives. Changelog: "Tests strengthened after mutation testing."

### Done when
- [ ] `cargo mutants` on the listed files reports no missed mutants apart from those in `.cargo/mutants.toml`.
- [ ] The default suite passes.

## 8. Make the desktop baselines work on other displays

**Why:** the 40 baselines in `tests/desktop/baselines/` were captured on one machine, at 100 % display scaling with its installed Segoe UI. `App::snapshot` (`tests/desktop_support/mod.rs`) names each file `<screen>-<look>@<scale>.png`. On a display at 125 % or 150 % there is no matching baseline, so every snapshot fails with "no baseline". On another machine, different font hinting would fail them too. The desktop suite only passes where it was recorded.
**Goal:** the desktop suite passes at 100 %, 125 % and 150 % scaling on this machine, and `docs/development.md` says exactly what to do on a new machine.
**Status (paused; updated 2026-11-01):** steps 1 and 3 committed in `288c9ee` (released in 0.3.0), and the whole desktop suite passes at 100 %: the missing-baseline message and the `App::px`/`App::area` helpers in `tests/desktop_support/mod.rs`, the blob sizes and gaps in `tests/desktop.rs` scaled by them, and the README paragraph. Left: step 2 (switch Windows to 125 % and 150 %, record, review every image) and the changelog line.

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
- `docs/development.md`, *Tests* section: baselines exist per display scale; how to record them for a new scale or machine, and to review every image first. Changelog: "Desktop test baselines for 125 % and 150 % display scaling."

### Done when
- [ ] `cargo test --test desktop -- --ignored --test-threads=1` passes at 100 %, 125 % and 150 %.
- [ ] Every new baseline was opened and reviewed.

## 15. Duplicate a class, with or without its pupils and marks

**Why:** a teacher who starts a new term, or teaches the same subject to another group, has to rebuild a class's sheet column by column: titles, weights, maxima, bonus tests, final grade, roundings. Nothing copies a class today; the class "⋯" menu (`ui/sheet-page.slint`, `more-menu`) has "Add from another class…", "Hide/Show weights & scales", the two Excel entries and "Delete class". The user asked (2026-10-28) to choose each time whether the copy brings the pupils, and whether it brings their marks. The backup work (2026-10-29) already added the two helpers this item needs: `columns::insert_column(connection, class_id, title, kind) -> Result<i64>` in `src/gradebook/columns.rs` and `classes::free_class_name(connection, name) -> Result<String>` in `src/gradebook/classes.rs`; reuse them. The "⋯" menu now also has "Export to Excel…" and "Import from Excel…" before "Delete class".
**Goal:** the class "⋯" menu has three entries, **Duplicate class**, **Duplicate with pupils** and **Duplicate with pupils and marks**; choosing one creates the copy, selects it and shows its sheet.
**Status (paused 2026-10-29):** implemented in worktree `.claude/worktrees/agent-aad6089da6876f4a9`, uncommitted; its tests passed (`duplicate` 11, `gradebook` 27, `i18n` 11, `ui` 100). Left: clippy, the coverage gate, the desktop screenshot of the Galician menu, the commit, and the merge into `main`. The worktree branches from `ff089ed`, before 0.3.0: the class "⋯" menu, `src/gradebook/classes.rs`, `columns.rs`, `tests/ui.rs` and the `sheet-menu-*` baselines have changed in `main` since, so rebase it onto `main` first and resolve those files (the menu now has the two Excel entries before "Delete class").

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
- `docs/user-guide.md`, *Using it → Columns*, after the first paragraph: "**⋯ → Duplicate class** copies a class's columns and settings; **Duplicate with pupils** brings the same pupils with empty marks (for a new term), and **Duplicate with pupils and marks** makes an exact copy. The copy is called e.g. *1A (2)*; rename it in place."
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
3. `docs/development.md`, after the requirements list: one sentence: "`.gitattributes` keeps the golden files and migrations in LF; don't remove it, or the golden tests fail on a Windows checkout."

### Tests
- In a fresh clone (`git clone <repo> D:\tmp\avalio-check`, with `core.autocrlf=true`): `cargo test --test golden --test spreadsheet --test db` passes. This proves the checkout keeps golden files byte-identical.
- `git diff --binary` of a changed `.ods` fixture (change one cell of a copy in LibreOffice, then diff it) shows `GIT binary patch`, not text.

### Docs
- `docs/development.md`: the sentence in step 3. Changelog: "Added a .gitattributes so a fresh Windows checkout passes the golden tests".

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

## 19. Check that exporting over an existing file asks before replacing it

**Why:** item 10 (Export to Excel…, released in 0.3.0) relies on the Windows Save dialog to ask before an export replaces an existing file: `NativeDialogs::save_file` in `src/app/dialogs.rs` uses rfd's `save_file()`, which turns on `FOS_OVERWRITEPROMPT`, so the app doesn't ask a second time. Automated tests swap in fake dialogs, and the desktop suite scripts the Save answer, so no test has ever seen the real dialog's question. Only a person at the real dialog can check it.
**Goal:** the user has seen Windows ask "… already exists. Do you want to replace it?" when exporting over an existing file, or the app asks itself if Windows doesn't.

### Behaviour
- Exporting to a file name that already exists: Windows asks before replacing it. Yes replaces it and the green banner appears; No returns to the Save dialog and nothing is written.
- The same holds for **Settings → Backup → Export everything…**, which uses the same `save_file`.

### Out of scope
- Any change to the dialogs when the question does appear.

### Implementation
1. Run the release build (`dist\avalio-lite-0.3.0-portable.exe`, or `cargo run --release`) on a class with marks.
2. **⋯ → Export to Excel…**, save as `D:\tmp\check.xlsx`; then export again to the same name. Note whether Windows asks, and try both No and Yes.
3. Repeat with **Settings → Backup → Export everything…** to `D:\tmp\check.avalio`.
4. If Windows does not ask in either case: in `src/app/dialogs.rs`, after `save_file()` returns a path that exists, ask `self.confirm(...)` with a new message `ConfirmReplaceFile { file }` ("{file} already exists. Replace it?" / "{file} xa existe. Substituílo?") in `src/i18n/messages.rs`, add it to `every_message()` in `tests/i18n.rs`, and test both answers in `tests/ui.rs`.

### Tests
- Manual: the two checks in steps 2 and 3, with a screenshot of each question.
- Only if step 4 is needed: `exporting_over_a_file_asks_first` and `no_keeps_the_existing_file` in `tests/ui.rs`, for the sheet export and the backup.

### Docs
- None if Windows asks. If step 4 is needed, changelog: "Fixed exporting over an existing file without asking first".

### Done when
- [ ] Screenshots of Windows (or the app) asking before replacing, for the sheet export and the backup.
- [ ] With No, the existing file is unchanged (its modified time is the same).

## 20. Collapse the sidebar to a narrow icon rail

**Why:** the sidebar (`component Sidebar` in `ui/shell.slint`) is always 236 px wide. On a 1000 px window that is almost a quarter of the width, taken from the sheet, where a class with many columns needs every pixel (it starts scrolling sideways sooner). Most modern apps (VS Code, Outlook, Teams, Windows Settings) let the user fold the sidebar to a narrow strip of icons and open it again with one click; avalio-lite has no such control.
**Goal:** a button at the top of the sidebar, or Ctrl+B, folds it to a 64 px rail and back. The sheet gains the 172 px freed. The choice is saved in `config.toml`, so the app opens the way it was left.

### Behaviour
- **Expanded (default, as today):** 236 px. A new collapse button (chevron pointing left, 28×28 px, hover background `Theme.raised`, radius `Theme.radius-s`) sits at the right end of the brand row, after "avalio lite". Accessible label "Collapse the sidebar".
- **Collapsed:** 64 px wide, keeping the 1 px right border line. Top to bottom:
  - The brand mark (30 px tile), centred; the "avalio lite" text is hidden. Below it the expand button (chevron pointing right), centred, accessible label "Expand the sidebar".
  - The "Classes" heading is hidden.
  - Each class is a 40×38 px centred entry showing the first 3 characters of its name, trimmed (`7A` → `7A`, `1º ESO A` → `1º`, `Matemáticas` → `Mat`), with the same selected/hover colours as `NavItem`. 3 characters because class names here are short codes ("7A", "1º B") that stay recognisable in 40 px; the pupil count is hidden. The accessible label stays the full class name, so screen readers lose nothing. Two classes with the same first 3 characters both show them; the selected highlight still tells which is open.
  - The "+ New class" field becomes a single "+" entry (accessible label "New class"); clicking it expands the sidebar and focuses the New class field, because a name can't be typed into 64 px.
  - Lock (only when a PIN is set), Light/Dark mode and Settings show only their icons, centred, with their full text as accessible label.
- **Toggling:** the collapse/expand button or the `toggle-sidebar` shortcut (default **Ctrl+B**, the key VS Code and Edge use for this) switch between the two. The width animates over 150 ms ease-out; the text is hidden as soon as the collapse starts, so it never wraps or overflows mid-animation.
- **Remembered:** saved to `config.toml` as `preferences.sidebar_collapsed` at each toggle and read on start and after switching database, exactly like `show_column_settings`. A settings file without the key opens expanded.
- **Saving fails** (settings file read-only): the sidebar still toggles for this session, the error banner shows the error as for the other preferences, and the log gets `warn` "saving the sidebar preference failed: {error}". Toggling still works so the user isn't stuck; only remembering it failed.
- **Keyboard:** Tab reaches the collapse/expand button and the collapsed entries in the same order as the expanded ones; Enter/Space press them (`KeyboardPress` + `FocusRing`, as `IconNavItem`).
- The lock and first-run screens have no sidebar and are unchanged; shortcuts don't run there today, so Ctrl+B does nothing there.
- If a user's `config.toml` already maps another action to Ctrl+B, the existing clash rule of `Shortcuts::from_config` (`src/shortcuts.rs`) applies: reported on start, default used.

### Out of scope
- Collapsing automatically on narrow windows (a possible follow-up once this exists).
- Tooltips on the collapsed entries (Slint has no built-in tooltip).
- Dragging the sidebar edge to any width.

### Implementation
1. `src/config.rs`: add `#[serde(default)] pub sidebar_collapsed: bool` to `Preferences` (doc comment "Whether the sidebar is folded to its icon rail."), and `sidebar_collapsed: false` in `Preferences::default()`. `serde(default)` so older settings files still load.
2. `src/app/state/mod.rs`: `set_show_column_settings`, `set_theme` and `set_language` differ only in the field they write. Extract a private `fn update_preferences(&mut self, change: impl FnOnce(&mut Preferences)) -> Result<()>` (clone config or `AppError::NoDatabaseOpen`, apply `change`, `save_config`), make the three call it, and add `pub fn set_sidebar_collapsed(&mut self, collapsed: bool) -> Result<()>` on top of it (DRY is a hard rule).
3. `src/shortcuts.rs`: add `Action::ToggleSidebar` to the enum and to `ALL` after `ToggleColumnSettings` (size becomes 7); `config_name` → `"toggle-sidebar"`; `default_combo` → `ctrl(false, Key::Letter('B'))`; extend the `default_combo` doc comment with "Ctrl+B folds the sidebar, as in VS Code and Edge".
4. `src/i18n/shortcuts.rs` `action_label`: English "Collapse or expand the sidebar", Galician "Contraer ou expandir a barra lateral".
5. `ui/shortcuts.slint`: add `toggle-sidebar` to `ShortcutAction`. `src/app/shortcuts.rs` `to_ui`: `Action::ToggleSidebar => ShortcutAction::ToggleSidebar`.
6. `ui/gradebook.slint`: add `short-name: string` to `ClassItem`; add `in property <bool> sidebar-collapsed: false;` next to `show-column-settings` and `callback toggle-sidebar();` next to `toggle-column-settings`.
7. `src/app/gradebook/view.rs` (line ~93, where `ClassItem` is built): fill `short_name` with the first 3 `char`s (not bytes: "º" is 2 bytes) of the trimmed name, trimmed again. Put that in a small pure function `short_class_name(name: &str) -> String` so it can be unit-tested.
8. `ui/components.slint`: add `chevron-left` and `chevron-right` to `IconKind`, drawn in `Icon` as `"M 15 5 L 8 12 L 15 19"` and `"M 9 5 L 16 12 L 9 19"` (24×24 grid, stroked, no fill). Add `in property <bool> compact` to `NavItem` (plus `in property <string> short-label`) and `IconNavItem`: when set, hide the label/detail texts, centre the content (`NavItem` shows `short-label`, `IconNavItem` only its icon). `accessible-label` stays `root.label` in both modes.
9. `ui/shell.slint` `Sidebar`: `width: Gradebook.sidebar-collapsed ? 64px : 236px; animate width { duration: 150ms; easing: ease-out; }`. Add the collapse button to the brand row (`if !Gradebook.sidebar-collapsed`) and the expand button under the brand mark (`if Gradebook.sidebar-collapsed`), both calling `Gradebook.toggle-sidebar()`; their accessible labels are `@tr("Collapse the sidebar")` / `@tr("Expand the sidebar")`. Hide "avalio lite" and "Classes" when collapsed. Pass `compact: Gradebook.sidebar-collapsed` and `short-label: class.short-name` to the class `NavItem`s, and `compact` to the three `IconNavItem`s. When collapsed, show a "+" entry in place of the New class row whose click calls `Gradebook.toggle-sidebar()` then `new-class.focus()`; keep the `new-class` field in the tree in both modes (`visible: !Gradebook.sidebar-collapsed`) so the focus call has a target.
10. `ui/app-window.slint` (shortcut dispatch, line ~134): add `else if (action == ShortcutAction.toggle-sidebar) { Gradebook.toggle-sidebar(); }`.
11. `src/app/gradebook/mod.rs` `wire_preferences`: add `on_toggle_sidebar`. Unlike column settings, set the UI flag first and then save, so the sidebar toggles even when saving fails; on `Err` `log::warn!("saving the sidebar preference failed: {error}")` and `window_view::show_error`. Extract the shared "flip, clear messages, save, report" shape into one helper used by both handlers. In `show_database`, `set_sidebar_collapsed(preferences.sidebar_collapsed)` next to `set_show_column_settings`.
12. `lang/gl/LC_MESSAGES/avalio-lite.po`: add "Collapse the sidebar" → "Contraer a barra lateral" and "Expand the sidebar" → "Expandir a barra lateral" ("New class" already exists). `tests/i18n.rs` fails until every locale has them.
13. `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `metadata -m` on every edited `.rs`/`.md` file.

### Tests
- `tests/config.rs`: a settings file without `sidebar_collapsed` loads `false`; `true` round-trips. Add the field to the struct literals (lines 19, 191).
- `tests/golden.rs` + `tests/golden/config-current.toml`: add `sidebar_collapsed = true` to `current_config()` and the golden file, pinning the on-disk format.
- `tests/state.rs`: `set_sidebar_collapsed` saves, survives switching database and a restart (as the `set_show_column_settings` test, lines 57–69), and returns `AppError::NoDatabaseOpen` without a database (as line 92). The existing column-settings, theme and language tests stay green after the `update_preferences` refactor.
- Unit test of `short_class_name`: `"7A"`, `"1º ESO A"` → `"1º"`, `"  Matemáticas"` → `"Mat"`, `""` → `""`, a name of emoji/multi-byte characters doesn't panic.
- `tests/shortcuts.rs`: `toggle-sidebar` parses from config and defaults to Ctrl+B; `lock = "Ctrl+B"` in config reports the clash (regression for the new default colliding with user keys).
- `tests/i18n.rs`: the new shortcut label and the two `@tr` strings exist in English and Galician.
- `tests/ui.rs`: `the_sidebar_collapses_and_expands_and_remembers_it` (button collapses, `get_sidebar_collapsed()` true and saved; Ctrl+B expands; restart keeps it); `the_collapsed_plus_expands_and_focuses_the_new_class_field`; `collapsed_entries_keep_full_accessible_labels` (class names, Lock, Dark mode, Settings); `a_failed_save_still_toggles_the_sidebar_and_shows_the_error` (read-only settings file); extend `tab_reaches_the_sidebar_buttons_and_enter_or_space_presses_them` to the collapsed rail.
- `tests/desktop.rs`: new `the_collapsed_sidebar` on the sample class, via `every_look` and `galician_look`, with baselines `sidebar-collapsed-{light,dark}-{1000x600,1280x800}@100.png` and `sidebar-collapsed-gl-1000x600@100.png`; check the sheet starts 64 px from the left edge. The brand row's new collapse button changes every existing baseline that shows the sidebar: regenerate them and review each one by eye before committing.

### Docs
- `docs/user-guide.md`: after the light/dark bullet (line ~61) add "**Narrow sidebar:** the « button at the top of the sidebar (or Ctrl+B) folds it to a strip of icons, giving the sheet more room; » opens it again. The app remembers which you chose." Add `| Ctrl+B | Collapse or expand the sidebar | \`toggle-sidebar\` |` to the shortcut table after Ctrl+H.
- `README.md`: refresh any screenshot whose baseline changed (same file paths, so nothing to edit unless one was renamed).
- Changelog: "The sidebar folds to a narrow strip of icons with the « button or Ctrl+B, giving the sheet more width; the choice is remembered".

### Done when
- [ ] Collapse and expand work by button and Ctrl+B, animated, in light, dark and Galician (desktop test output + reviewed baselines).
- [ ] The state survives a restart (`tests/ui.rs` output) and `config.toml` shows `sidebar_collapsed`.
- [ ] Every collapsed entry keeps its full accessible label and is reachable with Tab.
- [ ] A failed save still toggles the sidebar and shows the error (test output).
- [ ] Ctrl+B is listed in Settings → Keyboard shortcuts and can be changed.
- [ ] `cargo test`, the desktop suite, the coverage gate, clippy and fmt pass; `metadata -m` run on every edited file.

## 21. Make Escape close the column panel wherever the focus is in it

**Why:** the column panel (`ColumnPanel` in `ui/column-panel.slint`, opened by clicking a column header) is meant to close on Escape: a `FocusScope` with `key-pressed` wraps its content and calls `Gradebook.close-column-panel()`. The user reports Escape doesn't work. The only test, `regression_the_column_panel_starts_on_its_title_and_escape_closes_it` in `tests/ui.rs`, presses Escape right after opening, with the focus in the Title field. Two things make it fail otherwise: (1) `key-pressed` runs only after the focused child has turned the key down, so a child that takes Escape (the std `ComboBox` under "Rounds", Slint's `TextInput` in some states) never lets it reach the panel; (2) after a mouse click on the panel's blank area, the Bonus `Toggle` or a button (they use `KeyboardPress` with `focus-on-click: false`), the focus is not inside the panel's `FocusScope`, and may still be on a sheet cell, so the key never reaches it.
**Goal:** whenever the panel is open and the user's last interaction was in it, Escape closes it.

### Behaviour
- Escape closes the panel when the focus is in any of: Title, Weight, Out of, Decimals, the Rounds combo box (closed), or when the last click was on the Bonus toggle, Move left/right or the panel's blank area.
- With the Rounds combo box's list open, the first Escape closes only the list (as everywhere on Windows); a second Escape closes the panel.
- A half-typed, refused value in a field is put back to the saved one as the panel closes (the field loses focus, `TextField.restore()` already does that), and the banner error about it is cleared through `field-restored`, as today.
- When the user has clicked a sheet cell after opening the panel, the focus is in the sheet, and Escape does what it does in a cell today; the panel stays open. This is deliberate: Escape belongs to whatever the user is working in.

### Out of scope
- Escape in other places (menus already close on Escape).

### Implementation
1. `tests/ui.rs`: first write the failing tests listed below, one per focus state, and run them, to confirm which states fail before changing code.
2. `ui/column-panel.slint`: name the scope (`scope := FocusScope`) and change `key-pressed` to `capture-key-pressed`, so the panel sees Escape before any child can take it. Keep `return reject;` for every other key so typing still reaches the fields. The Rounds `ComboBox`'s open list is a `PopupWindow`, which gets the keys itself while it is open, so its first Escape closes the list without reaching the panel and no special case is needed. `escape_with_the_rounds_list_open_closes_only_the_list` (below) checks this; if it fails, the panel must let Escape through while the list is open: replace the std `ComboBox` with the app's own `Menu` popup (`ui/components.slint`, already used by the sheet's menus), whose open state the panel can read.
3. `ui/column-panel.slint`: move the click-swallowing `TouchArea { }` inside the `FocusScope`, with `clicked => { scope.focus(); }`, so a click on the panel's blank area puts the focus in the panel (it still keeps the click from reaching the sheet).
4. `ui/column-panel.slint`: in the Bonus `Toggle`'s `toggled` and the two Move buttons' `clicked`, call `scope.focus()` after the action, so a mouse click on them leaves the focus in the panel. (Move rebuilds the panel through `keep_in_sync` in `src/app/gradebook/panel.rs`, and the new panel's `init` focuses the Title field, which is also inside the scope; the call is harmless there.)

### Tests
- `tests/ui.rs` (regressions, each opens the panel on a test column, puts the focus or click in one place, presses Escape and checks `get_panel_index() == -1`): `escape_closes_the_panel_from_the_weight_field`, `…_from_the_out_of_field`, `…_after_clicking_the_bonus_toggle`, `…after_moving_the_column`, `…after_clicking_the_panels_blank_area`, `…from_the_rounds_combo_box` (on a Rounded column, list closed), `…from_the_decimals_field`; and `escape_with_the_rounds_list_open_closes_only_the_list` (first Escape: list closed, panel open; second: panel closed).
- `tests/ui.rs`: `escape_with_a_refused_value_restores_it_and_closes` (type "abc" in Weight, Escape: panel closed, the column's weight is the saved one, no error banner).
- `tests/ui.rs`: `escape_in_a_sheet_cell_leaves_the_panel_open` (open the panel, click a cell, Escape: panel still open).
- `tests/desktop.rs` `the_column_panel_and_menus`: after the existing `app.click("Move right ›")`, press Escape instead of clicking Close, and check `panel_index` is -1, so the real window is covered too.

### Docs
- `docs/user-guide.md` line 33: add "Esc closes the panel." after the list of settings.
- Changelog: "Fixed Escape not closing a column's settings panel after a click in it or from some of its fields".

### Done when
- [ ] Every test above failed before the fix where the bug applied, and passes after (output of both runs).
- [ ] Escape closes the panel in the real window after a Move click (desktop test output).
- [ ] `cargo test`, the desktop suite, the coverage gate, clippy and fmt pass; `metadata -m` run on every edited file.

## 22. Step the column panel to the previous or next column

**Why:** to change the settings of several columns (for example the weight of every test), the user today clicks a header, edits, closes or clicks the next header, and so on, reaching for the mouse each time. The user asked for arrows in the column panel that switch to the neighbouring column's settings without closing it. Do this after item 21, which makes the panel's key handler a `capture-key-pressed` that sees keys first.
**Goal:** two small arrow buttons in the panel's title row, and Alt+Left / Alt+Right, open the settings of the column to the left / right, keeping the panel open.

### Behaviour
- The panel's title row (`ui/column-panel.slint`, the `HorizontalLayout` with the title and the "×" `IconButton`) gets two `IconButton`s before the "×": "‹" (accessible label "Previous column (Alt+←)") and "›" ("Next column (Alt+→)").
- Clicking "›" or pressing Alt+→ opens the next column's settings in the panel: same as clicking that column's header today (`Gradebook.open-column-panel(index + 1)`). "‹" / Alt+← does the same with `index - 1`.
- On the first column "‹" is dimmed (`enabled: false`, which `IconButton` already greys out) and Alt+← does nothing; on the last column the same for "›" / Alt+→. No wrap-around, matching the Move buttons, so the user can tell they've reached the end.
- Alt and not plain arrows, because plain Left/Right move the cursor in the panel's text fields. Alt+arrows aren't used by the fields, and they can't clash with the configurable shortcuts (`src/shortcuts.rs` `Key` has no arrow keys).
- The new panel focuses its Title field, as when opened by a click (its `init`). A refused half-typed value in the old panel is put back before switching (the field loses focus), as when clicking another header today.
- The sheet scrolls sideways, if needed, so the column whose settings are shown is visible, as clicking a header requires it to be visible already. Check how the sheet's horizontal `ScrollView` / `Flickable` is set in `ui/sheet-page.slint` and set its `viewport-x` so the column is in view; if the column is already fully visible, it doesn't scroll.

### Out of scope
- Moving the column itself (item 23).
- Up/Down arrows for pupils.

### Implementation
1. `ui/column-panel.slint`: add the two `IconButton`s (glyphs "‹" and "›", tooltips `@tr("Previous column (Alt+←)")` / `@tr("Next column (Alt+→)")`, `enabled: !root.is-first` / `!root.is-last`), each calling `Gradebook.open-column-panel(root.index - 1)` / `(root.index + 1)`.
2. `ui/column-panel.slint`, in the panel's `capture-key-pressed` (item 21): if `event.modifiers.alt && !event.modifiers.control && !event.modifiers.shift` and `event.text == Key.LeftArrow` (or `Key.RightArrow`), call the same as the buttons when not at the end, and `return accept` in both cases (even at the end, so the key does nothing elsewhere). Put the "not at the end" check in one place: two `function`s `previous()` / `next()` used by the buttons and the keys (DRY).
3. `ui/sheet-page.slint`: when `Gradebook.panel-index` changes, scroll the sheet so that column is visible (`changed` on a property bound to `Gradebook.panel-index`, computing the column's x from the header widths the sheet already uses).
4. `lang/gl/LC_MESSAGES/avalio-lite.po`: "Previous column (Alt+←)" → "Columna anterior (Alt+←)", "Next column (Alt+→)" → "Columna seguinte (Alt+→)".

### Tests
- `tests/ui.rs`: `the_arrow_buttons_step_the_panel_through_the_columns` (from column 0, "›" opens 1, "‹" back to 0; `get_panel_index()` and the shown title follow); `alt_arrows_step_the_panel` (same with Alt+→ / Alt+←, typed while the focus is in the Weight field); `the_ends_dim_the_arrow_and_ignore_the_key` (first column: "‹" disabled, Alt+← leaves index 0; last column: the same for "›"); `stepping_puts_a_refused_value_back` (type "abc" in Weight, Alt+→, back with Alt+←: the saved weight shows, no error banner); `alt_arrows_dont_move_the_column` (the column order is unchanged after stepping).
- `tests/i18n.rs`: the two strings exist in every locale.
- `tests/desktop.rs` `the_column_panel_and_menus`: click "›" in the real window, check `panel_index` went up by one; add "Previous column (Alt+←)" and "Next column (Alt+→)" to the controls checked by `assert_on_screen`. The `column-panel-*` baselines change (new buttons): regenerate them and review each by eye, in light, dark and Galician.

### Docs
- `docs/user-guide.md` line 33: add "‹ and › at the top of the panel (or Alt+← and Alt+→) open the settings of the column to the left or right."
- Changelog: "‹ and › in a column's settings panel, and Alt+← / Alt+→, open the next or previous column's settings without closing the panel".

### Done when
- [ ] Buttons and keys step through the columns, stop at the ends, in the real window (desktop test output + reviewed baselines in light, dark, Galician).
- [ ] The tests above pass; `cargo test`, the desktop suite, the coverage gate, clippy and fmt pass; `metadata -m` run on every edited file.

## 23. Move the column with Ctrl+Left and Ctrl+Right in the column panel

**Why:** the column panel moves its column with the "‹ Move left" / "Move right ›" buttons only (`ui/column-panel.slint`, calling `Gradebook.move-column(root.column.id, -1 / 1)`, handled in `src/app/gradebook/columns.rs` by `columns::move_column`). Moving a column several places means several mouse clicks at the bottom of the panel. The user asked for Ctrl+arrow keys. Do this after item 21 (the panel's `capture-key-pressed`); it is independent of item 22.
**Goal:** with a column's panel open, Ctrl+← and Ctrl+→ move the column one place left or right, exactly like the Move buttons.

### Behaviour
- Ctrl+← = "‹ Move left", Ctrl+→ = "Move right ›": the column swaps with its neighbour, the panel stays open on the same column (at its new index, through `keep_in_sync`), and the change is saved at once as with the buttons.
- On the first column Ctrl+← does nothing, on the last Ctrl+→ does nothing (the buttons are already disabled there); the key is still accepted, so it doesn't reach anything else.
- Trade-off, chosen by the user: inside the panel's text fields Ctrl+←/→ normally jumps the cursor one word. The panel takes these keys first (`capture-key-pressed`), so in the panel's fields they move the column instead. Titles are short, so losing word jumps there costs little; Home/End and plain arrows still move the cursor. Outside the panel (sheet cells, Settings) Ctrl+←/→ keeps its usual meaning.
- The Move buttons' labels show the key so it can be found: "‹ Move left (Ctrl+←)" and "Move right (Ctrl+→) ›" are too wide for the panel in Galician (see `regression_column_panel_buttons_fit_in_galician`), so the keys go in the buttons' accessible labels and in a small faint line under the two buttons: "Ctrl+← / Ctrl+→" (`Theme.text-s`, `Theme.faint`, centred). The text is the same in every language (no `@tr`).
- These keys are fixed, not in **Settings → Keyboard shortcuts**: they act only inside the panel, and the shortcut `Key` set in `src/shortcuts.rs` has no arrow keys (adding arrows there would let users bind them globally, where plain arrows type into cells).
- The sheet keeps the moved column in view (if item 22's scroll-to-column exists, reuse it; it fires because `panel-index` changes).

### Out of scope
- Moving columns from the sheet without the panel open.
- Moving pupils (rows).

### Implementation
1. `ui/column-panel.slint`: add `function move(step: int)` that calls `Gradebook.move-column(root.column.id, step)` unless at that end (`root.is-first` for -1, `root.is-last` for 1). Make both Move buttons call it, so the end check lives in one place (DRY).
2. `ui/column-panel.slint`, in `capture-key-pressed`: if `event.modifiers.control && !event.modifiers.alt && !event.modifiers.shift` and `event.text == Key.LeftArrow` → `root.move(-1)`, `Key.RightArrow` → `root.move(1)`; `return accept` either way.
3. `ui/column-panel.slint`: give the Move buttons' accessible labels the keys (`@tr("Move left (Ctrl+←)")`, `@tr("Move right (Ctrl+→)")`; `ActionButton` takes its accessible label from `text` today, so add an `in property <string> spoken-label: root.text` to `ActionButton` in `ui/components.slint` and use it for `accessible-label`; grep every `ActionButton` use to be sure nothing else changes). Add the faint "Ctrl+← / Ctrl+→" line under the button row.
4. `lang/gl/LC_MESSAGES/avalio-lite.po`: "Move left (Ctrl+←)" → "Mover á esquerda (Ctrl+←)", "Move right (Ctrl+→)" → "Mover á dereita (Ctrl+→)".

### Tests
- `tests/ui.rs`: `ctrl_arrows_move_the_column_and_keep_the_panel_on_it` (on column 0, Ctrl+→ twice: the column is third, `get_panel_index() == 2`, the order is saved, checked by reloading the sheet); `ctrl_arrows_work_from_inside_a_text_field` (focus in Title, Ctrl+←: moved, title text unchanged); `ctrl_arrows_stop_at_the_ends` (first column Ctrl+←, last column Ctrl+→: order unchanged, no error); `ctrl_arrows_outside_the_panel_dont_move_columns` (panel closed, focus in a cell, Ctrl+→: order unchanged).
- `tests/ui.rs`: `regression_column_panel_buttons_fit_in_galician` still passes with the new hint line.
- `tests/i18n.rs`: the two new strings exist in every locale.
- `tests/desktop.rs` `the_column_panel_and_menus`: after the existing click on "Move right ›", press Ctrl+← in the real window and check `state["columns"][0] == "Exam 1"` again. Regenerate the `column-panel-*` baselines (new hint line) and review each in light, dark and Galician.

### Docs
- `docs/user-guide.md` line 33: add "Ctrl+← and Ctrl+→ move the column while its panel is open." and add the two keys to the Keyboard section as a note under the shortcut table ("In a column's panel: Ctrl+←/→ moves the column; these are fixed.").
- Changelog: "Ctrl+← and Ctrl+→ move a column left or right while its settings panel is open".

### Done when
- [ ] Ctrl+←/→ moves the column in the real window (desktop test output) and stops at the ends.
- [ ] Galician buttons still fit; baselines reviewed in light, dark and Galician.
- [ ] `cargo test`, the desktop suite, the coverage gate, clippy and fmt pass; `metadata -m` run on every edited file.

## 30. Write docs/design-decisions.md with the structure decisions

**Why:** the project has no record of why the code is laid out as it is. `docs/development.md` *Layout* says what lives where, never why, so each structure review starts over (this backlog's items 25–37 came from one). The source-organization rules ask for the structure and its reasons to live in a design-decisions doc. Do this first in the structure block: items 25–37 each mark their decision done here.
**Goal:** `docs/design-decisions.md` lists each structure decision as a short ADR (architecture decision record): context, decision, consequences, status. It is linked from the README's docs table and from `docs/development.md`.

### Behaviour
- Each decision has a number (D1…), a title, **Context** (the problem, with file names), **Decision**, **Consequences** (what gets easier, what it costs) and **Status**: `Accepted (TODO item N)` until its item ships, then `Done`.
- Decisions are never deleted. A reversed decision gets status `Superseded by Dn`.

### Out of scope
- Product decisions (column types, grade formulas): those are in `docs/user-guide.md`.

### Implementation
1. New `docs/design-decisions.md` (`metadata -c`), with a one-paragraph intro (what an entry is, the status rules above) and these entries:
   - **D1 Package by feature, with a composition root and a window kit.** Each feature folder (`gradebook/`, `spreadsheet/`, `backup/`, `shortcuts/`, `lock/`) holds its logic plus a `ui` submodule wiring it to Slint. `app/` only starts the app and wires the features together. `window/` is the shared UI kit every feature UI uses (callback wrappers, shared handles, dialogs, the status and error line, theme, language). Context: `src/app/` mirrored the feature folders (`app/gradebook` ↔ `gradebook`, `app/backup.rs` ↔ `backup.rs`, `app/shortcuts.rs` ↔ `shortcuts.rs`), so a change to one feature edited two trees. Consequence: one folder per feature change; the `window/` kit exists to break the `app` ↔ feature-UI cycle. Status: Accepted (item 31).
   - **D2 The app state is the core, outside the UI.** `state/` owns the settings, the open database, its PIN, data moves and backups, with no Slint code. Its PIN methods stay in `state/protection.rs`, not `lock/`, because they need `AppState`'s private fields. Status: Accepted (item 25).
   - **D3 Shared infrastructure under `platform/`.** `config`, `db`, `logging` and `storage` serve every feature, so they sit together under one folder. The top level then reads as the app's features (Screaming Architecture). `error` and `i18n` stay at the top: every layer uses them. Status: Accepted (item 32).
   - **D4 Dependencies point one way.** `app → feature ui → window → state → feature logic → platform → error`. Logic never imports Slint, `crate::ui`, `app`, `window` or any `ui` module. Core logic (`gradebook`, `platform/db`, `platform/storage`, `lock`) doesn't import `i18n`; it uses ports (traits) the caller fills, such as `ColumnTitles`. No module cycles. Enforced by `tests/architecture.rs`. Status: Accepted (items 26, 27, 29).
   - **D5 Files split at about 200 lines, by what changes together.** Larger files are split along the lines of change, never into arbitrary halves. Public paths stay through `pub use` in the folder's `mod.rs`, so callers don't change. Allowed exceptions, with reasons: `state/protection.rs` and `platform/logging.rs` carry `#[cfg(test)] mod tests` that need private access, so their code is under 200 lines. Status: Accepted (items 34, 35, 28, 36).
   - **D6 No repeated code.** The "no database open" guards, the encrypted-file page check and the "log a warning and show the error" pair each live in one helper. Status: Accepted (item 33).
2. `README.md` docs table (line 233 area): add a row `| [Design decisions](docs/design-decisions.md) | Why the code is laid out as it is: feature folders, the dependency rules, file sizes |`.
3. `docs/development.md`, *Layout*: first line under the heading: "Why it is laid out like this: [design decisions](design-decisions.md)."
4. `metadata -m` on `README.md` and `docs/development.md`.

### Tests
- Docs only. `cargo test` still passes; check the new links resolve (open the README and `docs/development.md` previews in VS Code and click them).

### Docs
- This item is the docs. Changelog: none (developer docs only).

### Done when
- [ ] `docs/design-decisions.md` exists with D1–D6, each with context, decision, consequences and status.
- [ ] The README table and `docs/development.md` link to it; `metadata` run on all three files.

## 25. Move the app state out of the UI layer and split it by concern

**Why:** `src/app/` is the Slint adapter: every file in it wires the window to the logic. But `src/app/state/` (`AppState`, 467 lines in `mod.rs` plus 252 in `protection.rs`) has no UI code at all: it never imports `slint` or `crate::ui`, and it is the app's core (settings, the open database, its PIN, data moves, backups). Living under `app/` hides it from someone reading the tree, makes the UI layer look like it owns the logic, and keeps it out of mutation testing by accident (`.cargo/mutants.toml` excludes `src/app/**`). `state/mod.rs` also changes for five unrelated reasons (startup, preferences and shortcuts, moving the data or logs, backups, the connection lifecycle), well past the ~200-line god-file threshold, so a change to one concern means scrolling through the others. Do this right after item 30: the other structure items assume the new paths.
**Goal:** `src/state/` at the crate root holds `AppState`, split into one file per concern, each under ~200 lines; `src/app/` holds only UI wiring. No behaviour changes.

### Behaviour
- The app behaves exactly as before: same screens, same files written, same log lines (only their module target changes, from `avalio_lite::app::state…` to `avalio_lite::state…`).
- Public API: `avalio_lite::state::{AppState, Startup, SetupReason, Opened}` replaces `avalio_lite::app::state::…`. No re-export from the old path is kept: an alias would let new code keep reaching the core through the UI layer.

### Out of scope
- Renaming `AppState` or any method: the diff stays a pure move, so review compares positions, not logic.
- Making `SetupReason` carry the `AppError` instead of already translated text (`AppError` isn't `Clone`; separate work if wanted).
- Any change to `src/app/` files beyond their import lines.

### Implementation
1. `git mv src/app/state src/state`, so history follows the files.
2. `src/lib.rs`: add `pub mod state;` (alphabetical, after `spreadsheet`). `src/app/mod.rs`: remove `pub mod state;`; change `use state::{AppState, Startup};` to `use crate::state::{AppState, Startup};`.
3. Split `src/state/mod.rs` along what changes together. Rust allows several `impl AppState` blocks in child modules (as `protection.rs` already does); private items of `mod.rs` are visible to its children, and a method one sibling needs from another gets `pub(super)`:
   - `mod.rs`: the `AppState` struct, `new`, `config`, `connection`, `shutdown`, and the shared private helpers `activate`, `close_connection`, `save_config`, `replace_connection`, `reopen_logger`; the `mod` lines for the files below; `pub use startup::{Opened, SetupReason, Startup};`.
   - `startup.rs`: `SetupReason`, `Startup`, `Opened`, `start`, `create_database`, `open_existing`, `choose`, `FileKind`, `existing_kind`, `require_path`, `MIN_DATABASE_BYTES`, `SMALLEST_PAGE`.
   - `preferences.rs`: `preferences`, `set_show_column_settings`, `set_theme`, `set_language`, `shortcuts`, `set_shortcut`, `reset_shortcut`, `reset_all_shortcuts`, `save_shortcuts`, `take_shortcut_problems`, `settings`, and `adopt_shortcuts` and `shortcut_overrides` (both `pub(super)`, used by `startup.rs`).
   - `locations.rs`: `move_database`, `move_logs`, `move_rotated_log`.
   - `backups.rs`: `export_backup`, `open_backup`, `restore_backup`, `add_backup_classes`, `RESTORE_SUFFIX`.
   - `protection.rs`: unchanged apart from imports. `replace_with` (used by `restore_backup`) is already `pub(super)`, so `backups.rs`, a child of `state`, can call it.
   Move every doc comment with its item; keep each file's imports to what it uses (clippy's `unused_imports` says).
4. Update every `crate::app::state` import: `src/app/callbacks.rs`, `src/app/lock_screen.rs`, `src/app/view.rs`, and any other hit of `grep -rn 'app::state' src`.
5. Update tests and fixtures to `use avalio_lite::state::…`: `tests/common/mod.rs`, `tests/backup.rs`, `tests/encryption.rs`, `tests/error_logging.rs`, `tests/state.rs` (including line 477's `avalio_lite::app::state::Opened`). In `tests/error_logging.rs` change the log targets `"avalio_lite::app::state"` (lines 48, 59, 71, 163) to `"avalio_lite::state"`; `has_line` matches `"<target>:"`, which also matches the submodule targets `avalio_lite::state::startup:` and `avalio_lite::state::protection:`.
6. `scripts/coverage.ps1` comments (lines 14 and 21): `src\app\state\mod.rs` → the file each "did not close cleanly" line now lives in (`src\state\mod.rs` for `close_connection`, `src\state\locations.rs` for `move_database`); `src\app\state\protection.rs` → `src\state\protection.rs`.
7. `.cargo/mutants.toml`: `exclude_globs` needs no change; `src/state/` is now inside mutation testing on purpose. Add a comment above it: `src/app/**` is the UI wiring only; the app state lives in `src/state/`.
8. Then `grep -n '<old path>' TODO.md` for every path this item moved and update the items still open, so the next implementer isn't sent to a file that is gone.
9. `metadata -c` on each new `.rs` file and `metadata -m` on every edited one; `cargo fmt`; `cargo clippy --all-targets -- -D warnings`.

### Tests
- No new behaviour, so no new cases. The proof is the existing suite passing with only imports and log targets changed: `tests/state.rs`, `tests/encryption.rs`, `tests/backup.rs`, `tests/error_logging.rs`, `tests/ui.rs`, `tests/entry_point.rs`.
- `.\scripts\coverage.ps1 -Branches` still passes its gates (a move must not uncover lines).
- `cargo test --test desktop -- --ignored --test-threads=1` passes with no baseline changes.

### Docs
- `docs/development.md`, *Layout*: move the `state/` line from under `app/` to the top level, as `state/  the app's core: settings, the open database and its PIN, data moves, backups; no UI code`; reword the `app/` line to "the Slint adapter: window startup and the UI callbacks of each screen".
- `docs/development.md`, *Tests*, mutation-testing bullet: after "(not `src/app/`)" add "; `src/state/` is included".
- `docs/design-decisions.md`: D2 status → Done.
- Changelog (Changed): "The app's core state moved out of the UI code and was split by concern; no visible change."

### Done when
- [ ] `grep -rn 'app::state' src tests examples scripts` finds nothing.
- [ ] Every file in `src/state/` is under ~200 lines (`wc -l src/state/*.rs`).
- [ ] `grep -rn 'slint\|crate::ui' src/state` finds nothing.
- [ ] `cargo test`, the desktop suite, `coverage.ps1 -Branches`, clippy and fmt pass; `metadata` run on every new and edited file.

## 31. Group each feature's logic and UI wiring into one folder

**Why:** the code is split by layer at the top: `src/app/` holds the Slint wiring of every feature, mirroring the logic folders (`src/app/gradebook/` ↔ `src/gradebook/`, `src/app/backup.rs` ↔ `src/backup.rs`, `src/app/shortcuts.rs` ↔ `src/shortcuts.rs`, `src/app/lock_screen.rs` ↔ `src/lock.rs`, `src/app/gradebook/spreadsheet.rs` ↔ `src/spreadsheet/`). Every feature change edits two trees, and nothing shows which UI file belongs to which feature. The user chose package by feature (decision D1 in `docs/design-decisions.md`, item 30). One problem must be solved on the way: every feature UI imports `crate::app` (`SharedState`, `SharedDialogs`, `with_window*`, `alive`, `view`), while `app` imports every feature UI to wire it, so a plain move would create cycles. A shared `window/` kit breaks them. After item 25.
**Goal:** each feature folder holds its logic and a `ui` submodule; `src/window/` is the shared UI kit; `src/app/` only starts the app and wires the features. No behaviour changes.

### Behaviour
- The app behaves exactly as before. Log lines keep their text; their module targets change (listed in step 9).
- Public paths of the logic modules don't change (`avalio_lite::gradebook::…`, `avalio_lite::backup::…`, `avalio_lite::lock::…`, `avalio_lite::shortcuts::…`, `avalio_lite::spreadsheet::…`). UI paths change: `avalio_lite::app::dialogs` → `avalio_lite::window::dialogs`, `avalio_lite::app::with_window` → `avalio_lite::window::with_window`; `avalio_lite::app::{run, run_with, open, Session}` stay.

### Out of scope
- `config`, `db`, `logging`, `storage`: item 32.
- Splitting large files: items 34–36.
- Any logic change.

### Implementation
1. Use `git mv` for every move below, so history follows the files.
2. New `src/window/` (the UI kit; `pub mod window;` in `src/lib.rs`):
   - `mod.rs`: from `src/app/mod.rs` move `SharedState`, `SharedDialogs`, the `window_callbacks!` macro with its `with_window*` functions, and `alive` (make it `pub`; `classes.rs` and `pupils.rs` call it).
   - `dialogs.rs`, `theme.rs`, `language.rs`: moved from `src/app/`.
   - `view.rs`: moved from `src/app/view.rs`. Every feature UI uses its status and error helpers, and `src/lock/ui.rs` uses `show_setup`.
   - `model` (the `ModelRc` helper, `src/app/gradebook/view.rs:293`) moves into `window/view.rs`. It is generic, and `src/app/shortcuts.rs` imports it from the gradebook UI; left there, it would make `shortcuts::ui` → `gradebook::ui` → `shortcuts::ui` a cycle. Update its callers to `crate::window::view::model`: `src/app/gradebook/panel.rs` (lines 45, 51), `src/app/gradebook/view.rs` (96, 110–112, 144, 237) and `src/app/shortcuts.rs` (105, 112).
3. Feature UIs:
   - `src/app/gradebook/` → `src/gradebook/ui/` (`mod.rs`, `cells.rs`, `classes.rs`, `columns.rs`, `panel.rs`, `pupils.rs`, `view.rs`); declare `pub mod ui;` in `src/gradebook/mod.rs`. Make `run` and `view` `pub(crate)`, since `spreadsheet::ui` uses them.
   - `src/app/gradebook/spreadsheet.rs` → `src/spreadsheet/ui.rs`. Remove `spreadsheet::wire` from `gradebook::ui::wire`; `app` calls it instead (step 5), because `spreadsheet::ui` uses `gradebook::ui` and the reverse would be a cycle.
   - `src/backup.rs` → `src/backup/mod.rs`; `src/app/backup.rs` → `src/backup/ui.rs`.
   - `src/shortcuts.rs` → `src/shortcuts/mod.rs`; `src/app/shortcuts.rs` → `src/shortcuts/ui/mod.rs` (a folder, because item 27 adds `key_events.rs` beside it).
   - `src/lock.rs` → `src/lock/mod.rs`; `src/app/lock_screen.rs` → `src/lock/ui.rs`.
   - Each `ui` module is `pub mod ui;` (the tests reach `shortcuts::ui` through item 27). Don't confuse it with `crate::ui`, the module Slint generates (`src/lib.rs`): UI files keep importing that one by its full path, `use crate::ui::…`. Logic files never import `crate::ui`, so a feature's `pub mod ui;` can't clash with it.
4. Split `src/app/callbacks.rs` by screen: `src/app/setup.rs` (`wire_setup_browsing`, `wire_setup_actions`) and `src/app/settings.rs` (`wire_settings`, `wire_language`, `after_data_change`, `after_database_opened`, `as_path`). Both helpers go in `settings.rs`: the Settings screen uses both (`move_database` and `move_logs` call `after_data_change`, `switch_database` calls `after_database_opened`), while setup uses only `after_database_opened` and `as_path`. Make those two `pub(super)`, so `setup` → `settings` is the only direction between the two files.
5. `src/app/mod.rs` keeps `run`, `run_with`, `launch`, `open`, `Session`, `saved_preferences` and `show_startup`. `open` wires `setup`, `settings`, `backup::ui`, `gradebook::ui`, `spreadsheet::ui`, `lock::ui` and `shortcuts::ui`, in the order the current code wires them.
6. Rewrite each moved file's imports: `crate::app::{SharedState, SharedDialogs, with_window…, alive}` → `crate::window::…`; `crate::app::view` → `crate::window::view`; `crate::app::dialogs` → `crate::window::dialogs`; `crate::app::gradebook` → `crate::gradebook::ui`; `crate::app::lock_screen` → `crate::lock::ui`; `crate::app::shortcuts` → `crate::shortcuts::ui`; `super::{SharedState, run}` in the old `app/gradebook/spreadsheet.rs` → `crate::window::SharedState` and `crate::gradebook::ui::run`. `cargo build` lists anything missed.
7. Check the result has no cycle among the UI modules. The intended direction: `app` → (`backup::ui`, `lock::ui`, `spreadsheet::ui`, `gradebook::ui`, `shortcuts::ui`) → `window`; `backup::ui` → `gradebook::ui`, `lock::ui`; `lock::ui` → `gradebook::ui`; `spreadsheet::ui` → `gradebook::ui`; `gradebook::ui` → `shortcuts::ui` (its `show_database` shows the database's shortcuts). Nothing points back up that list. Check with `grep -rn 'crate::app' src`: only `src/app/` and `src/main.rs` may match.
8. Tests and examples: `tests/common/mod.rs`, `tests/backup.rs`, `tests/ui.rs`, `tests/i18n.rs:386`: `app::dialogs` → `window::dialogs`; `tests/window_lifetime.rs:17`: `app::with_window` → `window::with_window`. `examples/app_driver.rs` keeps `app::{self, Session}`.
9. Log targets: `tests/window_lifetime.rs:45` `avalio_lite::app: callback after the window closed` → `avalio_lite::window:`. In `tests/error_logging.rs`: `"avalio_lite::app::gradebook"` (line 80) → `"avalio_lite::gradebook::ui"`; `"avalio_lite::app::callbacks"` (lines 89, 144) → `"avalio_lite::app::settings"`; `"avalio_lite::app::lock_screen"` (lines 105, 120, 131) → `"avalio_lite::lock::ui"`.
10. `.cargo/mutants.toml`: `exclude_globs = ["src/app/**", "src/window/**", "src/*/ui.rs", "src/*/ui/**", "src/main.rs"]`, with the comment saying these are the UI wiring.
11. `scripts/coverage.ps1` comments (lines 9–22): `src\app\dialogs.rs` → `src\window\dialogs.rs`; `src\app\language.rs` → `src\window\language.rs`; `lock_screen.rs` → `src\lock\ui.rs`; `src\app\backup.rs` → `src\backup\ui.rs`; `src\backup.rs` → `src\backup\mod.rs`. If the script excludes UI files from the line count by path, update those patterns the same way.
12. Files that items done before this one added to `src/app/` (items 24, 15 and 18–23 may add some): place each by the same rule. A feature's wiring goes to that feature's `ui`; code several features share goes to `window/`; startup and the setup and settings screens stay in `app/`. Then `grep -n '<old path>' TODO.md` for every path this item moved and update the items still open, so the next implementer isn't sent to a file that is gone.
13. `metadata -c` on new files (`setup.rs`, `settings.rs`, `window/mod.rs`); `metadata -m` on every moved and edited file; `cargo fmt`; `cargo clippy --all-targets -- -D warnings`.

### Tests
- No new behaviour. The existing suite passes with only imports and log targets changed (step 8 and 9): especially `tests/ui.rs`, `tests/error_logging.rs`, `tests/window_lifetime.rs`, `tests/entry_point.rs`, `tests/entry_point_failure.rs`.
- `.\scripts\coverage.ps1 -Branches` passes its gates.
- `cargo test --test desktop -- --ignored --test-threads=1` passes with no baseline changes.

### Docs
- `docs/development.md`, *Layout*: rewrite the `src/` tree: `app/` "starts the app and wires the features (setup and settings screens)"; `window/` "the UI kit every feature UI uses: callback wrappers, shared handles, dialogs, status and error line, theme, language"; each feature line gains "; `ui` wires it to the window".
- `docs/development.md`, *Tests*, mutation-testing bullet: "(not `src/app/`)" → "(not the UI wiring: `src/app/`, `src/window/` and each feature's `ui`)".
- `docs/design-decisions.md`: D1 status → Done.
- Changelog (Changed): "Source reorganised by feature; no visible change."

### Done when
- [ ] `src/app/` holds only `mod.rs`, `setup.rs`, `settings.rs` and any startup file step 12 placed there on purpose.
- [ ] `grep -rn 'crate::app' src` matches only `src/app/` and `src/main.rs`.
- [ ] Full suite, desktop suite, `coverage.ps1 -Branches`, clippy and fmt pass; `metadata` run on every new, moved and edited file.

## 32. Group the shared infrastructure under src/platform/

**Why:** after item 31 the top of `src/` mixes the app's features (`gradebook`, `spreadsheet`, `backup`, `shortcuts`, `lock`, `state`) with plumbing every feature uses (`config.rs`, `db/`, `logging.rs`, `storage.rs`). The tree doesn't show what the app is about. Decision D3 in `docs/design-decisions.md` (item 30) puts the plumbing in one folder. After item 31.
**Goal:** `src/platform/` holds `config.rs`, `db/`, `logging.rs` and `storage.rs`; the top level reads as `app`, `window`, the features, `platform`, `error`, `i18n`. No behaviour changes.

### Behaviour
- No visible change. Public paths become `avalio_lite::platform::{config, db, logging, storage}`. No re-export is kept at the old paths, so new code can't bypass the folder.

### Out of scope
- `error.rs` and `i18n/`: every layer uses them, so they stay at the top (D3).

### Implementation
1. `git mv src/config.rs src/db src/logging.rs src/storage.rs` into a new `src/platform/`; new `src/platform/mod.rs` with `pub mod config; pub mod db; pub mod logging; pub mod storage;` and a `//!` line: "Plumbing every feature uses: the settings file, the database, the log file, safe file moves." `src/lib.rs`: replace the four `pub mod` lines with `pub mod platform;`.
2. `src/platform/db/migrations.rs` lines 14–17: `include_str!("../../migrations/…")` → `include_str!("../../../migrations/…")`. The file is now one folder deeper, and the build fails until this is fixed.
3. Replace `crate::config`, `crate::db`, `crate::logging` and `crate::storage` with `crate::platform::…` everywhere: `grep -rln 'crate::\(config\|db\|logging\|storage\)' src`.
4. Tests and examples: replace `avalio_lite::config` (18 uses), `avalio_lite::db` (21), `avalio_lite::logging` (8) and `avalio_lite::storage` (2) with `avalio_lite::platform::…` (`grep -rn 'avalio_lite::\(config\|db\|logging\|storage\)' tests examples`).
5. Log targets: `grep -rn '"avalio_lite::\(db\|config\|logging\|storage\)' tests` and add `platform::` to each match. `logging.rs`'s `APP_TARGET_PREFIX` (`"avalio_lite"`) still matches every target, so it needs no change.
6. `scripts/coverage.ps1` comments: `src\db\mod.rs` → `src\platform\db\mod.rs`; check its other path patterns too. `docs/development.md` *Translations and schema changes*: `src/db/migrations.rs` → `src/platform/db/migrations.rs`. Then `grep -n '<old path>' TODO.md` for every path this item moved and update the items still open, so the next implementer isn't sent to a file that is gone.
7. `metadata -c src/platform/mod.rs`; `metadata -m` on every moved and edited file; `cargo fmt`; `cargo clippy --all-targets -- -D warnings`.

### Tests
- No new behaviour. The full suite passes with only paths changed; `tests/golden.rs` and the migration walk in `tests/db.rs` prove the moved `include_str!` paths still embed the same SQL.
- `coverage.ps1 -Branches` and the desktop suite pass.

### Docs
- `docs/development.md`, *Layout*: a `platform/` entry with `config.rs`, `db/`, `logging.rs` and `storage.rs` indented under it, keeping their current descriptions.
- `docs/design-decisions.md`: D3 status → Done. Changelog: covered by item 31's line.

### Done when
- [ ] `ls src` shows `app`, `backup`, `error.rs`, `gradebook`, `i18n`, `lib.rs`, `lock`, `main.rs`, `platform`, `shortcuts`, `spreadsheet`, `state`, `window` and nothing else.
- [ ] `grep -rn 'crate::\(config\|db\|logging\|storage\)\b' src` finds nothing.
- [ ] Full suite, desktop suite, coverage gate, clippy and fmt pass; `metadata` run on every moved and edited file.

## 26. Stop the gradebook logic from depending on the translations

**Why:** `src/gradebook/` is documented as "no UI code", yet `src/gradebook/columns.rs:14` imports `crate::i18n::{Language, default_column_title}` so `add_column` can name a new column ("Test 3" in English, its Galician equivalent), while `src/i18n/messages.rs` imports `crate::gradebook::model::ColumnType`. That is a cycle between the two packages (gradebook → i18n → gradebook), against the acyclic-dependencies rule: the core logic can't be read, tested or changed without the translation tables. It touches only `src/gradebook/` and `src/i18n/`, which items 25 and 31 don't move, so its paths hold in this position.
**Goal:** `src/gradebook/` imports nothing from `src/i18n/`; the dependency runs one way, i18n → gradebook. New columns get exactly the same default titles as today.

### Behaviour
- A new column's title is unchanged in English and Galician, including the numbering that skips a title already taken ("Test 3" when "Test 2" exists after "Test 1" was deleted).
- Every existing caller compiles unchanged, because `Language` implements the new trait (step 2).

### Out of scope
- The other `i18n` users outside `src/gradebook/`: `src/config.rs` stores a `Language`, and `src/spreadsheet/export.rs` writes translated headers. Both are edges that may depend on i18n.

### Implementation
1. `src/gradebook/model.rs`: add a port the gradebook owns, next to `ColumnType`:
   ```rust
   /// Names new columns: the gradebook numbers them, the caller says how a title reads.
   pub trait ColumnTitles {
       fn default_title(&self, column_type: ColumnType, number: usize) -> String;
   }
   ```
2. `src/i18n/messages.rs`: `impl ColumnTitles for Language`, calling the existing `default_column_title(*self, column_type, number)`. Keep `default_column_title`; check its other callers with `grep -rn default_column_title src tests`.
3. `src/gradebook/columns.rs`: remove the `crate::i18n` import; change `add_column(…, language: Language)` to `add_column(…, titles: impl ColumnTitles)` and `default_title(…, language: Language)` to take `titles: &impl ColumnTitles`, calling `titles.default_title(column_type, number)`. The 59 call sites (`src/app/gradebook/columns.rs`, `examples/demo_data.rs`, `tests/backup.rs`, `tests/common/mod.rs`, `tests/golden.rs`, `tests/gradebook.rs`, `tests/spreadsheet.rs`, `tests/ui.rs`) pass a `Language` by value and keep compiling; confirm with `cargo test --no-run` and `cargo build --examples`.
4. `metadata -m` on every edited file; `cargo fmt`; `cargo clippy --all-targets -- -D warnings`.

### Tests
- `tests/gradebook.rs`, new `add_column_names_columns_through_the_given_titles`: a test-only `struct Numbered;` implementing `ColumnTitles` as `format!("{column_type:?}#{number}")`. Adding two Test columns gives `Test#1` and `Test#2`; deleting `Test#1` and adding another gives `Test#3` (one Test left → tries 2, taken → 3); adding an Info column gives `Info#1`. Proves the gradebook only numbers and the caller names.
- `tests/i18n.rs`, new `language_names_columns_like_default_column_title`: for every `ColumnType` and numbers 1–3, `Language::Galician.default_title(t, n) == default_column_title(Language::Galician, t, n)`, and the same for English. Proves the adapter is a straight pass-through.
- The existing default-title tests in `tests/gradebook.rs` and `tests/golden.rs` pass unchanged (same titles).

### Docs
- `docs/development.md`, *Layout*, `gradebook/` line: append "; names new columns through `ColumnTitles`, so it never imports `i18n`". Changelog: none (no visible change).

### Done when
- [ ] `grep -rn 'crate::i18n' src/gradebook` finds nothing.
- [ ] Both new tests and the full suite pass; a new column in the running app is still titled as before in English and Galician.
- [ ] clippy and fmt clean; `metadata -m` run on every edited file.

## 27. Keep Slint's key events out of the shortcut rules

**Why:** `src/shortcuts/mod.rs` (the old `src/shortcuts.rs`, moved by item 31) holds the shortcut rules, which are pure logic: which keys are allowed, Ctrl+Alt refused, the `config.toml` format. But it imports `slint::platform::Key as SlintKey` and turns Slint's key-event text into a `Key` (the third column of `NAMED_KEYS`, `FUNCTION_KEYS`, `Key::from_event_text`, `KeyCombo::from_key_press`, `KeyCombo::checked_key_press`). So the rules depend on the UI framework, against decision D4: a Slint upgrade that changes its key characters edits the logic file, and the rules can't be built without Slint. After item 31.
**Goal:** `src/shortcuts/mod.rs` never mentions Slint; turning a Slint key event into a `KeyCombo` lives in the feature's UI side, `src/shortcuts/ui/key_events.rs`. Shortcuts work exactly as before.

### Behaviour
- Every shortcut fires and records as today: letters as typed ("l", or "L" with Shift), digits, comma, period, F1–F12, PageUp, PageDown, Home, End; the same refusals (`NeedsCtrlOrAlt`, `CtrlAlt`, `EditingKey`, `UnknownKey`).

### Out of scope
- Changing which keys are allowed, or the `config.toml` format.

### Implementation
1. `src/shortcuts/mod.rs`: rename the private `Key::parse` to `pub fn from_name(text: &str) -> Option<Key>`. It reads `config.toml` key names and is the UI side's fallback. Update its callers in the file: `from_event_text` until it moves, and `FromStr for KeyCombo`.
2. `src/shortcuts/mod.rs`: drop the `SlintKey` column from `NAMED_KEYS` (it becomes `[(Key, &str); 4]`; fix the tuple patterns in `all`, `name` and `from_name`). Delete `FUNCTION_KEYS`, `Key::from_event_text`, `KeyCombo::from_key_press` and `KeyCombo::checked_key_press`, and remove the `use slint::…` line.
3. New `src/shortcuts/ui/key_events.rs`, declared `pub mod key_events;` in `src/shortcuts/ui/mod.rs` (public because `tests/shortcuts.rs` drives it). It holds the Slint table `[(Key, SlintKey); 4]` for PageUp, PageDown, Home and End, the `FUNCTION_KEYS` array, and these functions, with their doc comments moved along:
   - `pub fn key(text: &str) -> Option<Key>`: the old `from_event_text`, falling back to `Key::from_name`;
   - `pub fn combo(text: &str, ctrl: bool, shift: bool, alt: bool) -> Option<KeyCombo>` (was `from_key_press`);
   - `pub fn checked_combo(text: &str, ctrl: bool, shift: bool, alt: bool) -> Result<KeyCombo, ShortcutError>` (was `checked_key_press`), calling `KeyCombo::new`.
4. `src/shortcuts/ui/mod.rs` (the old `src/app/shortcuts.rs` lines 26 and 34): call `key_events::combo` and `key_events::checked_combo`.
5. `tests/shortcuts.rs` lines 124 and 182–186: call `avalio_lite::shortcuts::ui::key_events::{combo, checked_combo}`.
6. `metadata -c src/shortcuts/ui/key_events.rs`; `metadata -m` on the edited files; `cargo fmt`; `cargo clippy --all-targets -- -D warnings`.

### Tests
- `tests/shortcuts.rs`: the existing key-press tests pass through the new functions (same inputs, same results).
- `tests/shortcuts.rs`, new `every_allowed_key_round_trips_through_its_name`: for every `Key::all()`, `Key::from_name(&key.name()) == Some(key)`, and the same with the name lower-cased. Proves the now-public parser and the trimmed `NAMED_KEYS` agree.
- `tests/shortcuts.rs`, new `slint_special_keys_map_to_their_keys`: `key_events::key` on `char::from(SlintKey::F5).to_string()` is `Some(Key::Function(5))`, on `SlintKey::Home` is `Some(Key::Home)`, on `"q"` is `Some(Key::Letter('Q'))`, and on `" "` is `None`.
- The shortcut tests in `tests/ui.rs` and the desktop suite pass unchanged.

### Docs
- `docs/development.md`, *Layout*, `shortcuts/` line: "keyboard shortcuts: the actions, their keys and the rules for config.toml, with no Slint code; `ui/` wires them to the window and reads key events (`ui/key_events.rs`)". Changelog: none (no visible change).

### Done when
- [ ] `grep -n slint src/shortcuts/mod.rs` finds nothing.
- [ ] The new tests and the full suite pass; in the running app Ctrl+L locks and recording a new shortcut in Settings works (desktop suite).
- [ ] clippy and fmt clean; `metadata` run on every new and edited file.

## 33. Remove the repeated code in the app state and the UI error reporting

**Why:** three things are written out again and again, so a change to one has to be made in every copy:
- `self.config.clone().ok_or(AppError::NoDatabaseOpen)?`, `self.config.as_ref().ok_or(…)?` and `self.connection.as_ref().ok_or(…)?` appear 13 times in `AppState` (today `src/app/state/mod.rs` lines 115, 254, 260, 266, 274, 282, 310, 318, 341, 346 and `protection.rs` 36, 112, 113). `set_theme`, `set_language` and `set_show_column_settings` are the same clone, change one field, save body three times.
- The "an encrypted file must be whole 512-byte pages" check is written twice, each copy with its own `SMALLEST_PAGE = 512` (`src/app/state/mod.rs:29` and `:453`, `src/backup.rs:28` and `:134`).
- The UI reports a failure with the same two lines, `log::warn!("<action> failed: {error}");` then `view::show_error(window, &error);`, in 10 places in `src/app/`.
After items 25, 31 and 32 (the paths below are the post-move ones).
**Goal:** each of the three lives in one helper; behaviour, log lines and messages are unchanged.

### Behaviour
- No visible change, and every log line keeps its text, level and module target.

### Out of scope
- The `after_*` helpers (`app/settings.rs`, `lock/ui.rs`, `shortcuts/ui/mod.rs`): each does different follow-up work. Only their warn-and-show lines use the new macro.

### Implementation
1. `src/state/mod.rs`: add `fn open_config(&self) -> Result<&AppConfig>` and `fn open_connection(&self) -> Result<&Connection>` (both `NoDatabaseOpen` when absent), and `fn update_config(&mut self, change: impl FnOnce(&mut AppConfig)) -> Result<()>`, which clones the config, applies `change`, then `save_config`s it. They are private to `state`, so its child modules can use them.
2. Replace the 13 guards in `src/state/{preferences,locations,backups,protection}.rs` with `open_config()?` / `open_connection()?`, adding `.clone()` only where the code needs an owned copy (`move_database`, `move_logs`). Rewrite `set_theme`, `set_language`, `set_show_column_settings` and `save_shortcuts` on `update_config`. `save_shortcuts` must still take the new shortcuts only after a successful save.
3. `src/platform/db/mod.rs`: add `pub const SMALLEST_PAGE: u64 = 512;` (with the existing doc comment) and `pub fn has_whole_pages(path: &Path) -> Result<bool>`, which reads the length with `io_at(path)` and checks `length % SMALLEST_PAGE == 0`. Use it in `existing_kind` (`src/state/startup.rs`) and `open_checked_copy` (`src/backup/…`); delete both local constants. `existing_kind` reads the length for its own `MIN_DATABASE_BYTES` check as well, so it keeps that `fs::metadata` call.
4. `src/window/mod.rs`: add `macro_rules! report_failure { ($window:expr, $action:expr, $error:expr) => {{ log::warn!("{} failed: {}", $action, $error); $crate::window::view::show_error($window, $error); }} }`, exported to the crate with `pub(crate) use report_failure;`. It is a macro, not a function, because `log::warn!` takes its target from the module it expands in, so log lines keep pointing at the feature that failed (`tests/error_logging.rs` checks them).
5. Replace each warn-and-show pair with `report_failure!(window, <action>, &error);`, keeping the action text exactly. The 10 pairs, at their pre-move places (`grep -rn -B1 'show_error(window, &error)' src` finds them after the moves): `app/backup.rs` 62 "export backup", 135 `format!("opening the backup {}", backup.file.display())`, 167 "restore backup", 185 "add classes from a backup"; `app/callbacks.rs` 100 "saving the language", 115 "saving the theme"; `app/gradebook/mod.rs` 45 "saving the column settings preference", 101 `action`; `app/lock_screen.rs` 154 "PIN change"; `app/shortcuts.rs` 80 `action`. Leave `after_data_change` as it is: it shows the locations between the log line and the error.
6. `metadata -m` on every edited file; `cargo fmt`; `cargo clippy --all-targets -- -D warnings`.

### Tests
- `tests/state.rs`, new `preference_changes_without_a_database_are_refused`: on a fresh `AppState` (no database chosen), `set_theme`, `set_language`, `set_show_column_settings` and `set_shortcut` each return `Err(AppError::NoDatabaseOpen)` and write no settings file. Proves `update_config` kept the guard.
- `tests/db.rs`, new `has_whole_pages_checks_the_page_size`: files of 512 and 1024 bytes → `true`; 513 and 100 → `false`; a missing path → `Err` naming the path.
- `tests/error_logging.rs`: the existing target-and-text assertions pass unchanged. That proves `report_failure!` keeps the caller's target.
- `tests/state.rs` and `tests/backup.rs`: the existing tests for "not a database" and partial encrypted files pass unchanged.

### Docs
- `docs/design-decisions.md`: D6 status → Done. Changelog: none (no visible change).

### Done when
- [ ] `grep -rn 'ok_or(AppError::NoDatabaseOpen)' src` shows only the two helpers.
- [ ] `grep -rn 'SMALLEST_PAGE' src` shows only `src/platform/db/mod.rs` and its users.
- [ ] No warn-and-show pair is left (step 5's grep); full suite, coverage gate, clippy and fmt pass; `metadata -m` run.

## 34. Split the shortcut rules, the translations and the sheet view into smaller files

**Why:** three files still pass the ~200-line threshold (decision D5) after the moves, each because it does several jobs:
- `src/shortcuts/mod.rs` (407 lines; about 370 after item 27) holds `Action`, `Key`, `ShortcutError`, `KeyCombo`, `ShortcutProblem` and the `Shortcuts` set.
- `src/i18n/messages.rs` (364) and `src/i18n/errors.rs` (271) each hold an English and a Galician function side by side, plus helpers. Today a new language means editing both files in many places. A translator works one language at a time.
- `src/gradebook/ui/view.rs` (295) mixes refreshing the window (`Refresh`, `apply`, `show_*`, `refresh_*`) with turning domain values into Slint structs (`column_item`, `row_item`, `cell_items`, `to_ui_tone`, `to_ui_type`, `from_ui_type`, `round_sources`, `number`).
After items 27 and 31.
**Goal:** every file involved is under ~200 lines, split by what changes together; all public paths stay the same.

### Behaviour
- No visible change; same texts in every language.

### Out of scope
- Rewording any text, or adding a language.

### Implementation
1. `src/shortcuts/`: move `Action` to `action.rs`, `Key` with `NAMED_KEYS` to `key.rs`, `KeyCombo`, `ShortcutError` and `EDITING_LETTERS` to `combo.rs`, and `ShortcutProblem`, `ProblemKind` and `Shortcuts` to `set.rs`. `mod.rs` keeps the `//!` doc, the `mod` lines and `pub use` of every public item, so `avalio_lite::shortcuts::{Action, Key, KeyCombo, …}` keeps working (`tests/shortcuts.rs`, `src/error.rs`, `src/i18n/`).
2. `src/i18n/`: new `english.rs` and `galician.rs`, each with `pub(super) fn message(message: &Message) -> String` (the body of today's `messages::english` / `galician`) and `pub(super) fn error(error: &AppError) -> String` (the body of `errors::english` / `galician`). `messages.rs` keeps `Message`, `text` (dispatching on `Language`) and the shared helpers `plural`, `counted`, `renamings`, made `pub(super)` where the language files use them. Move `export_labels`, `ExportLabels`, `column_type_name` and `default_column_title` (and item 26's `impl ColumnTitles for Language`) to `labels.rs`. `errors.rs` keeps `error_message`, `entries`, `IoProblem` and `DatabaseProblem`. `i18n/mod.rs` re-exports exactly what it exports today.
3. `src/gradebook/ui/`: move the domain-to-Slint conversions (`column_item`, `row_item`, `cell_items`, `number`, `to_ui_tone`, `to_ui_type`, `from_ui_type`, `round_sources`, `NO_SOURCE`) to `items.rs`. `view.rs` keeps the refresh flow. Update the callers in `columns.rs`, `panel.rs` and `view.rs` (`super::items::…`).
4. A private helper that a new sibling file needs (for example `entries` in `errors.rs`, used by both language files) becomes `pub(super)`, never `pub`. Then `grep -n '<old path>' TODO.md` for every path this item moved and update the items still open, so the next implementer isn't sent to a file that is gone.
5. `metadata -c` on the new files, `metadata -m` on edited ones; `cargo fmt`; `cargo clippy --all-targets -- -D warnings`.

### Tests
- No new behaviour. `tests/i18n.rs` proves every message and error still exists in both languages (it fails on a missing one); `tests/shortcuts.rs` and `tests/ui.rs` pass unchanged.
- The per-language dispatch is already guarded: `tests/i18n.rs` `rust_messages_are_translated_and_keep_their_details` and `errors_are_translated_and_keep_their_details` run every entry of `every_message()` and `every_error()` through both languages and fail on an untranslated or detail-losing text. So no new test is needed; any `Message` or `AppError` variant added during the split goes into those two lists.

### Docs
- `docs/development.md`, *Layout*, `i18n/` line: "interface language: one file per language (`english.rs`, `galician.rs`) for Rust-side texts and error messages". *Translations and schema changes*: "any text built in Rust needs a translation in every `src/i18n/<language>.rs`". Changelog: none.

### Done when
- [ ] `wc -l src/shortcuts/*.rs src/i18n/*.rs src/gradebook/ui/*.rs`: all under ~200.
- [ ] Full suite, clippy and fmt pass; `metadata` run on every new and edited file.

## 35. Split the spreadsheet import, export, backup and grade-calculation files

**Why:** four logic files pass the ~200-line threshold (decision D5), each mixing separate jobs:
- `src/spreadsheet/import.rs` (309 lines) reads the workbook (`read_table`, `cell_text`, `without_float_noise`, `clock`, `iso_seconds`), plans the import (`plan_import`, `match_headers`, `refuse_repeated_names`, `same_value`) and applies it (`apply_import`).
- `src/spreadsheet/export.rs` (252) builds safe tab and file names (`tab_name`, `file_name`, `cut_to_utf16` and their constant tables) beside writing the workbook.
- `src/backup/mod.rs` (271) exports, opens and checks a backup copy (`OpenedBackup`, `open_copy`, `open_checked_copy`, `count_contents`), and merges classes in (`add_classes`, `Adding`, `copy_columns`).
- `src/gradebook/compute.rs` (267) holds number parsing and formatting, the weight rules and row evaluation.
After item 31.
**Goal:** each becomes a folder of files under ~200 lines, split by job; public paths stay the same.

### Behaviour
- No visible change; the same workbooks, backups and grades.

### Out of scope
- Any change to formats or formulas (the golden files must not change).

### Implementation
1. `src/spreadsheet/import.rs` → `src/spreadsheet/import/`: `mod.rs` (the public types `ImportTable`, `ImportRow`, `LeftOut`, `ImportSummary`, `ImportPlan`, and `pub use` of the functions), `read.rs` (`read_table`, `read_first_tab`, `cell_text`, `without_float_noise`, `clock`, `iso_seconds`, `SECONDS_PER_DAY`, `SIGNIFICANT_DIGITS`), `plan.rs` (`plan_import`, `PlannedPupil`, `Target`, `refuse_repeated_names`, `match_headers`, `same_value`), `apply.rs` (`apply_import`). `tests/spreadsheet.rs` lines 25 and 852 must compile unchanged.
2. `src/spreadsheet/export.rs` → `src/spreadsheet/export/`: `mod.rs` (the workbook writing) and `names.rs` (`tab_name`, `file_name`, `cut_to_utf16`, `TAB_NAME_LIMIT`, `TAB_REFUSED`, `FILE_REFUSED`, `DEVICE_NAMES`, `RESERVED_TAB`, `TAKEN_SUFFIX`, `REPLACEMENT`), re-exported.
3. `src/backup/`: `mod.rs` keeps `EXTENSION`, `default_file_name` and `export`; `open.rs` takes `OpenedBackup`, `BackupContents`, `open_copy`, `open_checked_copy`, `open_with` and `count_contents`; `merge.rs` takes `AddSummary`, `add_classes`, `Adding` and `copy_columns`; all re-exported. `ui.rs` from item 31 stays.
4. `src/gradebook/compute.rs` → `src/gradebook/compute/`: `numbers.rs` (`parse_number`, `format_number`, `round_to`, `DISPLAY_DECIMALS`, `FLOAT_SNAP`), `weights.rs` (`FULL_WEIGHT`, `weight_total`, `bonus_weight_total`, `test_weights`, `WEIGHT_TOLERANCE`, `weights_add_up`, `grades_are_provisional`, `depends_on_weights`), `mod.rs` (the row evaluation and `grade_summary`, with `pub use` of the above). `.cargo/mutants.toml`'s `exclude_re` entry `"replace < with <= in weights_add_up"` matches by function name, so it still applies; confirm with `cargo mutants --list --file src/gradebook/compute/weights.rs | grep weights_add_up`.
5. A private helper that a new sibling file needs becomes `pub(super)`, never `pub`. Then `grep -n '<old path>' TODO.md` for every path this item moved and update the items still open, so the next implementer isn't sent to a file that is gone. Item 7, if still open, names `src/gradebook/compute.rs` for mutation runs: it becomes `--file 'src/gradebook/compute/*.rs'`.
6. `metadata -c` on new files, `metadata -m` on moved and edited ones; `cargo fmt`; `cargo clippy --all-targets -- -D warnings`.

### Tests
- No new behaviour. `tests/golden.rs` (saved formats unchanged), `tests/spreadsheet.rs`, `tests/backup.rs` and `tests/compute.rs` (including its property tests) pass unchanged.
- `coverage.ps1 -Branches` passes.

### Docs
- `docs/development.md`, *Layout*: `spreadsheet/` line: "Excel export (writing, safe names) and import (read, plan, apply) of a class sheet"; `backup/` line: ".avalio backup files: export, open and check, add classes". Changelog: none.

### Done when
- [ ] `wc -l` on every file under `src/spreadsheet/`, `src/backup/` and `src/gradebook/compute/`: all under ~200.
- [ ] `git status tests/golden` is clean after the full suite; clippy and fmt pass; `metadata` run.

## 28. Split ui/components.slint into one file per component family

**Why:** `ui/components.slint` is 589 lines holding 20 unrelated widgets: buttons, text fields, the menu, navigation items, icons, banners, cards, key caps, the toggle. Nearly every UI change touches it, and each page pulls in a file mostly made of widgets it doesn't use (`ui/sheet-page.slint` imports 10 names from it, `ui/column-header.slint` 3). One file per family keeps a menu change out of the buttons' file and lets each page import only what it uses.
**Goal:** `ui/components/` holds the widgets grouped by family; `ui/components.slint` is gone; every screen is pixel-identical.

### Behaviour
- No visible change: every desktop baseline still matches without re-recording.

### Out of scope
- Changing any widget's look or properties.
- Splitting the page files (`sheet-page.slint` 423 lines, `settings-page.slint` 323): only if they still feel too big after this, as a new item.

### Implementation
1. Create `ui/components/` with these files, moving each `export` block from `ui/components.slint` unchanged, with its comments:
   - `focus.slint`: `KeyboardPress`, `FocusRing`
   - `buttons.slint`: `ButtonKind`, `ActionButton`, `IconButton`, `IconActionButton`, `Toggle`
   - `fields.slint`: `TextField`, `FolderField`
   - `menu.slint`: `MenuSurface`, `MenuOption`, `MenuEntry`, `Menu`
   - `navigation.slint`: `NavItem`, `IconNavItem`
   - `icons.slint`: `IconKind`, `Icon`, `BrandMark`
   - `surfaces.slint`: `MessageBanner`, `Card`, `Tag`, `KeyCap`
2. Give each new file only the imports its components use: `Theme` from `"../theme.slint"`, plus the sibling files it needs (`buttons.slint` and `navigation.slint` import `Icon`/`IconKind` from `"icons.slint"`; whichever use `FocusRing` or `KeyboardPress` import `"focus.slint"`). Grep each component name in each new file to find them; `cargo build` reports anything missing.
3. `git rm ui/components.slint`.
4. In `ui/column-header.slint`, `ui/column-panel.slint`, `ui/lock-page.slint`, `ui/settings-page.slint`, `ui/setup-page.slint`, `ui/sheet-page.slint` and `ui/shell.slint`, replace the `from "components.slint"` import with one import per family file used (`from "components/buttons.slint"` etc.), listing only the names already imported.
5. `build.rs` compiles `ui/app-window.slint` and Slint follows the imports, so it needs no change; confirm with `cargo build`.
6. `tests/i18n.rs` `ui_strings()` (line 28) lists `ui/` with one flat `fs::read_dir` and calls `read_to_string` on every entry. A subfolder makes it fail, and once that is fixed naively, every `@tr` inside `ui/components/` would go unchecked. Split it into `ui_strings()`, which calls `ui_strings_in(&manifest_path("ui"))`, and `ui_strings_in(root: &Path)`, which walks subfolders recursively and reads only `.slint` files.
7. Then `grep -n '<old path>' TODO.md` for every path this item moved and update the items still open, so the next implementer isn't sent to a file that is gone.

### Tests
- `tests/ui.rs` passes unchanged: it finds controls by accessible label (`ElementHandle::find_by_accessible_label`), and the labels move with their components.
- `cargo test --test desktop -- --ignored --test-threads=1`: every snapshot matches the existing baselines in light, dark and Galician, at both window sizes, without `AVALIO_UPDATE_BASELINES`. This is the proof the move changed nothing.
- `tests/i18n.rs` `every_ui_string_is_translated_and_no_stale_entries_remain` passes with the recursive walk, and the `.po` files need no edit. Before the walk is recursive it reports the `ui/components/` strings as stale, which proves the walk matters.
- `tests/i18n.rs`, new `ui_strings_include_nested_folders`: a temporary folder with `a.slint` (`@tr("Top")`), `sub/b.slint` (`@tr("Nested")`) and `sub/notes.txt` (`@tr("Ignored")`) gives exactly {"Top", "Nested"} from `ui_strings_in`.

### Docs
- `docs/development.md`, *Layout*, `ui/` line: add "`components/` the shared widgets, one file per family (buttons, fields, menu, navigation, icons, surfaces, focus)". Changelog: none (no visible change).

### Done when
- [ ] `ui/components.slint` no longer exists and `grep -rn '"components.slint"' ui` finds nothing.
- [ ] No file in `ui/components/` is over ~200 lines.
- [ ] The desktop suite passes against unchanged baselines (`git status tests/desktop/baselines` clean); `cargo test` and clippy pass.

## 36. Split the large Slint pages into their parts

**Why:** after item 28, four page files still pass the ~200-line threshold (decision D5), each holding several independent parts: `ui/sheet-page.slint` (423 lines: `NameCell`, `Summary`, `SheetTitle`, the sheet grid, three popup menus `column-menu`, `more-menu` and `pupil-menu`, and `NoClassPage`); `ui/settings-page.slint` (323: seven cards, Appearance, Language, Keyboard shortcuts, Database, PIN, Backup and Logs, plus the helpers `PathBox`, `PinField` and `PinCard`); `ui/lock-page.slint` (271: the keypad `Key` and `LetterKey`, plus the page); `ui/column-panel.slint` (227: `FieldLabel` and `NumberField`, plus the panel). After item 28 (its `ui/components/` imports).
**Goal:** one folder per page, each file under ~200 lines; every screen pixel-identical.

### Behaviour
- No visible change: every desktop baseline matches without re-recording.

### Out of scope
- Any change to look, texts or element ids.

### Implementation
1. `ui/sheet/`: move `sheet-page.slint` there; move `NameCell` → `name-cell.slint`, `Summary` and `SheetTitle` → `sheet-header.slint`, `NoClassPage` → `no-class-page.slint`. Move each popup's contents (the `MenuSurface` subtree) into a component in `menus.slint` (`ColumnMenuContent`, `MoreMenuContent`, `PupilMenuContent`), passing the data and callbacks it uses as `in` properties and `callback`s. The `PopupWindow` elements stay in `sheet-page.slint`, because their `show()` and position belong to the page.
2. `ui/settings/`: move `settings-page.slint` there; one file per card, as a component taking the properties and callbacks it uses (`appearance-card.slint`, `language-card.slint`, `shortcuts-card.slint`, `database-card.slint` with `PathBox`, `pin-card.slint` with `PinField` and `PinCard`, `backup-card.slint`, `logs-card.slint`). The page keeps the layout and the bindings to `AppWindow`.
3. `ui/lock/`: `lock-page.slint` plus `keypad.slint` (`Key`, `LetterKey`).
4. `ui/column-panel/`: `column-panel.slint` plus `fields.slint` (`FieldLabel`, `NumberField`).
5. Update the imports in `ui/app-window.slint`, `ui/shell.slint` and wherever these pages are used (`grep -rn 'sheet-page\|settings-page\|lock-page\|column-panel' ui`); relative paths gain `../` for `theme.slint`, `gradebook.slint`, `shortcuts.slint`, `backup.slint` and `components/`.
6. `tests/ui.rs` finds controls by accessible label (`ElementHandle::find_by_accessible_label`), so every `accessible-label` must move unchanged with its element; check with `grep -c accessible-label` on the old file before the split against the sum over the new files. Then `grep -n '<old path>' TODO.md` for every path this item moved and update the items still open, so the next implementer isn't sent to a file that is gone.
7. `cargo build`; `cargo clippy --all-targets -- -D warnings`.

### Tests
- `tests/ui.rs` passes unchanged (accessible labels kept).
- Desktop suite: every snapshot matches the existing baselines in light, dark and Galician at both sizes, without `AVALIO_UPDATE_BASELINES`.
- `tests/i18n.rs` passes: no `@tr` string changed, so the `.po` files are untouched.

### Docs
- `docs/development.md`, *Layout*, `ui/` line: "one folder per page (`sheet/`, `settings/`, `lock/`, `column-panel/`) holding the page and its parts". `docs/design-decisions.md`: D5 status → Done if items 34, 35 and 28 are done. Changelog: none.

### Done when
- [ ] `wc -l ui/*.slint ui/*/*.slint`: all under ~200.
- [ ] Desktop suite passes against unchanged baselines (`git status tests/desktop/baselines` clean); `cargo test` passes.

## 29. Guard the module dependency rules with a test

**Why:** items 25–27 and 31–32 fix the places where the code's layers pointed the wrong way: the core state inside the UI layer, the gradebook → i18n cycle, Slint inside the shortcut rules, feature UIs reaching into `app`. Nothing stops these coming back, since each looked fine file by file. Decision D4 in `docs/design-decisions.md` says a test enforces the rules. It is the last item in the structure block (after 25, 26, 27, 31 and 32), and it fails before them.
**Goal:** `cargo test` fails, naming the file, the line and the rule, when a module breaks D4.

### Behaviour
- A file is **UI** if it is under `src/app/` or `src/window/`, is `src/lib.rs`, or has a `ui` path component (`src/<feature>/ui.rs` or `src/<feature>/ui/…`). Every other file is **logic**. The rules, each failure message printing its reason:
  - Logic files have no `slint::` path or `use slint` and no `crate::ui`, and don't use `crate::app`, `crate::window` or any feature's `ui` module. The UI framework stays at the edge; logic never depends on wiring.
  - Nothing outside `src/app/` and `src/main.rs` uses `crate::app`. The composition root sits on top.
  - Files under `src/gradebook/` (except `ui/`), `src/platform/db/`, `src/platform/storage.rs` and `src/lock/mod.rs` don't use `crate::i18n`. Core logic doesn't know the interface language; it uses ports such as `ColumnTitles`.
  - The module graph has no cycle. Its nodes are the top-level modules, with each feature's `ui` as a node of its own (`gradebook` and `gradebook::ui` are separate), and `platform::<name>` as one node per platform module. Edges come from each file's `crate::…` uses.
- A failure lists every offending file and line, not only the first.

### Out of scope
- External tools (cargo-modules, cargo-deny): a plain test needs nothing installed and runs in the default suite.
- Rules for the `ui/*.slint` files.

### Implementation
1. New `tests/architecture.rs` (`metadata -c`). Walk `src/` recursively with `std::fs::read_dir` from `env!("CARGO_MANIFEST_DIR")`. For each `.rs` file, record its node (`src/state/startup.rs` → `state`, `src/gradebook/ui/view.rs` → `gradebook::ui`, `src/platform/db/mod.rs` → `platform::db`), whether it is UI, and every `crate::<path>` and `super::<path>` on each code line, resolved to a node the same way (`super::` relative to the file's own module, so `src/gradebook/x.rs` using `super::ui::…` counts as an edge to `gradebook::ui`). Skip lines whose trimmed text starts with `//`: comments may name things they don't use (`src/i18n/mod.rs`'s doc comment says "Slint strings"). The codebase writes one `use crate::<module>…` per line. If a grouped `use crate::{a, b}` appears, the test fails with "write one `use crate::…` per module so tests/architecture.rs can read it", rather than silently missing edges.
2. One `#[test]` per rule, each collecting every violation as `path:line: text` and asserting the list is empty, with the rule's reason in the message.
3. A pure helper `find_cycle(edges: &BTreeMap<String, BTreeSet<String>>) -> Option<Vec<String>>` (depth-first search) used by the cycle test, which fails with the cycle written as `a → b → a`.
4. `cargo fmt`; `cargo clippy --all-targets -- -D warnings`.

### Tests
- `tests/architecture.rs`: `logic_does_not_depend_on_the_ui`, `only_main_uses_the_composition_root`, `core_logic_does_not_use_i18n` and `modules_have_no_cycle`, passing on the tree after items 25–27 and 31–32.
- `tests/architecture.rs`: `find_cycle_reports_a_planted_cycle` (`a → b → c → a` gives `Some(["a", "b", "c", "a"])`) and `find_cycle_accepts_a_dag` (`a → b`, `a → c`, `b → c` gives `None`). Proves the detector can fail, so a pass means something.
- Error path, checked once by hand and noted in the commit body: add `use crate::i18n::Language;` to `src/gradebook/cells.rs`, see `core_logic_does_not_use_i18n` fail naming that file and line, then remove it. Do the same with `use crate::window::SharedState;` in `src/backup/mod.rs` for `logic_does_not_depend_on_the_ui`.

### Docs
- `docs/design-decisions.md`, D4: list the four rules exactly as the test checks them; status → Done.
- `docs/development.md`, *Tests*, default-suite bullet: add "the dependency rules (`tests/architecture.rs`, see D4 in the design decisions)".
- `CONTRIBUTING.md`, *Submitting Changes*: one line, "Keep to the dependency rules in [docs/design-decisions.md](docs/design-decisions.md) (D4); `cargo test` checks them."
- Changelog: none (tests only).

### Done when
- [ ] `cargo test --test architecture` passes, and both planted violations above failed with the right file and line.
- [ ] D4 in `docs/design-decisions.md` matches what the test checks.
- [ ] Full suite, clippy and fmt pass; `metadata` run on every new and edited file.
