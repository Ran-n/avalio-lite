[//]: # ( ---------------------------------------------------------------------- )
[//]: # (+ Authors: 	Ran# <ran.hash@proton.me> )
[//]: # (+ Created: 	2026/11/02 21:31:24.223445 )
[//]: # (+ Revised: 	2026/11/25 11:43:00.467936 )
[//]: # ( ---------------------------------------------------------------------- )

# Design decisions

[README](../README.md) · [User guide](user-guide.md) · [Development guide](development.md) · **Design decisions**

Why the code is laid out as it is. Each entry is a short architecture decision record: the
**Context** (the problem, with the files involved), the **Decision**, its **Consequences** (what
gets easier, what it costs) and its **Status**. A decision is `Accepted (TODO item N)` until the
backlog item that carries it out ships, then `Done`. Decisions are never deleted: one that is
reversed gets the status `Superseded by Dn`, and the new entry says why.

Product decisions (column types, grade formulas) are in the [user guide](user-guide.md).

## D1. Package by feature, with a composition root and a window kit

**Context:** `src/app/` mirrored the feature folders: `app/gradebook/` wired `gradebook/`,
`app/backup.rs` wired `backup.rs`, `app/shortcuts.rs` wired `shortcuts.rs`, `app/lock_screen.rs`
wired `lock.rs`. A change to one feature edited two trees, and nothing showed which UI file
belonged to which feature.

**Decision:** each feature folder (`gradebook/`, `spreadsheet/`, `backup/`, `shortcuts/`,
`lock/`) holds its logic plus a `ui` submodule wiring it to Slint. `app/` only starts the app and
wires the features together (the composition root). `window/` is the shared UI kit every feature
UI uses: callback wrappers, shared handles, dialogs, the status and error line, theme, language.

**Consequences:** one folder per feature change. Feature UIs need things `app` used to own
(`SharedState`, `with_window`), and `app` needs every feature UI to wire it, so the `window/` kit
exists to break that cycle. Moving the files changes log targets and the paths tests import.

**Status:** Done

## D2. The app state is the core, outside the UI

**Context:** `src/app/state/` (`AppState`) held the settings, the open database, its PIN, data
moves and backups, with no UI code, yet lived inside the UI layer. That hid the app's core from
someone reading the tree and kept it out of mutation testing by accident, since
`.cargo/mutants.toml` excludes `src/app/**`.

**Decision:** `src/state/` at the crate root owns the settings, the open database, its PIN, data
moves and backups, with no Slint code, split into one file per concern. Its PIN methods stay in
`state/protection.rs` rather than `lock/`, because they need `AppState`'s private fields.

**Consequences:** the UI layer only wires; the state is mutation-tested. The move changes
`avalio_lite::app::state::…` to `avalio_lite::state::…`; no alias is kept at the old path, so new
code can't reach the core through the UI layer.

**Status:** Done

## D3. Shared infrastructure under `platform/`

**Context:** the top of `src/` mixed the app's features with plumbing every feature uses
(`config.rs`, `db/`, `logging.rs`, `storage.rs`), so the tree didn't show what the app is about.

**Decision:** `config`, `db`, `logging` and `storage` sit together under `src/platform/`. The top
level then reads as the app's features (Screaming Architecture). `error` and `i18n` stay at the
top: every layer uses them.

**Consequences:** paths become `avalio_lite::platform::{config, db, logging, storage}`, with no
re-export at the old ones. `include_str!` paths to `migrations/` gain a `../`.

**Status:** Done

## D4. Dependencies point one way

**Context:** layers pointed the wrong way in places that each looked fine file by file: the
gradebook imported the translations to name new columns (`gradebook → i18n → gradebook`), the
shortcut rules imported Slint's key events, and feature UIs reached into `app`.

**Decision:** dependencies run `app → feature ui → window → state → feature logic → platform →
error`. A file is UI if it is under `src/app/` or `src/window/`, is `src/lib.rs` or `src/main.rs`,
or has a `ui` path component (`src/<feature>/ui.rs`, `src/<feature>/ui/…`); every other file is
logic. `tests/architecture.rs` reads every `crate::…` and `super::…` path (and bare paths through a
module imported whole) and enforces four rules:

1. Logic names no `slint::` path, has no `use slint` and doesn't use `crate::ui`, `crate::app`,
   `crate::window` or any feature's `ui` module.
2. Nothing outside `src/app/` and `src/main.rs` uses `crate::app`.
3. Core logic, `src/gradebook/` (except `ui/`), `src/platform/db/`, `src/platform/storage.rs` and
   `src/lock.rs`, doesn't use `crate::i18n`; it uses ports (traits) the caller fills, such as
   `ColumnTitles`.
4. The module graph has no cycle. Its nodes are the top-level modules, each feature's `ui` and
   each `platform` module; a failure prints the cycle as `a → b → a`.

**Consequences:** the logic can be built, read and tested without the UI framework or the
translation tables. A port costs a trait and an adapter where a direct call used to do.

**Status:** Done

## D5. Files split at about 200 lines, by what changes together

**Context:** several files had grown well past 200 lines, each doing several jobs
(`ui/components.slint`, the large Slint pages, the shortcut rules, the translations, the Excel
import and export, backups, grade calculation), so one change meant scrolling through others.

**Decision:** larger files are split along the lines of change, never into arbitrary halves.
Public paths stay through `pub use` in the folder's module file (`<folder>.rs` beside it), so callers don't change. The translations are split the same way: `Message` joins one enum per topic (`src/i18n/messages/<topic>.rs`), and each language has one file per topic (`src/i18n/<language>/<topic>.rs`), so a translator works in one topic's file.

Clippy holds the limits in place (`clippy.toml` and `[lints.clippy]` in `Cargo.toml`): functions
of at most 40 lines and 4 parameters, no bool parameters, nesting at most 3 deep, a module file
beside its folder instead of `mod.rs`. An exception carries `#[expect(..., reason = "...")]`;
a bare `#[allow]` fails the lint.

**Consequences:** a translator works in one language's file, a menu change stays out of the
buttons' file. More files, and a helper a new sibling needs becomes `pub(super)`. Splitting the
translations by topic changed no word: `tests/golden/messages-en.txt` and `messages-gl.txt` hold
every message and error, and the golden test compares them with the code.

**Status:** Done

## D6. No repeated code

**Context:** the "no database open" guard was written out 13 times in `AppState`, the encrypted
file's "whole 512-byte pages" check twice with its own constant each time, and the UI's "log a
warning and show the error" pair in 10 places.

**Decision:** each lives in one helper: `open_config`, `open_connection` and `update_config` in the
state, `has_whole_pages` and `SMALLEST_PAGE` in the database module, and the `report_failure!`
macro in the window kit (a macro, so log lines keep the target of the feature that failed).

**Consequences:** one place to change each rule; log lines and messages stay as they were.

**Status:** Done

## D7. Background work goes through a Worker

**Context:** checking a licence goes online, which can take up to 15 seconds; the window must
never wait for it. Slint hands results back to the window through its event loop, which the
headless UI tests don't have, so work started from a thread there would never report back.

**Decision:** work that must not block the window goes through the `Worker` trait in
`src/window/worker.rs`. The job runs off the UI thread and returns what to do with the window;
`ThreadWorker` (the app) runs it on a thread and hands that back through the event loop, and
`InlineWorker` (the tests and the desktop driver) runs it at once. Nothing else spawns threads for
the UI. The worker lives in the window kit, not in a feature, because it needs Slint (D4) and
every feature that goes online uses it. `launch` waits for running work at shutdown.

**Consequences:** the UI tests drive activation and checks synchronously against a fake server.
The thread and the event-loop hand-off have a test binary of their own, `tests/worker.rs`, which
runs Slint's testing backend with an event loop; a screen that waits for work starts it with
`worker::run_or`, which ends the wait when the work can't start or breaks off.

**Status:** Done

## D8. In-app updates: two forges, a minisign signature, checked before the download

**Context:** the updater downloads a program and runs it, so a tampered or substituted file would
run with the user's rights. The release files carry no Windows (Authenticode) signature, and HTTPS
only proves a file came from a forge, not that the developer made it. All of the author's apps
update themselves the same way, whatever they are written in; this is that standard, built in Rust.

**Decision:** `src/update/`, one module per concern, none of them knowing the window except `ui`:

- **Which build** (`target`): a release build has its target (`windows-x86_64`) compiled in through
  `AVALIO_LITE_RELEASE_TARGET`; any other build is from source and never checks or replaces itself.
  The asset names are made in one function, which the release workflow also calls through
  `avalio-lite-asset-name`, so the two can't drift apart.
- **The check** (`source`, `check`): GitHub's and GitLab's latest release are asked at the same time
  on every check; the newest version either reports wins, so one forge can't hide a release or stop
  the updates by being down. A pre-release is only offered to a pre-release. The first automatic
  check is a minute after the window opens, then a day after the last successful check, or an hour
  after a failed one. Requests carry only the URL, `Accept` and `User-Agent`.
- **The proof** (`signature`, `download`): every asset has `<asset>.b3` (its BLAKE3 hash) and
  `<asset>.minisig`, a signature in minisign's format (`signature::minisign`), so anyone can check a
  file with `minisign -Vm <file> -P <key>`. Its trusted comment is the release statement,
  `avalio-lite-release-v2 version:<version> file:<asset> blake3:<hash>`; its first signature is
  over the file's BLAKE2b-512 hash (minisign's prehashed mode, the only one accepted; that is why
  BLAKE2b is used next to BLAKE3); the second is over the first plus the comment. The signature
  names its key by an 8-byte ID, the start of the key's BLAKE3 hash, and a signature whose ID is no
  trusted key's is refused. Two checks, in this order: before one byte of the asset is requested,
  the comment must be the one built from the offered version, name and `.b3` hash, and the second
  signature must hold; then the asset is streamed into a temporary file, hashed as it arrives
  (both hashes in one pass, `signature::hashes`), compared with the signed BLAKE3 hash, and the
  first signature checked over the bytes received. The limits are 200 MiB, 60 s without data and
  30 minutes in all. Anything that fails deletes the file. Two public keys are built in, the
  signing key and a backup; a signature by either is accepted, and the README publishes both.
- **Installing** (`install`, `registration`): installed and portable copies are the same binary,
  so both update by replacing the running exe (`install::replace`), closing as every quit does and
  starting the new one. The new program is copied whole beside the old one before the old one is
  moved aside, and the old one goes back if the swap fails, so a full disk never leaves no program
  (`self_replace::self_replace` moves the program away first; only its delete-after-exit is used).
  An installed copy brings its uninstall entry to the new version at the next
  start. Old download files are cleaned up after 40 minutes.

Release builds ignore every test override (`AVALIO_LITE_UPDATE_SOURCES`, `AVALIO_LITE_UPDATE_KEY`,
`AVALIO_LITE_UPDATE_DIR`, a run-time `AVALIO_LITE_RELEASE_TARGET`); debug builds read them, so the
tests run the whole path against a local server with a key of their own.

**Compared with common practice:** most desktop updaters trust the HTTPS download, or an
Authenticode signature bought from a certificate authority. Here the trust is our own key: no
certificate to buy or renew, and a compromised forge account alone can't push a working update.
Signing the version and the name as well as the hash means a genuine older build, or another
target's, can't be served as the offered one. Checking the proof first means a forged release
costs two tiny requests, not a 25 MB download.

**The bridge from 0.6.0 to 0.10.0:** those copies ask GitHub only, for
`avalio-lite-<version>-portable.exe` or `-setup.exe` with a `.sig` in their own format (base64url,
over three lines without the version). Until 2027-05-15 every release also carries those names,
byte-identical copies of the new files, signed in that format by the same key
(`signature::bridge`, written only by `avalio-lite-sign`; the app never accepts it). A release
therefore has twelve files: four exes, four `.b3`, a `.minisig` for each new name and a `.sig` for
each bridge name. A test runs 0.10.0's own check against what the tool writes, so the bridge can't
break unnoticed.

**Consequences and accepted limits:**

- No operating-system code signing: the first download from a release page isn't covered by the
  signature, only updates are. SmartScreen warns on that first download (README, "Unblock").
- No rollback.
- A forge reporting a fake higher version wins the comparison; its download then fails the
  signature check and the error is shown, rather than falling back to the other forge.
- The build attestation proves a file came from the release workflow, not that the workflow was
  honest; reproducible builds would close that.
- avalio-lite has no long tasks and saves as you type, so an update restarts the app as soon as
  it is in place: there is no "installed, starts when you close" state and no unsaved-edits
  question to hold the restart.
- Key custody: the signing key is GPG-encrypted in
  `~/.release-keys/avalio-lite-release-signing.key.gpg`, so a release is signed unattended, with a
  recovery copy in the maintainer's KeePass; the backup key's secret is only in KeePass, in a
  separate database, and on no computer.
- Losing the signing key: the next release is signed with the backup key and ships new built-in
  keys (docs/development.md, Releases). Copies from 0.6.0 to 0.10.0 trust only the first key, so
  if that one is lost they need one manual download.

**Status:** Done

## D9. Name columns are sheet columns

**Context:** pupils used to have one name, shown in a pupil column fixed at the left of the sheet
with its width in `classes`. Splitting the name into first name and surname, and letting teachers
put them anywhere (Surname first, or an info column before the names), needed the names to take
part in the column order that `sheet_columns.position` already kept for the mark columns.

**Decision:** every class has exactly one `first_name` and one `surname` row in `sheet_columns`
(migration 007; a trigger adds them to each new class, a partial unique index keeps them single,
and since migration 009 triggers keep them from being deleted or changed into another kind).
They hold no title, settings or cells: the names themselves stay in `pupils`, because a pupil is
shared by all their classes and their name is one fact. `SheetRow.values` carries the names in
those columns, so the sheet, the export and the import line up with `columns` without a special
case. Migration 007 rebuilds `sheet_columns` to widen its kind CHECK, so the migration runner turns
foreign keys off around migrations and checks them with `foreign_key_check` before committing.

**Consequences:** one ordered list means one way to move, resize, drag and export every column; the
× that removes a pupil moved to a fixed strip at the start of each row, so it stays reachable
wherever the names are. Code that means "typed cells" asks `ColumnKind::holds_cells` rather than
"not computed", since name columns are neither.

**Status:** Done

## D10. Comma names are split once, by a migration

**Context:** migration 006 gave pupils a surname but left every existing name whole in the first
name, because guessing where "Xoán Carlos Pérez" splits would get some names wrong. Many teachers
had typed their pupils as school lists write them, "Pérez López, Ana", and in that form the comma
marks the split, so fixing them by hand was work the app could do without guessing.

**Decision:** migration 008 splits a pupil whose surname is empty and whose name has exactly one
comma with text on both sides: the part before the comma becomes the surname, the part after the
first name, both trimmed. Everything else stays as it was. It is a migration rather than a script
for teachers to run or a check at start-up: it runs once per database, in its own transaction, and
backups opened for restoring or adding classes go through the same migrations, so old backups are
split too. It is a new migration rather than an edit to 006, which had already run on databases
at version 6 and 7.

**Consequences:** no Rust code path and nothing at a normal start; names without a comma are
still never guessed. A whole-name column in an Excel import still lands in the first name, since
splitting it there would be permanent code; that can be a decision of its own.

**Status:** Done

## D11. The schema is checked for BCNF, table by table

**Context:** every schema must be in Boyce-Codd normal form (`feedback_database_bcnf`): for every non-trivial dependency, its determinant is a superkey. Nothing wrote the check down, and `sheet_columns` has settings that depend on its `kind` (enforced by CHECK constraints), which needed an argument rather than an assumption. The check first ran on migrations `001_initial.sql` to `010_names_required.sql`, and found one violation, the rounded column's class copy. It is checked again below as the schema stands after `012_pupil_pictures.sql`.

**Decision:** the check, table by table:

| Table | Candidate keys | Dependencies | Result |
|---|---|---|---|
| `classes(id, name, created_at)` | `id`; `name` (UNIQUE, 001) | `id → name, created_at`; `name → id, created_at` | BCNF: both determinants are keys |
| `pupils(id, first_name, surname, created_at)` | `id` | `id → first_name, surname, created_at` | BCNF. `first_name` isn't a key, since two pupils may share a name and none is merged by guess |
| `class_pupils(class_id, pupil_id, position)` | `(class_id, pupil_id)` | `(class_id, pupil_id) → position` | BCNF. `position` depends on the whole key. A UNIQUE `(class_id, position)` added later would be a second key, and the table would still be BCNF |
| `cells(column_id, pupil_id, value)` | `(column_id, pupil_id)` | `(column_id, pupil_id) → value` | BCNF. Nothing reachable through `column_id → sheet_columns.class_id` is copied; "the pupil is in the column's class" is a trigger (003), not a dependency inside `cells` |
| `sheet_columns` | `id`. The partial UNIQUE `(class_id, kind)` (007) covers name rows only, so it is not a key of the table | `id →` every column | BCNF. The rounding source is not stored here (see `round_sources`) |
| `round_sources(column_id, source_column_id)` | `column_id` (the primary key) | `column_id → source_column_id` | BCNF. `source_column_id` determines nothing in the table, and the class of the two columns is not copied: the triggers of 011 keep them in one class |
| `pupil_pictures(pupil_id, image)` | `pupil_id` (the primary key) | `pupil_id → image` | BCNF. A table of its own, since most pupils have no picture. `image` isn't a key: two pupils may be given the same photo |

**The settings of `sheet_columns`.** `title`, `weight`, `max_score`, `decimals`, `bonus` and `width` each depend on `id` alone. The CHECK constraints tie `kind` to which of them are NULL (or `''` for a name column's title). That is an applicability rule, not a dependency: `kind` doesn't determine a value, since two test columns hold different weights. A NULL means "this setting doesn't apply to this kind", so it is not a transitive dependency. Name columns hold no settings.

**The violation, now fixed.** Up to 010, a rounded column's source was a column of `sheet_columns`, and the triggers forced it into the same class. So for rounded rows `source_column_id → class_id` held, and `class_id` was reached only through a non-key column: a transitive dependency, so the table was not BCNF. The file also didn't stop a source from moving to another class, which left its rounded column pointing across classes.

**The fix (011).** The link has a table of its own, `round_sources`, keyed by the rounded column. Its triggers refuse a link unless the rounded column is `round` and the source is a test, final grade or average of the same class. The triggers on `sheet_columns` refuse a change to the class or kind of a linked column that would break its link. Deleting either column deletes the link. An old file may hold a link the new rules refuse, such as a source moved to another class before 011. The migration drops such a link, keeps the rounded column's decimals, and logs a warning, rather than failing to open the file.

**Status:** Check done; the schema is BCNF

## D12. The pupil view replaces the main screen

**Context:** a teacher wants to let a pupil see their own grades on the teacher's computer, with the app open, and nothing of anyone else's. Hiding parts of the sheet would leave every other way into it (shortcuts, menus, the update bar, what a screen reader reads) to be hidden one by one.

**Decision:** the view is a stage of the window (`PupilViewStage`: `off`, `menu`, `grades`, `pin`), and while it isn't `off` the window doesn't make the `Shell` (sidebar, sheet, Settings, update bar) at all. Only Rust (`src/pupil_view/ui.rs`) changes the stage, and each callback first checks the stage it belongs to, so no path skips the PIN. `src/pupil_view.rs` decides what a pupil sees by listing the column kinds one by one, so a new kind doesn't compile until it is decided on. The grades are put in the window when they are shown and cleared on every way out of the page; the list of names is cleared while a pupil looks. The PIN page is the lock screen's with another title, a way back to the list and no "Forgot the PIN?", and it shares the lock screen's `AttemptLimiter`, made in `app.rs`. The PIN is checked against the key in memory; the database stays open.

Where the PIN is asked, if there is one (the user, 2026-11-25):

| Step | Asks | Why |
|---|---|---|
| Sheet → list | no | the teacher is at the sheet, so the PIN was typed already |
| List → a pupil, also the same one again | yes | it is what opens someone's grades |
| A pupil → the next | yes | the same |
| A pupil, or the PIN page → list | no | the list holds names, numbers and pictures, no grades |
| List → sheet | yes | it opens everything |

So it is typed once per pupil, the fewest that still keeps a pupil left alone with the computer from any other's grades. It is the app's PIN and not a short code for the session: one secret to keep, and the same guess limit. With no PIN nothing is asked, and the list says what that means. Pupils go by their number, their place in the class by surname, which is worked out and never stored.

Locking ends the view. It doesn't start while an update downloads, because the restart would open the sheet when there is no PIN. The user also decided that the teacher picks in any order or goes by number, and that only the open class's grades show.

**Consequences:** the right PIN, or a database with no PIN, is the only way to a second pupil's grades. Whoever adds a page that should show over the main screen must decide whether it shows over the pupil view too.

**Status:** Done

## D13. Pupils' pictures live in the database, small and bare

**Context:** pupils may have a picture, shown on the sheet and in the pupil view. A photo of a minor is the most sensitive thing the app holds: it must be as protected as the marks, go wherever they go, and not carry more than a face.

**Decision:** a picture is a row of `pupil_pictures` (012), not a file in the data folder. So the PIN encrypts it, backups, data moves and clean shutdown need nothing new, and deleting the pupil deletes it. What is stored is the middle square of the photo, turned upright, 256 pixels to a side, as a JPEG made from the pixels alone: the photo's place, time and camera never reach the database, and a class of thirty usually weighs under a megabyte. The table refuses anything over 256 KiB. `gradebook/pictures.rs` reads JPEG, PNG and WebP (the `image` crate with those decoders only, all in Rust), refuses files over 50 MB and limits the size and memory of what they unpack to. A file's name is often a pupil's, so errors show it on screen but leave it out of the log.

A stored picture that can't be read is logged and the initials show instead: one bad row doesn't keep a class from opening. Adding a backup's classes never replaces a picture that is already here.

**Consequences:** a picture can't be taken back out at its first size. A new place that shows pupils has the round avatar (`ui/components/avatar.slint`) to show them with.

**Status:** Done
