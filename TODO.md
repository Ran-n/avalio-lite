[//]: # ( ---------------------------------------------------------------------- )
[//]: # (+ Authors: 	Ran# <ran.hash@proton.me> )
[//]: # (+ Created: 	2026/10/28 10:26:46.388247 )
[//]: # (+ Revised: 	2026/11/12 08:46:25.202460 )
[//]: # ( ---------------------------------------------------------------------- )

# TODO

The backlog. Each item is self-contained: reading it alone is enough to
implement it fully, without asking anything. Work top to bottom unless an item
says otherwise.

Next item number: 42

## Index

41. [Split "Surname, First name" pupils into their two columns on update](#41-split-surname-first-name-pupils-into-their-two-columns-on-update)

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

## 41. Split "Surname, First name" pupils into their two columns on update

**Why:** migration `migrations/006_pupil_surnames.sql` (unreleased; 0.7.0 ships schema 5) renames
`pupils.name` to `first_name` and adds an empty `surname`, so every existing pupil keeps their whole
name as the first name and the teacher has to retype it (`docs/user-guide.md`, "Pupils": "fix them
by hand"). That was chosen because guessing where "Xoán Carlos Pérez" splits would get some names
wrong. But many teachers typed their pupils the way school lists write them, "Pérez López, Ana"
(surnames, a comma, the first name, with or without a space after the comma), and that form is not
a guess: the comma marks the split. After updating, those pupils show "Pérez López, Ana" under
First name and nothing under Surname, and the teacher must fix every one of them by hand.

**Goal:** after updating, a pupil stored as "Pérez López, Ana" shows **Ana** under First name and
**Pérez López** under Surname in every class they are in, without the teacher doing anything. Names
without a comma stay exactly as they are.

**Why a migration, not a script or app code:** the users are teachers running a Windows exe; they
can't be asked to run a script, and a script would miss the old backups they restore later. A new
SQL migration *is* the one-off script, run by the app itself: exactly once per database
(`user_version` records it), in its own transaction, never again. No Rust code path is added and
nothing runs at a normal start. Backups opened through `src/backup/open.rs` (restore, and "Add
classes from a backup…") go through the same migrations, so an old backup gets its names split too.
It is a new migration 008 rather than an edit to 006, because development databases (and
`tests/golden/db-current.db`) are already at version 7 and would never re-run 006.

### Behaviour
- `first_name` "Pérez López, Ana", `surname` "" → first name "Ana", surname "Pérez López".
- "Pérez López,Ana" (no space after the comma) → the same result.
- Any number of words on either side: "Pérez, Ana María" → "Ana María" / "Pérez";
  "de la Fuente García, Xoán Carlos" → "Xoán Carlos" / "de la Fuente García".
- Spaces, tabs, line breaks and no-break spaces (U+00A0) around each part are trimmed, as
  `clean_name` / `clean_surname` in `src/gradebook/model.rs` trim them.
- A pupil whose surname is already filled in is left alone (someone on a dev build already fixed
  it, or the comma really belongs to the first name).
- No comma ("Xoán Carlos Pérez") → unchanged.
- More than one comma ("Pérez, Ana, B") → unchanged: the split is no longer unambiguous.
- Nothing on one side of the comma once trimmed (", Ana", "Pérez,", ",") → unchanged: the first
  name must never be empty, and a lone part is not a surname/first-name pair.
- A pupil in several classes is one row in `pupils`, so the fix shows in all their classes at once.
- Two pupils whose full names become equal after the split ("Ana Pérez" and "Pérez, Ana") both
  stay; `pupils` has no uniqueness on names, and joining people is not this item's job.
- Marks, class membership, column order and widths are untouched; only `pupils.first_name` and
  `pupils.surname` change.

### Out of scope
- Excel import with one whole-name column (`src/spreadsheet/import/read.rs`) keeps putting the
  whole cell in the first name. Applying the comma rule there would be permanent app code; make it
  its own item if wanted.
- Names without a comma: no guessing, as decided in 006.
- No UI, message or setting: the change is silent apart from the log line the migration runner
  already writes ("applied schema migration 8").

### Implementation
1. `migrations/008_split_comma_names.sql` (new): a header comment saying why (the comma form is
   unambiguous, unlike the guess 006 refused; only pupils with an empty surname and exactly one
   comma with text on both sides), then one statement:
   ```sql
   UPDATE pupils
   SET surname    = trim(substr(first_name, 1, instr(first_name, ',') - 1), ' ' || char(9, 10, 13, 160)),
       first_name = trim(substr(first_name, instr(first_name, ',') + 1), ' ' || char(9, 10, 13, 160))
   WHERE surname = ''
     AND instr(first_name, ',') > 0
     AND instr(substr(first_name, instr(first_name, ',') + 1), ',') = 0
     AND trim(substr(first_name, 1, instr(first_name, ',') - 1), ' ' || char(9, 10, 13, 160)) <> ''
     AND trim(substr(first_name, instr(first_name, ',') + 1), ' ' || char(9, 10, 13, 160)) <> '';
   ```
   SQLite evaluates every right-hand side from the old row, so `surname` reads the unsplit
   `first_name`. No schema change, so `tests/golden/schema.sql` stays as it is.
2. `src/platform/db/migrations.rs`: add `include_str!("../../../migrations/008_split_comma_names.sql")`
   to `MIGRATIONS` after 007; `latest_version()` becomes 8 by itself.
3. Leave `migrations/006_pupil_surnames.sql` unedited (dev databases already ran it); its comment
   stays true for names without a comma.
4. Golden samples, as `docs/development.md` → "Translations and schema changes" says: copy
   `tests/golden/db-current.db` to `tests/golden/db-v7.db` *before* regenerating, insert a pupil
   "Pérez López, Ana" with an empty surname into `db-v7.db` (with `sqlite3`) so its upgrade test has
   something to split, then regenerate the current sample with the suite's update switch
   (`updating()` in `tests/golden.rs`).
5. Run `metadata -c` on the new files and `metadata -m` on every edited one.

### Tests
- `tests/db.rs`, `migration_8_splits_names_written_surname_first`: migrate to 7, insert one pupil
  per Behaviour bullet ("Pérez López, Ana", "Pérez López,Ana", "Pérez, Ana María",
  " de la Fuente García ,\u{a0}Xoán Carlos ", "Xoán Carlos Pérez", "Pérez, Ana, B", ", Ana",
  "Pérez,", ",", and "Pérez, Ana" with surname "Gómez" already set), migrate to 8, and assert each
  `(first_name, surname)` pair exactly. Proves the split, the trimming and every left-alone case.
- `tests/db.rs`, `migration_8_leaves_marks_and_classes_alone`: a pupil "Pérez, Ana" in two classes
  with a mark in each; after migrating to 8 the pupil id, both `class_pupils` rows and both cells
  are unchanged, and both classes' sheets (`src/gradebook/sheet.rs`) show "Ana" / "Pérez".
- `tests/db.rs`: the existing `stepping_through_every_migration_keeps_the_data` already walks to
  `latest_version()`; it must still pass (its "Ana" has no comma, so is untouched).
- `tests/golden.rs`, `a_version_7_database_upgrades_and_splits_comma_names`: open `db-v7.db`
  through the normal open path; the comma pupil comes out split and the rest of the sample is as
  `assert_current_sheet` expects.
- `tests/backup.rs`, `restoring_an_old_backup_splits_comma_names`: a backup at schema 5 with a
  pupil "Pérez López, Ana" is restored, and in a second case merged with "Add classes from a
  backup"; both times the pupil arrives as "Ana" / "Pérez López". Proves old backups are covered.
- Full suite, `cargo test` with every group, desktop included. Demo data has no commas, so no
  baseline should change; if one does, find out why before re-recording.

### Docs
- `docs/user-guide.md`, "Pupils" bullet: replace "Pupils from before surnames existed keep their
  whole name as the first name; fix them by hand." with "Pupils from before surnames existed that
  were typed surname first, as in "Pérez López, Ana", are split into the two columns when you
  update; the others keep their whole name as the first name, to fix by hand."
- `docs/design-decisions.md`: a new D10 "Comma names are split once, by a migration" (Context,
  Decision, Consequences, Status), saying the comma form is split by migration 008, why a
  migration rather than a script or start-up code, and that names without a comma are never
  guessed.
- `CHANGELOG.md`, `[Unreleased]` → `### Added`: "Pupils typed surname first, as in "Pérez López,
  Ana", get their first name and surname in the right columns when you update; other names stay as
  they were".
- No locale change: no user-facing string is added.

### Done when
- [ ] `migrations/008_split_comma_names.sql` exists and is listed in `MIGRATIONS`.
- [ ] Every Behaviour bullet has an assertion in `migration_8_splits_names_written_surname_first`, and it passes.
- [ ] `tests/golden/db-v7.db` exists with a comma pupil, and its upgrade test passes.
- [ ] The backup restore and merge tests pass.
- [ ] `cargo test` passes in full with no re-recorded baseline.
- [ ] A copy of a 0.7.0 data folder with a "Surname, Name" pupil, opened in the built app, shows the
      name split in the sheet (screenshot of the class).
- [ ] User guide, design decisions and changelog updated; `metadata` run on every touched file.
