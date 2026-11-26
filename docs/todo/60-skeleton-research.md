[//]: # ( ---------------------------------------------------------------------- )
[//]: # (+ Authors: 	Ran# <ran.hash@proton.me> )
[//]: # (+ Created: 	2026/11/23 00:02:35.499935 )
[//]: # (+ Revised: 	2026/11/23 00:02:35.499935 )
[//]: # ( ---------------------------------------------------------------------- )

# Item 60 research: a Rust + Slint desktop app skeleton

Research notes for TODO item 60, made 2026-11-22. They are not approved. Talk this through with the user
before saving anything as a rule.

## Proposed tree

```
avalio-lite/                          virtual workspace: members = ["crates/*/*"],
│                                     default-members = [app], [workspace.package|dependencies|lints]
├── crates/
│   ├── app/avalio-lite/              main, startup, composition; tests (entry, architecture, rendered UI)
│   ├── model/avalio-lite-{backup,gradebook,licence,lock,shortcuts,signature,spreadsheet,state,update}/
│   ├── ui/
│   │   ├── shared/avalio-lite-ui-shared/          build.rs (one slint-build), ui/*.slint, lang/, i18n
│   │   ├── workspace/avalio-lite-ui-workspace/    the window that places the parts
│   │   └── feature/avalio-lite-ui-{about,backup,gradebook,licence,lock,shortcuts,spreadsheet,update}/
│   ├── platform/avalio-lite-{config,db,error,http,logging,storage,windows}/   db holds migrations/
│   └── tools/avalio-lite-{asset-name,sign}/
├── assets/  packaging/windows/  docs/  .github/workflows/
```

The architecture test would move to `app` and read `cargo metadata`, not source text.

## Dependency graph

```
app/avalio-lite   -> every ui/*, model/*, platform/*
ui/feature/*      -> ui/shared, model/<same name>, platform/*
ui/workspace      -> ui/shared
ui/shared         -> platform/config, platform/error, slint
model/*           -> model/* (state uses the rest), platform/*
platform/*        -> platform/error, OS crates only
tools/*           -> model/signature, model/update
```

## The open question: how the UI layer is split

- **A. One `ui` crate**, one slint-build, one `AppWindow`. This is what Slint documents. Features are modules,
  and tests/architecture.rs keeps their boundaries.
- **B. Rust split, one Slint crate.** `ui/shared` holds the only `build.rs`, every `.slint` file and the
  translations. Feature crates are Rust only and wire their part to the generated types. The build enforces
  the boundaries, and only documented Slint APIs are used. The cost: a look change touches two crates. Nobody
  has yet tried using the generated types from feature crates.
- **C. One `.slint` compile unit per feature crate.** This needs `slint-build/experimental-module-builds`. KDAB
  says global handling "does not yet cover all edge cases", and a diamond of crates instantiates globals twice.
  Each crate would also need its own translation domain. Not recommended.

The user's point (2026-11-22): the skeleton depends on the type of app more than on the language. It would be
saved for a "Rust + Slint desktop app".

## Already decided

- The shared error type goes in `platform/avalio-lite-error`. The definition of platform widens to "OS access
  and shared foundations".

## Deviations and gaps found

- Cargo recommends a flat `crates/` ("Keep all member packages in a flat directory"). It still supports nested
  globs (`"path/to/member3/*"` in its own example), so the nesting is a deliberate deviation.
- `[lints.clippy]` becomes `[workspace.lints.clippy]`, and every member needs `[lints] workspace = true`.
  Nobody has checked whether `clippy.toml` is found from member crates.
- A virtual workspace uses all members unless `default-members` is set, so it must name `app/avalio-lite` for
  `cargo run`.
- The bundled translations domain is the crate name (`<lang>/LC_MESSAGES/<domain>.po`), so `lang/` moves into
  `ui/shared`, renamed to that crate's name.
- `i-slint-backend-testing` is internal and not semver-stable, so pin it exactly, behind a `test-support`
  feature.
- The Slint docs are silent on cross-crate `.slint` imports in the stable reference.

## Sources

- [Cargo: Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [slint-build](https://docs.rs/slint-build/latest/slint_build/) and its
  [CompilerConfiguration](https://docs.rs/slint-build/latest/slint_build/struct.CompilerConfiguration.html)
- [Slint include_modules!](https://docs.slint.dev/latest/docs/rust/slint/macro.include_modules.html)
- [Slint translations guide](https://docs.slint.dev/latest/docs/slint/guide/development/translations/)
- [i-slint-backend-testing](https://docs.rs/i-slint-backend-testing/latest/i_slint_backend_testing/)
- [KDAB: reusable Slint UI libraries with Rust crates](https://www.kdab.com/building-reusable-slint-ui-libraries-with-rust-crates/)
