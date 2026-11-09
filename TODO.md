[//]: # ( ---------------------------------------------------------------------- )
[//]: # (+ Authors: 	Ran# <ran.hash@proton.me> )
[//]: # (+ Created: 	2026/10/28 10:26:46.388247 )
[//]: # (+ Revised: 	2026/11/09 13:39:17.921272 )
[//]: # ( ---------------------------------------------------------------------- )

# TODO

The backlog. Each item is self-contained: reading it alone is enough to
implement it fully, without asking anything. Work top to bottom unless an item
says otherwise.

Next item number: 30

## Index

27. [Let the user drag column edges to set each column's width](#27-let-the-user-drag-column-edges-to-set-each-columns-width)
28. [Keep the cursor in view while typing in a field that is too narrow](#28-keep-the-cursor-in-view-while-typing-in-a-field-that-is-too-narrow)
29. [Make Enter finish an edit visibly, and move down a row in mark cells](#29-make-enter-finish-an-edit-visibly-and-move-down-a-row-in-mark-cells)

## Adding an item

- Write for someone who has never seen the conversation: what happens today,
  what should happen instead, and why each non-obvious decision was made.
- Be concrete: real file paths, names, sizes, colours, keys. No open "maybe";
  if the implementer must choose, say how to choose and how to check.
- Use the template below. Implementation is a numbered list of steps in the
  order to do them; Done when is a checklist.
- Number it with "Next item number" above, then increase that number.
- File order is priority: the first item is the next one to do.
- Numbers ascend in file order. Deleting leaves a gap; when adding at the top
  or moving an item breaks the order, renumber every item in file order (from
  the lowest number in use) and update every reference to them.
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

## 27. Let the user drag column edges to set each column's width

**Why:** every sheet column has a fixed width set in `ui/sheet/sheet-grid.slint`: the pupil column `name-width` 210px, test and calculated columns `cell-width` 128px, text (info) columns `wide-cell-width` 176px. A long pupil name or a long note in a text column is cut off at the right edge and can't be read: there is no resize, no tooltip, and the field doesn't scroll to the cursor (that last part is item 28). The user reported on 2026-11-09 that a very long pupil name is "unreadable entirely".
**Goal:** the user drags the right edge of any column header, the "Pupil" header included, to make that column wider or narrower. The sheet keeps that width for that column of that class, after a restart and in backups too.

### Behaviour
- Hovering the right 8px of a column header (or of the "Pupil" header) shows the left-right resize cursor (`MouseCursor.col-resize`) and a 2px `Theme.accent` line along that edge, the full height of the header.
- Pressing there and dragging sideways resizes the column live: the header and every cell of that column follow the pointer, and the columns to its right move with it. The width is clamped: test, calculated and text columns 72px to 640px, the pupil column 120px to 640px. 72px still fits a "10.00" pill; 120px still fits the "×" button and a few letters of the name.
- Releasing the mouse saves the width. Only the release writes to the database, so a drag is one write, not one per pixel.
- Double-clicking the edge puts the column back to its default width (210 / 128 / 176px) and saves that.
- Clicking the edge zone doesn't open the column panel; clicking anywhere else on a header still does, as today.
- With keyboard focus on a column header (Tab reaches it, as today), Shift+→ widens it by 16px and Shift+← narrows it by 16px, within the same limits, saved on each press. No default shortcut uses Shift+arrow (`src/shortcuts/action.rs`), and a header has no text to select, so nothing is lost.
- A new column starts at its kind's default width. A column never resized has no stored width and follows the default, so a later change of the default still reaches it.
- **Duplicate class** copies each column's width and the pupil column's width to the copy.
- While the column panel is open, resizing the column it shows keeps that column in view (the existing `reveal` in `sheet-grid.slint` runs again when the width changes).
- A screen reader reads the edge as a "Column width" slider with its value in pixels; the header button keeps its label.

### Out of scope
- Fitting a column to its content automatically.
- Text wrapping in cells, and a tooltip for clipped text.
- Row heights.

### Implementation
1. `migrations/005_column_widths.sql` (new): `ALTER TABLE sheet_columns ADD COLUMN width INTEGER CHECK (width IS NULL OR width BETWEEN 72 AND 640);` and `ALTER TABLE classes ADD COLUMN name_width INTEGER CHECK (name_width IS NULL OR name_width BETWEEN 120 AND 640);`, with a comment that NULL means "the default for the kind". Each is a fact about one column or one class, so it sits in that row and the schema stays BCNF. Never edit migrations 001–004.
2. `src/platform/db/migrations.rs`: add the new file to `MIGRATIONS`.
3. `src/gradebook/model.rs`: add `pub width: Option<u16>` to `Column`, and the limits as constants next to it (`MIN_COLUMN_WIDTH = 72`, `MIN_NAME_WIDTH = 120`, `MAX_COLUMN_WIDTH = 640`), so Rust names each number in one place.
4. `src/gradebook/columns.rs`: select `width` in `list_columns` (as column 8) and read it in `column_from_row`; add `pub fn set_column_width(connection, column_id, width: Option<u16>) -> Result<()>` that clamps to the limits and runs `UPDATE sheet_columns SET width = ?2 WHERE id = ?1` through the existing `update_one`; log at `info!` with the column id and width.
5. `src/gradebook/classes.rs`: add `class_name_width(connection, class_id) -> Result<Option<u16>>` and `set_name_width(connection, class_id, width: Option<u16>) -> Result<()>` the same way. In `duplicate_class`, after `insert_column`, call `set_column_width` with the source column's width, and copy the class's `name_width` once. Grep every other `Column {` construction (`rg "Column \{" src tests`) and give it `width`.
6. `ui/gradebook.slint`: add `width: int` (0 = default) to `ColumnItem`; add `in property <int> name-width;` (0 = default), `callback set-column-width(int, int);` (column index, px, 0 = default) and `callback set-name-width(int);`.
7. `src/gradebook/ui/items.rs`: fill `width` in `column_item` (`column.width.map_or(0, i32::from)`). `src/gradebook/ui/view.rs`: set `name-width` whenever a class's sheet loads.
8. `src/gradebook/ui/columns.rs`: wire `on_set_column_width` and `on_set_name_width` through `run(...)` like the other column settings, returning a refresh that updates only the column items (not the cells), so no row is rebuilt after a resize. An error reaches the user through `run`'s existing error banner; never swallow it.
9. `ui/components/resize-edge.slint` (new): a `ResizeEdge` component: an 8px `TouchArea` at the right edge with `mouse-cursor: col-resize`, the 2px accent line while hovered or pressed, `in-out property <length> width-value`, `in property <length> min-width`, `max-width`, `default-width`, `callback committed(length)`. On `moved` it sets `width-value` from the width at press plus `mouse-x - pressed-x`, clamped; on release it calls `committed`; `double-clicked` sets `default-width` and commits. `accessible-role: slider`, `accessible-label: @tr("Column width")`, `accessible-value` the px number. One component for both header kinds, so the drag code isn't repeated.
10. `ui/sheet/sheet-grid.slint`: replace the three fixed widths with a function `column-width(c)` that returns `Gradebook.columns[c].width * 1px` when above 0, else the kind's default (keep `name-width`, `cell-width`, `wide-cell-width` as the defaults); use it for each `ColumnHeader` and each `GridCell`, plus a live drag width per column so the cells follow during a drag. Put a `ResizeEdge` on the "Pupil" header. Rerun `reveal` on `changed width` of the header shown in the panel.
11. `ui/column-header.slint`: add the `ResizeEdge` on top of the header's `TouchArea`, so the edge isn't covered by the click that opens the panel; in its `KeyboardPress` (widen it to a `FocusScope` handler if needed), accept Shift+`Key.RightArrow` / Shift+`Key.LeftArrow` to call `Gradebook.set-column-width` with ±16px, clamped.
12. `lang/gl/LC_MESSAGES/avalio-lite.po`: add "Column width" → "Largura da columna". Every locale gets every new string.
13. `metadata -m` on each edited file and `metadata -c` on the two new ones; `cargo fmt`, then `cargo clippy --all-targets -- -D warnings`, fixed by hand.

### Tests
- `tests/db.rs`: a new database has `sheet_columns.width` and `classes.name_width`; storing 50 or 700 is refused by the CHECK; NULL is allowed.
- `tests/golden.rs`: add `tests/golden/db-v4.db` (today's `db-current.db`, copied before the migration lands) and `a_version_4_database_upgrades_with_all_its_data`; regenerate `schema.sql` and `db-current.db` with `AVALIO_UPDATE_GOLDEN=1` and check their diffs show only the two new columns.
- `tests/gradebook.rs`: `set_column_width` stores and reads back 200; clamps 10 → 72 and 900 → 640; `None` resets to the default; an unknown column id returns the existing not-found error. The same for `set_name_width`.
- `tests/duplicate.rs`: every `DuplicateContents` variant keeps the column widths and the pupil column width.
- `tests/ui.rs`: `dragging_a_column_edge_resizes_and_saves_it` (drag the first test column's edge +80px, check `Gradebook.columns[0].width` is 208 and the database holds 208); `double_clicking_the_edge_restores_the_default`; `shift_arrow_on_a_focused_header_changes_its_width`; `the_pupil_column_width_is_kept_per_class` (resize in one class, switch to another and back); `a_click_on_the_edge_does_not_open_the_column_panel`; `a_failed_width_save_shows_the_error`.
- `tests/desktop.rs`: in `the_sheet`, give the sample a long pupil name, drag the "Pupil" edge to 400px with real posted mouse messages, and add a reviewed baseline per theme, size and scale; the full name must show in the image.
- `tests/architecture.rs` still passes (the new component sits in `ui/components/`).

### Docs
- `docs/user-guide.md`, "### Columns": add "Drag the right edge of a column's header to change its width (double-click the edge to reset it, or Shift+← / Shift+→ on a focused header); each class keeps its widths."
- `docs/user-guide.md`, "### Keyboard": add Shift+← / Shift+→ on a focused column header.
- Changelog: "[A] drag a column's edge to set its width; each class keeps its widths".

### Done when
- [ ] Dragging, double-clicking and Shift+arrow resize each column kind and the pupil column within the limits, seen in the running app in light and dark.
- [ ] The widths survive a restart, a class switch and **Duplicate class**.
- [ ] A version 4 database opens and upgrades with its data.
- [ ] Galician has the new string.
- [ ] `cargo test` and the desktop suite pass; the new baselines are reviewed.

## 28. Keep the cursor in view while typing in a field that is too narrow

**Why:** the in-place fields clip their text. `TextField` in `ui/components/fields.slint` sets `clip: true` and puts its `TextInput` at a fixed `x`, and `GridCell` in `ui/grid-cell.slint` does the same with a bare `TextInput`. Slint's plain `TextInput` doesn't scroll to follow the cursor (std `LineEdit` adds that itself), so in a pupil name, class name or text cell longer than its box, End, → or typing at the end take the cursor past the right edge, out of sight. The user reported on 2026-11-09 that they "can't move on the field" to read a long pupil name. Item 27 lets a column grow; this item makes a field usable at any width.
**Goal:** while a field has focus, its text slides sideways so the cursor is always visible, as in any Windows text box; when it loses focus it shows its start again.

### Behaviour
- Typing at the end of a long text, End, →, ← and Home keep the cursor inside the field, 8px from its edges.
- Selecting with Shift+arrows or the mouse scrolls the same way.
- A text that fits never moves.
- Losing focus puts the text back to its start, so the resting sheet shows the beginnings of names.
- Right-aligned number cells (`numeric: true`) behave the same, mirrored.
- Calculated cells aren't text fields and don't change.

### Out of scope
- Column widths (item 27) and Enter (item 29).
- A tooltip with the full text.

### Implementation
1. `ui/components/fields.slint`: add a `ScrollingTextInput` component: a clipping `Rectangle` holding a `TextInput` that is `max(parent.width, self.preferred-width)` wide, placed at a private `scroll` offset. In `cursor-position-changed(position)`, shift `scroll` so `position.x + scroll` stays between 8px and `parent.width - 8px`; reset `scroll` to 0 when focus is lost. Forward `text`, `edited`, `accepted`, `has-focus`, alignment, font and colour properties. This is the pattern std `LineEdit` uses.
2. `ui/components/fields.slint`: use `ScrollingTextInput` inside `TextField`, keeping `restore()`, `clear()` and `focus-input()` working.
3. `ui/grid-cell.slint`: use `ScrollingTextInput` instead of the bare `TextInput`, so the logic lives in one place.
4. `metadata -m` on the edited files. No new strings, so no locale change.

### Tests
- `tests/ui.rs`: `a_long_pupil_name_scrolls_to_the_cursor` (rename a pupil to 80 characters, press End, check through the testing backend's element geometry that the cursor's x is inside the field); `leaving_the_field_shows_its_start_again`; `a_short_name_does_not_move`; `a_long_note_in_a_text_cell_scrolls`; `a_right_aligned_mark_cell_scrolls_mirrored`.
- `tests/desktop.rs`: in `the_sheet`, type a long note into a text cell, press End, and add a reviewed baseline showing the note's end, light and dark.

### Docs
- None in the guide (it is what a text box is expected to do). Changelog: "[F] long text in a field scrolls to keep the cursor in view".

### Done when
- [ ] In the running app, End in an 80-character pupil name shows its end, and Tab away shows its start.
- [ ] Mark cells and the column panel fields behave the same.
- [ ] `cargo test` and the desktop suite pass; the new baselines are reviewed.

## 29. Make Enter finish an edit visibly, and move down a row in mark cells

**Why:** every field in the sheet saves as you type (`edited` calls `Gradebook.rename-pupil`, `Gradebook.cell-edited` and so on), so pressing Enter changes nothing on screen. In a mark cell (`GridCell`, `ui/grid-cell.slint`) the `TextInput` has no `accepted` handler; in the pupil name, class name and column panel fields (`TextField` with `restores: true`), Enter only puts back a refused value. The blue edit border stays, so it looks as if Enter didn't take. The user asked on 2026-11-09 for Enter to "actually look like it does the edit", and chose that in mark cells Enter moves to the cell below, as in a spreadsheet, so a column of marks can be typed in one go.
**Goal:** Enter in a mark or text cell leaves it showing the saved value and puts the cursor in the same column one row down. Enter in the pupil name, class name or a column setting ends the editing: the blue border goes and the field looks like the rest of the page.

### Behaviour
- Typed cell (test or info column), not on the last row: Enter moves focus to the same column's cell in the next row, with its text all selected, so typing a mark replaces the old one. The cell left shows its saved value with its resting look (red shading if out of range, as today).
- Typed cell on the last pupil: Enter clears focus, and the cell shows its resting look.
- The move only goes down within the same typed column, so it never lands on a calculated cell.
- `TextField` with `restores: true` (pupil name in `ui/sheet/name-cell.slint`, class name in `ui/sheet/sheet-header.slint`, column title in `ui/column-panel/column-panel.slint`, number fields in `ui/column-panel/fields.slint`): Enter first puts back a refused value as today, then clears focus.
- `TextField` without `restores` (Add pupil…, new class in the sidebar and on the no-class page, PIN, licence key, setup folders) keeps today's Enter, because those fields must stay focused to add the next item or to submit.
- Shift+Enter and Esc are unchanged.

### Out of scope
- Moving up with Shift+Enter, or sideways with Tab inside the grid.
- The scrolling of item 28.

### Implementation
1. `ui/components/fields.slint`: in `input`'s `accepted`, after `root.restore()` and `root.accepted(...)`, call `self.clear-focus()` when `root.restores` is true. Keying it on `restores` keeps the add-fields focused.
2. `ui/grid-cell.slint`: add `callback next();`, `public function focus-and-select()` (`input.focus(); input.select-all();`) and `public function leave()` (`input.clear-focus();`); in the `TextInput`, add `accepted => { root.next(); }`.
3. `ui/sheet/sheet-grid.slint`: add `property <int> focus-row: -1;` and `property <int> focus-column: -1;`. On each `GridCell`, add `property <bool> requested: root.focus-row == r && root.focus-column == c;` with `changed requested => { if (self.requested) { self.focus-and-select(); root.focus-row = -1; } }`, and `next => { if (r + 1 < Gradebook.rows.length) { root.focus-row = r + 1; root.focus-column = c; } else { self.leave(); } }`. Resetting to -1 lets the same cell be asked for again later.
4. Check in the running app that `Refresh::Row(row)` from `src/gradebook/ui/cells.rs` (it rebuilds the edited row's item after each keystroke) doesn't take focus from the row below; if it does, update only that cell's `CellItem` in the row model instead of the whole row, and grep the other `Refresh::Row` users so they keep working.
5. `metadata -m` on the edited files. No new strings, so no locale change.

### Tests
- `tests/ui.rs`: `enter_in_a_mark_cell_moves_to_the_cell_below` (type "7" in row 0 of the first test column, press Enter, check the cell below has focus and the database holds 7); `enter_selects_the_cell_below_so_typing_replaces_it`; `enter_on_the_last_pupil_leaves_the_sheet_unfocused`; `enter_in_an_out_of_range_cell_still_moves_down_and_keeps_it_red`; `enter_in_a_text_column_moves_down_too`; `enter_in_a_pupil_name_ends_the_editing` (focus gone, name saved); `enter_with_a_refused_class_name_puts_it_back_and_ends_the_editing`; `enter_in_add_pupil_keeps_the_field_focused` (regression guard).
- `tests/desktop.rs`: in `the_sheet`, type a mark, press Enter, and add a reviewed baseline showing the focus border on the cell below, light and dark.

### Docs
- `docs/user-guide.md`, "### Marks": add "Press Enter to go to the same column in the next pupil's row."; "### Keyboard": add Enter in a mark cell and Enter in a name or setting.
- Changelog: "[A] Enter in a mark moves to the next pupil; Enter in a name or setting ends the editing".

### Done when
- [ ] In the running app, Enter walks down a test column while typing marks, and leaves the sheet after the last pupil.
- [ ] Enter in the pupil name, class name and column fields removes the blue border; Add pupil and new class stay focused.
- [ ] `cargo test` and the desktop suite pass; the new baselines are reviewed.
