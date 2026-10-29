[//]: # ( ---------------------------------------------------------------------- )
[//]: # (+ Authors: 	Ran# <ran.hash@proton.me> )
[//]: # (+ Created: 	2026/10/28 10:26:46.388247 )
[//]: # (+ Revised: 	2026/10/29 04:09:17.326373 )
[//]: # ( ---------------------------------------------------------------------- )

# TODO

The backlog. Each item is self-contained: reading it alone is enough to
implement it fully, without asking anything. Work top to bottom unless an item
says otherwise.

Next item number: 17

## Index

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
16. [Get the user's sign-off on the logo, and redraw it if they ask](#16-get-the-users-sign-off-on-the-logo-and-redraw-it-if-they-ask)

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
