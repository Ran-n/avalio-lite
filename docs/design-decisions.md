[//]: # ( ---------------------------------------------------------------------- )
[//]: # (+ Authors: 	Ran# <ran.hash@proton.me> )
[//]: # (+ Created: 	2026/11/02 21:31:24.223445 )
[//]: # (+ Revised: 	2026/11/07 20:25:35.695351 )
[//]: # ( ---------------------------------------------------------------------- )

# Design decisions

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
   `src/lock/mod.rs`, doesn't use `crate::i18n`; it uses ports (traits) the caller fills, such as
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
Public paths stay through `pub use` in the folder's `mod.rs`, so callers don't change. Allowed
exceptions: `state/protection.rs` and `platform/logging.rs` carry `#[cfg(test)] mod tests` that
need private access, so only their code counts towards the limit.

**Consequences:** a translator works in one language's file, a menu change stays out of the
buttons' file. More files, and a helper a new sibling needs becomes `pub(super)`.

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

**Consequences:** the UI tests drive activation and checks synchronously against a fake server;
the thread and the event-loop hand-off are only exercised by the real exe and the desktop tests.

**Status:** Done

## D8. Updates are trusted by our own signature, not by Windows

**Context:** the updater downloads an exe and runs it, so a tampered or substituted file would run
with the user's rights. The release files carry no Windows (Authenticode) signature, and HTTPS
only proves a file came from GitHub, not that the developer made it.

**Decision:** `scripts/release.ps1` signs every release file with an Ed25519 key kept outside the
repository; the matching public key is built into the app (`src/update/mod.rs`). The signature
covers a fixed context, the file name and the file's BLAKE3 hash (`src/update/signature.rs`), so
it can't be reused for another file or purpose. `src/update/` downloads the file and its `.sig`,
checks both, and deletes a file that doesn't verify before anything can run it. Releases are read
from GitHub only.

**Consequences:** no certificate to buy or renew, and a compromised GitHub account alone can't push
a working update. Losing the private key means existing copies can no longer update themselves (a
new key needs one manual install); leaking it lets anyone sign updates, so it stays out of the
repository (`*.key` is ignored), with a second copy kept offline.

**Status:** Done
