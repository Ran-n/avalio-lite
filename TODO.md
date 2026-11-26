[//]: # ( ---------------------------------------------------------------------- )
[//]: # (+ Authors: 	Ran# <ran.hash@proton.me> )
[//]: # (+ Created: 	2026/10/28 10:26:46.388247 )
[//]: # (+ Revised: 	2026/11/26 12:55:30.119576 )
[//]: # ( ---------------------------------------------------------------------- )

# TODO

The planned work. Each item is self-contained: reading it alone is enough to
implement it fully, without asking anything. Work top to bottom unless an item
says otherwise. Ideas not planned yet live in `BACKLOG.md`.

| To do | Backlog |
|:-----:|:-------:|
| **2** | **0** |

Next item number: 69

## Index

- [50. Keep the release signing key GPG-encrypted for unattended signing](#50-keep-the-release-signing-key-gpg-encrypted-for-unattended-signing)
- [60. Propose a Rust and Slint source skeleton for approval](#60-propose-a-rust-and-slint-source-skeleton-for-approval)

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
- Keep the Index in file order: one `- [<n>. title](#anchor)` line per item;
  update it whenever an item is added, retitled, moved or deleted.
- Keep the count table above the Index equal to the items here and in
  `BACKLOG.md`, in both files; update it whenever an item is added, deleted,
  promoted or parked.
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
   new items (here if it's planned, in `BACKLOG.md` if not).

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

## 50. Keep the release signing key GPG-encrypted for unattended signing

**Why:** `feedback_in_app_updater.md` (section 7, "Release key custody") says the signing key lives in
`~/.release-keys/<app>-release-signing.key.gpg`, encrypted to the user's GPG key, with a copy in KeePass. Claude
uses it for every release without asking the user for anything. The backup key lives only in KeePass. Today
`scripts/release.ps1:22,137-139,216-217` reads the key from KeePassXC (`keepassxc-cli show -s -a Password
<db> "avalio-lite release key"`), which asks the user for the database password on every release.
`~/.release-keys/` has no avalio lite file. `docs/development.md:161` and the project spec line 39 describe the
KeePassXC route. The user decided this is a gap: follow the global rule.

**Goal:** a release is signed unattended from
`~/.release-keys/avalio-lite-release-signing.key.gpg`. The backup's secret is only in KeePass, and the signing
key's KeePass copy stays as its recovery copy.

### Behaviour
- `release.ps1` decrypts the key into a pipe, never to disk:
  `gpg --batch -q --pinentry-mode error -d ~/.release-keys/avalio-lite-release-signing.key.gpg | avalio-lite-sign sign - <assets>`.
  It runs this pipe through Git Bash, because PowerShell 5.1 re-encodes native output and adds CRLF.
- If decrypting fails (the GPG key is missing or gained a passphrase), the script stops with that reason. It never
  falls back to a plain file or to KeePassXC.
- No plain copy of the signing key ever touches disk while the `.gpg` file is made.

### Out of scope
- The backup key and the signature format.
- Changing the keys themselves: `BUILT_IN_KEYS[0]` stays.

### Implementation
1. Make the `.gpg` file with the user at the keyboard, because KeePassXC asks for its password once. In Git Bash:
   `keepassxc-cli show -s -a Password <releases.kdbx> "avalio-lite release key" | gpg --batch -e -r AA4C66BE276AA0D8D95F003D11705FC643951387 -o ~/.release-keys/avalio-lite-release-signing.key.gpg`.
   The fingerprint is the one on the `GPG` line of `CONTRIBUTORS`; always use the full fingerprint, never a name.
2. Check it, without printing the key:
   `gpg --batch -q --pinentry-mode error -d ~/.release-keys/avalio-lite-release-signing.key.gpg | cargo run --release --quiet --bin avalio-lite-sign -- public -`
   must print `BUILT_IN_KEYS[0]`. Repeat with Gpg4win's `gpg` (`%APPDATA%\gnupg`), since both GnuPG installs must
   decrypt with no prompt.
3. `scripts/release.ps1`:
   - drop `-KeePassXCli`, `-KeyDatabase` and `-KeyEntry`;
   - add `-SigningKey` with the default `$HOME\.release-keys\avalio-lite-release-signing.key.gpg`, and fail early if
     the file or `gpg` is missing;
   - build `avalio-lite-sign` once (`cargo build --release --bin avalio-lite-sign`) and run
     `bash -c "gpg --batch -q --pinentry-mode error -d '<key>' | '<target>/release/avalio-lite-sign.exe' sign - <assets>"`;
   - check `$LASTEXITCODE`, and keep `-DryRun` printing the command.
4. Update the header comment of `scripts/release.ps1` (lines 8-16) to the new needs: gpg with the user's key, Git
   Bash, gh, glab, b3sum, curl, `CODEBERG_TOKEN`.
5. Run `.\scripts\release.ps1 -DryRun` on the current tag to check that it prints the gpg pipe and no KeePassXC step.

### Tests
- `.\scripts\release.ps1 -DryRun`: prints the gpg pipe, finds the key file, and fails with a clear message when
  `-SigningKey` points at a missing file.
- The step 2 check (public key equals `BUILT_IN_KEYS[0]`) is recorded with its output in the hand-over.
- `cargo test` stays green. No Rust code changes, apart from any trimming fix the pipe shows is needed. If one is
  needed, add a test that a key with `\r\n` from a pipe reads.

### Docs
- `docs/development.md` Signing keys (lines 159-172):
  - the signing key is `~/.release-keys/avalio-lite-release-signing.key.gpg`, encrypted to the CONTRIBUTORS GPG
    fingerprint, with a copy in KeePass; the backup is only in KeePass;
  - list the GPG pipe, and the KeePassXC pipe (`keepassxc-cli show -s -a Password <db> "<entry>" | avalio-lite-sign sign - …`)
    for other maintainers;
  - say "never decrypt the key to disk; never delete the `.gpg` file".
- `docs/design-decisions.md` D8, "Losing the signing key": name the new custody.
- Project spec line 39: "The release signing key is in `~/.release-keys/avalio-lite-release-signing.key.gpg`
  (GPG-encrypted, never delete it), with a copy in KeePass; the backup key is only in KeePass." The user asked
  for this change.
- Changelog: none (maintainer tooling only).

### Done when
- [ ] `~/.release-keys/avalio-lite-release-signing.key.gpg` exists and decrypts, with no prompt, to the key whose
      public half is `BUILT_IN_KEYS[0]`, under both GnuPG installs.
- [ ] `release.ps1 -DryRun` shows the gpg pipe; no KeePassXC parameter is left.
- [ ] The development guide, D8 and the spec describe the new custody.

---

## 60. Propose a Rust and Slint source skeleton for approval

**Why:** `feedback_source_organization.md` says every project uses an approved skeleton: the base skeleton's
layers as build-enforced packages (`app/<app>`, `model/<domain>`, `ui/{shared,workspace,<feature>}`,
`platform/<thing>`, `tools/<tool>`), package names `<app>`, `<app>-<name>` and `<app>-ui-<name>`, and the dependency
direction `app` → all, `ui` → `model` + `platform`, `model` → `platform`, `tools` → `model`. Only Rust + gpui is
approved. For any other tech, the rule says: say none is defined, research it (official sources first), propose
one skeleton, and get the user's choice. avalio lite is one crate, `src/<feature>/` with `ui` submodules
(D1–D4). `tests/architecture.rs` enforces its direction by discipline, not by the build. The release tools sit
in `src/bin/`, not `tools/`. Rust + Slint has no approved skeleton.

Research is done ([docs/todo/60-skeleton-research.md](docs/todo/60-skeleton-research.md)). The shared error type goes in
`platform/avalio-lite-error` for now: the user picked it, then said they didn't understand the choice, so settle it in the
same talk. On 2026-11-22 the user deferred the UI-split choice ("we'll talk about it later"): pick this
item up only after that talk. The skeleton is for a Rust + Slint *desktop app*, since the app type matters more than the
language. The design entry is D14, because D11 is the BCNF check, D12 the pupil view and D13 the pictures.

**Goal:** a written proposal the user approves or changes, saved in the global memory. After approval, the moves
are filed as their own items. None is done here.

### Behaviour
- The proposal is a tree with depth lines: a Cargo workspace (`members = ["crates/*/*"]`, shared version,
  dependencies and `[workspace.lints]`), `crates/app/avalio-lite`, `crates/model/<domain>` (gradebook,
  spreadsheet, backup, licence, update, shortcuts, state), `crates/ui/{shared,workspace,<feature>}`,
  `crates/platform/{config,db,logging,storage,windows}` and `crates/tools/{sign,asset-name}`, plus `ui/` `.slint`
  placement and `migrations/`, `lang/`, `assets/`, `packaging/windows/`.
- It answers the Slint-specific questions from Slint's own docs:
  - where `.slint` files live and which crate runs `slint-build` (one `ui` crate compiling all `.slint`, or one per
    feature crate);
  - how `@tr` translations and `lang/` bundling work per crate;
  - how the headless testing backend (`i-slint-backend-testing`) is shared through a `test-support` feature.
- It names where it follows and where it deviates from the Cargo book (flat `crates/`) and Slint's guidance.

### Out of scope
- Moving any file. After approval, one item per layer goes at the end of this file (tools, platform, model, ui,
  app), each moving one layer with every import updated.

### Implementation
1. Read the official sources: the Cargo book (workspaces, `[workspace.lints]`, package layout), Slint's docs
   (Rust API `slint-build`, `slint::include_modules!`, translations, the testing backend), and Slint's own
   repository examples with several crates. Quote what they say, and say where they are silent.
2. Read `tests/architecture.rs` and D1–D4, and map every current module to its target crate in a table (module →
   crate).
3. Write the proposal (tree, dependency graph, the Slint answers, deviations) in the reply. Ask the user with
   AskUserQuestion to approve it or pick between the real alternatives found in step 1 (for example one `ui` crate
   or one per feature).
4. On approval, add it to `~/.claude/memory/feedback_source_organization.md` under "Approved
   skeletons" as "Rust + Slint desktop app", with its reason and the date. Add a D14 to `docs/design-decisions.md`
   (the chosen skeleton and its deviations, Status `Accepted (TODO items …)`).
5. File the move items with `/todo`, in dependency order.

### Tests
- None for the proposal. Each move item lists its tests (architecture rules enforced by crate boundaries, and the
  full suite).

### Docs
- `docs/design-decisions.md` D14 (on approval). Changelog: none.

### Done when
- [ ] The user approved a skeleton, and it is saved in the global memory.
- [ ] D14 is written and the move items are filed.
