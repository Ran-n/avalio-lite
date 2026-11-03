[//]: # ( ---------------------------------------------------------------------- )
[//]: # (+ Authors: 	Ran# <ran.hash@proton.me> )
[//]: # (+ Created: 	2026/10/28 10:26:46.388247 )
[//]: # (+ Revised: 	2026/11/03 14:30:17.111348 )
[//]: # ( ---------------------------------------------------------------------- )

# TODO

The backlog. Each item is self-contained: reading it alone is enough to
implement it fully, without asking anything. Work top to bottom unless an item
says otherwise.

Next item number: 19

## Index

18. [Keep the app window active while the desktop tests capture it](#18-keep-the-app-window-active-while-the-desktop-tests-capture-it)

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

## 18. Keep the app window active while the desktop tests capture it

**Why:** on 2026-11-03 a full desktop run failed once in `the_column_panel_and_menus`: `column-panel-light-1000x600@100.png: 684 pixels changed (allowed 600)`. The only difference was the Title field's outline: grey `(207, 212, 221)` instead of the focused accent `(47, 84, 214)`. The same test passed on its rerun, and the light 1280 × 800 snapshot just before it showed the field focused. Slint draws a field as focused only while its window is the active one, and the desktop tests drive the app through posted messages without taking the foreground (they must never hijack the user's mouse or keyboard). So when another window comes to the front during the 12-minute run, a snapshot can catch every focused field unfocused. `App::capture` (`tests/desktop_support/mod.rs`) can't tell that from a real change, and with `AVALIO_UPDATE_BASELINES=1` it would record the wrong state as a baseline.
**Goal:** a snapshot always shows the app as the active window, whatever else is in front, without ever changing which window has the user's foreground.

### Behaviour
- Before each capture, the app's window is told it has focus, so its focused field draws focused, as a user sees it.
- The user's foreground window is never changed: no `SetForegroundWindow`, `SetActiveWindow` or simulated clicks on other windows.
- If the window still shows itself inactive after that (the posted message had no effect), the snapshot fails with "the app window isn't active; another window took the foreground during the run", not with a pixel count.

### Out of scope
- Keeping the window active between snapshots: only what is captured matters.

### Implementation
1. `tests/desktop_support/window.rs`: add `pub fn activate(&self)`, posting `WM_SETFOCUS` (0x0007) through the existing `post` helper. winit, Slint's Windows backend, turns `WM_SETFOCUS` into `WindowEvent::Focused(true)`, which makes the Slint window active again. If it doesn't, try `WM_ACTIVATE` with `WA_ACTIVE` (1), and keep whichever the test in step 3 shows working.
2. `tests/desktop_support/mod.rs`, `App::capture`: call `self.window.activate()` before `self.settle()`.
3. The driver (`examples/app_driver.rs`) reports in its `state` reply whether the Slint window is active (`window().is_active()` or the closest Slint 1.18 API; check its docs). `App::capture` asserts it after settling, with the message above.

### Tests
- `tests/desktop.rs`, new `a_snapshot_shows_the_window_active`: open the column panel (the Title field takes focus), post `WM_KILLFOCUS` (0x0008) to the window, take `app.snapshot("column-panel-light-1280x800")`, and assert it matches the existing baseline (the field drawn focused). This proves that a lost activation no longer reaches the image.
- The full desktop suite passes at 100 % with the baselines unchanged.

### Docs
- `docs/development.md`, *Tests*, desktop bullet: one sentence, "Each capture first tells the window it has focus, so a window brought to the front during the run doesn't change the images." Changelog: none (tests only).

### Done when
- [ ] `a_snapshot_shows_the_window_active` passes, and fails with the step-2 call removed.
- [ ] The full desktop suite passes at 100 % with `git status tests/desktop/baselines` unchanged.
