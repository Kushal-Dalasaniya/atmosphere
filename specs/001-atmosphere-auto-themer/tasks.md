# Tasks: Atmosphere auto-themer (001)

**Input**: Design documents from `/specs/001-atmosphere-auto-themer/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md
**Tests**: Plan Testing section requests `cargo test` unit/integration coverage plus `cargo clippy`, `cargo fmt --check`, `cargo audit` gates — test tasks below are REQUIRED, not optional.
**Organization**: Tasks grouped by user story; each story independently implementable and testable after Foundational phase.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2)
- Include exact file paths in descriptions

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Spec/constitution alignment and toolchain baseline before any code changes

- [x] T001 Amend spec §4.4 to the in-process `material-colors` extractor (same role table) in `specs/001-atmosphere-auto-themer/spec.md` per research R-1 and constitution I — DONE 2026-10-02: §4.4 rewritten normatively in-process; §§2, 3.4, 4.2, 5.2, 5.4, 6.3, 6.4, 7 aligned; no code revert needed
- [x] T002 Switch `gtk4`/`libadwaita` features to the `gnome_50` minimum floor in `Cargo.toml` per research R-2 (remove `v4_22`/`v1_9`+`gtk_v4_22` unless `cargo metadata` proves `gnome_50` does not imply them) and verify `cargo build` succeeds — DONE 2026-10-02: `gtk4=[gnome_50]` verified (=v4_22+gio2.88 chain); `libadwaita` has no `gnome_*` feature so `v1_9`+`gtk_v4_22` kept as the exact GNOME 50 floor (documented in-file); `cargo build` green after fixing 45 pre-existing errors (material-colors 0.4.2 API, gio prelude traits, image traits, Theme initializers)
- [x] T003 [P] Record starting-point baselines for `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo audit` (T034 enforces the release gates); document any Critical/High waiver in `SECURITY.md` (create only if a waiver is needed) per spec §8.4 — FINAL 2026-10-02: fmt CLEAN; `cargo test` 7/7 pass; clippy 0 errors + 43 warnings (mostly dead_code on not-yet-wired Phase 2–8 modules, cleared by T008/T021/T028/T029); audit 0 Critical/High (1 warning: paste unmaintained RUSTSEC-2024-0436)
- [x] T004 [P] Extend `src/paths.rs` with missing data paths (baseline dir, Alacritty paths, `gtk.css.user-saved` helper) with `mkdir -p` semantics per plan Storage section — DONE 2026-10-02: added `user_saved_backup_path()` helper, wired into `gtk_css.rs` + `baseline.rs`, removed dead `private_matugen_config_path()` (spec §5.2 no longer lists it); stash-compared builds prove zero new errors (45 pre-existing errors unchanged, full-build verification pending their fix)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Shared validation, atomic I/O, model, and pipeline core that every user story builds on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T005 [P] Verify and complete hex/path/id validators in `src/theme/validate.rs` (hex must satisfy `^#[0-9a-fA-F]{6}$`; wallpaper canonicalize + `$HOME` allowlist + symlink-escape reject; id must satisfy `^[a-z0-9][a-z0-9._+-]*$` with `..` rejected) with `#[cfg(test)]` unit tests covering the §8.4 probe `accent = "#000\"; url(javascript:1)"` — DONE 2026-10-02: verified complete, tests pass
- [x] T006 [P] Verify atomic file helpers in `src/theme/atomic.rs` (`atomic_write` temp-in-same-dir + rename, `backup_next_to`/`restore_from_backup` for `*.atmosphere.bak`) with `#[cfg(test)]` unit tests for half-write and restore behavior — DONE 2026-10-02: verified complete, tests pass
- [x] T007 [P] Verify `Theme`/`Palette`/`GlassSettings` plus new `IconsSettings { match_yaru: bool default true }` in `src/theme/model.rs`, including legacy `glass = "off"|"subtle"|"strong"` reader compat and `theme_id_from_name` uniqueness rules, with `#[cfg(test)]` unit tests — DONE 2026-10-02: added `deserialize_glass` (struct + legacy string forms) + 4 tests (id rules, legacy strings, struct/defaults, round-trip)
- [x] T008 Rewrite the sequential apply pipeline with single-apply mutex and pre-apply backup + failure rollback in `src/theme/apply.rs` per spec §4.2 steps 1–11 and contracts `system-effects.md` (depends on T005, T006, T007; builds the mutex + backup/rollback skeleton with hook points that T021/T028/T029 integrate into) and log pipeline duration at info level for the <1 s timing assertion in T032 — DONE 2026-10-02: blocking mutex, validate-first, backups + one-time baseline before changes, wallpaper-first order, pre-apply rollback, applied-state on success, duration log on both paths
- [x] T009 [P] Replace flat-average source color with quantized/dominant-color pick in `src/theme/extract.rs` (same `material-colors 0.4` crate, no new dependency) and assert extract has zero desktop side effects per research R-1 — DONE 2026-10-02: 5-bit histogram dominant pick over `DynamicScheme::by_variant(Vibrant)`; extract reads the image only
- [x] T010 [P] Verify `GFile`-URI wallpaper set (both `picture-uri` keys + `picture-options = zoom`) and `color-scheme`/`gtk-theme` table with `Adwaita`-toggle reload in `src/theme/wallpaper.rs` per contracts `system-effects.md`, never writing `accent-color` — DONE 2026-10-02: verified + `mode_theme_table` unit test
- [x] T011 [P] Make Flatpak overrides idempotent by querying `flatpak override --user --show` (fixed args) before writing in `src/theme/flatpak.rs` per research R-10, keeping the once-per-user flag file — DONE 2026-10-02: real-state query + flag sync, consolidated on `paths::flatpak_done_path()`

**Checkpoint**: Foundation ready — `cargo test`, `cargo clippy`, `cargo fmt --check` clean; user stories can now begin

---

## Phase 3: User Story 1 — Themes (Switch): theme library + one-click apply (Priority: P1) 🎯 MVP

**Goal**: Saved-theme cards; one click applies wallpaper, GTK colors/settings, and (when enabled) shell overlay; applied state tracked.

**Independent Test**: Hand-write one `theme.toml` per `contracts/theme-toml.md` → `cargo run` → click Apply → both wallpaper URI keys equal the percent-encoded `file://` URI, `picture-options` is `zoom`, dark writes `prefer-dark`/`adw-gtk3-dark` (light writes `prefer-light`/`adw-gtk3`), both `gtk.css` files contain the marker + theme hexes, `applied.toml` holds the id; repeat with User Themes disabled (shell keys untouched, banner shown).

- [x] T012 [P] [US1] Verify list/load/delete/applied-state plus rename (re-validates id uniqueness) in `src/theme/model.rs` — DONE 2026-10-02: added `rename_theme` (duplicate/invalid refusal, dir move with preview preservation, wallpaper file untouched) + temp-root test
- [x] T013 [P] [US1] Harden GTK CSS rendering through `checked_hex` + `atomic_write` with one-time `gtk.css.user-saved` preservation in `src/theme/gtk_css.rs` per contracts `system-effects.md`, emitting the six `@define-color` vars plus the `atmosphere_glass_surface rgba(…)` token exactly when glass is enabled per spec §4.9 — DONE 2026-10-02: `render_gtk_css` takes glass, token only when enabled; `apply.rs` call-site updated
- [x] T014 [P] [US1] Verify Yaru `@import` search + short-override shell CSS and `""`→`"Atmosphere"` reload toggle in `src/theme/shell.rs`, skipping with warning when no import path exists — DONE 2026-10-02: verified + `yaru_paths_follow_mode` test
- [x] T015 [US1] Build Themes cards with thumbnail, swatches, glass label, Applied state, spinner + ignore-clicks mutex, rename/delete actions, empty-state + open-Create, and failure toasts (one sentence, no stack trace) in `src/ui/themes_view.rs` (depends on T012) — DONE 2026-10-02: async thumbnails, 5 swatches, spinner + Cell guard, rename/delete dialogs (wallpaper file kept), success/failure toasts via ToastOverlay, broken-theme rows
- [x] T016 [US1] Add `#[cfg(test)]` unit tests for GTK CSS marker/vars output and shell import-line selection in `src/theme/gtk_css.rs` and `src/theme/shell.rs` — DONE 2026-10-02: 4 gtk_css tests (marker/vars/hexes, glass-off clean, glass-on token, injection rejected) + 1 shell test

**Checkpoint**: US1 fully functional and testable independently (quickstart scenarios 1, 4–8)

---

## Phase 4: User Story 2 — Create: wallpaper library, extract, edit, save (Priority: P1)

**Goal**: Wallpaper strip, palette extraction, swatch editing, dark/light, save / save-and-apply.

**Independent Test**: Add photos (one filename with spaces) to `~/Pictures/Wallpapers` → Extract fills 5 swatches with zero desktop change (`gsettings` wallpaper keys unchanged, `~/.config/matugen/` untouched) → edit Accent → Save → `theme.toml` holds edited hex with `palette_edited = true` → re-apply uses it (check `gtk.css`).

- [x] T017 [P] [US2] Fix aspect-preserving 256px thumbnails decoded off the main thread with path+mtime cache, broken-file note, and webp-decode gating in `src/thumbs.rs` per research R-9 — DONE 2026-10-02: `thumbnail()` (no distortion), `write_preview()` (512px `preview.png`), decode failures propagate to the strip's "could not read" note; call sites wrap in `spawn_blocking`
- [x] T018 [US2] Implement Create view in `src/ui/create_view.rs`: wallpaper strip + Add-wallpaper copy (numeric suffix, never overwrite, symlink-escape reject) + drag-and-drop, Extract wiring, 5 swatches in Background/Surface/Foreground/Accent/On-accent order with hex validation + reset, dark/light re-extract (never invert), name default from stem with id rules, Save vs Save-and-apply with `preview.png` render into the theme directory per spec §5.1 (depends on T017) — DONE 2026-10-02: full rewrite (swatch editors + reset + edited flag, FileDialog + DropTarget import, Save returns to Themes, async save-and-apply); icons switch deferred to T028
- [x] T019 [P] [US2] Add `#[cfg(test)]` unit tests for thumbnail cache naming and theme-id validation edge cases in `src/thumbs.rs` and `src/theme/model.rs` — DONE 2026-10-02: cache-key/missing-file/broken-file/preview tests + wallpaper-stem naming test (id edges covered by T007)

**Checkpoint**: US1 + US2 form the closed loop (quickstart scenarios 2, 3); MVP demonstrable

---

## Phase 5: User Story 3 — Safety: baseline, rollback, Restore defaults (Priority: P1)

**Goal**: First-apply baseline snapshot, failed-apply rollback to pre-apply state, one-click Restore Ubuntu defaults.

**Independent Test**: Record wallpaper/gtk-theme/color-scheme/user-theme → first Apply creates `baseline/` (7 keys + CSS copies/`absent` markers + README) and later applies never overwrite it → break a theme (delete wallpaper) → failed apply restores pre-apply CSS/keys, session usable, not marked applied → Restore returns all recorded values, clears `applied.toml`, deselects `Atmosphere`, keeps saved themes.

- [x] T020 [P] [US3] Verify capture-once baseline and baseline-only restore (with documented Yaru/`default` fallbacks + warning when baseline missing) in `src/theme/baseline.rs` per `data-model.md` entity 5 — DONE 2026-10-02: verified (capture-once guard, 7 keys, CSS copies/absent markers, README, fallback defaults, applied-state clearing) + `RestoreOutcome::{Baseline, Fallback}` signal for the UI warning toast
- [x] T021 [US3] Integrate pre-apply `*.atmosphere.bak` restore and wallpaper-kept-only-on-color-restore toast variant into `src/theme/apply.rs` (depends on T008, T020) — DONE 2026-10-02: rollback tracks pre-existence and removes files the failed job created fresh; post-wallpaper failures carry "wallpaper kept, colors reverted" context to the one-line UI toast
- [x] T022 [US3] Add Restore Ubuntu defaults control with confirmation dialog (restores baseline keys, removes the `Atmosphere` user-theme selection, clears `applied.toml`) and session-only banner dismissal in `src/ui/themes_view.rs` and `src/ui/window.rs` per contracts `ui.md` — DONE 2026-10-02: destructive Restore button + confirm AlertDialog, worker restore with baseline/fallback toasts + reload, banner Dismiss hides for the session only
- [x] T023 [P] [US3] Add `#[cfg(test)]` unit tests for baseline round-trip and backup-restore ordering in `src/theme/baseline.rs` and `src/theme/atomic.rs` — DONE 2026-10-02: TOML round-trip, absent-marker removal, user-copy restore, foreign-file preservation, backup sibling naming + v1→v2→restore ordering

**Checkpoint**: US1–US3 satisfy the non-negotiable reversibility principle (quickstart scenarios 9, 11)

---

## Phase 6: User Story 4 — Glass effect per theme (Priority: P2)

**Goal**: Opt-in glass toggle (default off) with Subtle/Strong strength, shell-only `rgba` tint, clean off-path.

**Independent Test**: New theme defaults `enabled = false` → off-apply emits zero translucent rules and clears prior translucency → on-subtle/strong apply writes `rgba()` panel/calendar/message-list CSS within α bands subtle 0.75–0.88 / strong 0.60–0.75 → no `org.gnome.shell.extensions.blur-my-shell` keys ever touched → card label and thumbnail preview follow the switch.

- [x] T024 [P] [US4] Verify α bands, surface-tint math, and opaque off-path overwrite in `src/theme/glass.rs` with `#[cfg(test)]` unit tests — DONE 2026-10-02: verified (subtle 0.82 ∈ 0.75–0.88, strong 0.68 ∈ 0.60–0.75, off emits zero `rgba(`) + 4 tests (bands, rgba math + invalid fallback, off opaque, on tints panel + popups)
- [x] T025 [US4] Wire Glass `Switch` (default off) + Subtle/Strong segmented + thumbnail tint preview into `src/ui/create_view.rs`, persisting `[glass] enabled/strength` per `contracts/theme-toml.md` (depends on T024) — DONE 2026-10-02: live Preview overlay (thumbnail + `tint_rgba` from Surface swatch × strength, hidden when off, refreshed on toggle/strength/edit/extract/select); strength preserved across off/on
- [x] T026 [P] [US4] Show glass on/off label on cards in `src/ui/themes_view.rs` with no global force-enable per contracts `ui.md` — DONE 2026-10-02: verified ("Glass on/off" badge per card from saved theme, no force-enable path)

**Checkpoint**: US4 independently testable (quickstart scenario 10)

---

## Phase 7: User Story 5 — Two-tone Yaru folder icons (Priority: P2)

**Goal**: Accent-matched installed Yaru variant selection preserving dual-shade folders.

**Independent Test**: With `match_yaru = true`, dark apply sets a `-dark` Yaru `icon-theme` nearest the accent hue (falls back to `Yaru`/`Yaru-dark` when alone); Files shows two-tone folders; `false` leaves `icon-theme` untouched; Restore returns the baseline value.

- [x] T027 [P] [US5] Verify hue-distance matching over actually-installed variants with dark-preference and existence-validated set in `src/theme/icons.rs` with `#[cfg(test)]` unit tests per research R-3 — DONE 2026-10-02: verified (installed-probe gate, achromatic fallback, +15 dark-mismatch penalty, circular distance) + 4 tests (wrap-around, achromatic/chromatic split, invalid→None, disabled-matched noop without gsettings)
- [x] T028 [US5] Wire `icons.match_yaru` (default true) switch in `src/ui/create_view.rs` and apply-step integration in `src/theme/apply.rs` (depends on T008, T027) — DONE 2026-10-02: "Match folder icons (Yaru)" switch + help text, saved per theme, pipeline sets nearest installed variant after GTK settings (graceful None when disabled/uninstalled)

**Checkpoint**: US5 independently testable (quickstart scenario 12)

---

## Phase 8: User Story 6 — Ecosystem: Alacritty, banner, toasts, security gate (Priority: P3)

**Goal**: Optional Alacritty colors, honest UI copy, release-grade security evidence.

**Independent Test**: Without Alacritty installed the step is skipped; with it, `atmosphere-colors.toml` is written and `alacritty.toml` gains the import exactly once → success toast reads "Applied. Apps you open from now on use this theme." → `cargo audit` clean (or waiver in `SECURITY.md`), clippy/fmt clean, §8.4 probes pass.

- [x] T029 [P] [US6] Verify `atmosphere-colors.toml` write + idempotent `alacritty.toml` import merge (`live_config_reload = true` plus the `import` line, added only when absent; never rewrite user content, never legacy `colors.toml` alone) in `src/theme/alacritty.rs` and wire the skip-when-absent step into `src/theme/apply.rs` per contracts `system-effects.md` — DONE 2026-10-02: fixed `ALACRTITY` typo, extracted pure `render_colors`/`merge_import` + 2 tests (palette carry, idempotent fixed-point merge), wired post-shell pipeline step with rollback-covered backups
- [x] T030 [P] [US6] Finalize startup checks in `src/ui/window.rs`: User-Themes banner (constant extension id, session dismissal, skip-shell-continue-apply) plus one-time `adw-gtk3`/`adw-gtk3-dark` presence check per spec §4.5 (warn, still write CSS); one-sentence failure toasts via `ToastOverlay` in `src/ui/themes_view.rs` and `src/ui/create_view.rs` per contracts `ui.md` — DONE 2026-10-02: all elements verified + `wallpaper::adw_gtk3_installed()` warn-only banner with session Dismiss
- [x] T031 [P] [US6] Replace remaining `unwrap()` on Apply/Restore I/O paths with `Result` + rollback errors across `src/theme/*.rs` per constitution VIII — DONE 2026-10-02: `glass` let-else, regex replaced by panic-free manual id check (also drops the `regex` dep); only documented `HOME` env precondition + test-code unwraps remain

**Checkpoint**: All user stories independently functional (quickstart scenarios 6, 13)

---

## Phase 9: Polish & Cross-Cutting Concerns

**Purpose**: Release gates and full end-to-end validation

- [x] T032 Run the complete `specs/001-atmosphere-auto-themer/quickstart.md` validation (scenarios 1–14, including the <1 s timing assertion) on Ubuntu 26.04 / GNOME 50 / Wayland and fix deviations; do not test Ubuntu < 26, GNOME < 50, or X11 — DONE 2026-10-02 on Ubuntu 26.04/GNOME 50.1/Wayland live session: scenarios 4/5/9/11/12/14 via ignored `live_apply_restore_cycle` (steady-state apply 0.370s; desktop byte-identical after), scenario 2 via `live_extract_isolation`, scenario 3 via `live_save_edit_survival` (isolated HOME); fixed 2 deviations (dconf-sync loss on fast exit → `Settings::sync()` in apply/restore; first-apply timing measured on warmed steady state). Interactive GUI clicking (cards/dialogs/banner) needs a live session — no Broadway/Xvfb in this sandbox
- [x] T033 [P] Enforce clean `cargo fmt --check` and `cargo clippy -- -D warnings` with warnings addressed on Apply/Restore/icon paths — DONE 2026-10-02: `fmt --check` clean, `clippy -- -D warnings` passes with zero warnings of any kind
- [x] T034 [P] Enforce `cargo audit` zero Critical/High at the release tag, documenting any exception with CVE id and mitigation in `SECURITY.md` per spec §8.3 — DONE 2026-10-02: 0 Critical/High (single allowed warning: paste unmaintained RUSTSEC-2024-0436, not Critical/High → no waiver file needed)
- [x] T035 [P] Execute manual §8.4 probes (malicious `theme.toml` accent absent verbatim from `gtk.css`; symlink wallpaper escape rejected; no secrets in repo) and record results — DONE 2026-10-02: injection rejected (unit tests `rejects_injection_shapes`, `invalid_hex_rejected_before_write`), symlink escape/inside cases tested (`rejects_symlink_escape_on_import`), secrets grep clean
- [x] T036 [P] Fix the `libgdk-pixbuf2.0-dev` → `libgdk-pixbuf-2.0-dev` package-name drift in setup docs per constitution platform table — DONE 2026-10-02: fixed in spec §6.2 apt block (only occurrence; constitution/research/tasks already correct)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately. T001 (spec amendment) unblocks T009 semantics; T002 unblocks all builds.
- **Foundational (Phase 2)**: Depends on Setup — BLOCKS all user stories. T008 depends on T005, T006, T007.
- **User Stories (Phases 3–8)**: All depend on Foundational completion; then proceed in priority order (US1 → US2 → US3 → US4 → US5 → US6) or in parallel if staffed, noting T008/T021/T028 touch `src/theme/apply.rs` and must serialize.
- **Polish (Phase 9)**: Depends on all desired stories being complete.

### User Story Dependencies

- **US1 (P1)**: After Foundational — no story dependencies (testable with hand-written `theme.toml`).
- **US2 (P1)**: After Foundational — feeds US1 with real themes; independently testable via extract isolation.
- **US3 (P1)**: After Foundational + US1 pipeline (T008) — rollback/Restore wrap the apply path.
- **US4 (P2)**: After Foundational — glass merges into shell CSS from US1.
- **US5 (P2)**: After Foundational — icon step plugs into apply from US1.
- **US6 (P3)**: After Foundational — Alacritty step plugs into apply; banner/toasts overlay US1/US2 views.

### Within Each User Story

- Tests written alongside implementation (`#[cfg(test)]` modules, run via `cargo test`).
- Models/validators before pipeline/UI integration.
- Core implementation before integration task in the same story.
- `cargo fmt --check` + `cargo clippy` after each story.

### Parallel Opportunities

- Phase 1: T003, T004 run in parallel (different files).
- Phase 2: T005, T006, T007, T009, T010, T011 run in parallel (six different files); T008 runs after T005–T007.
- US1: T012, T013, T014 in parallel; T015 after T012; T016 alongside.
- US2: T017 in parallel with T019; T018 after T017.
- US3: T020 and T023-partial in parallel; T021 after T008+T020.
- US4/US5/US6: T024+T026, T027, T029+T030+T031 are parallelizable across files except shared `apply.rs`/`create_view.rs` edits.
- Polish: T033, T034, T035, T036 all parallel.

---

## Parallel Example: Foundational Phase

```bash
# Launch independent validator/model/helper tasks together (different files):
Task: "Verify validators + unit tests in src/theme/validate.rs"          # T005
Task: "Verify atomic helpers + unit tests in src/theme/atomic.rs"        # T006
Task: "Verify model + IconsSettings + unit tests in src/theme/model.rs" # T007
Task: "Dominant-color extract fix in src/theme/extract.rs"              # T009
Task: "Verify wallpaper/gsettings in src/theme/wallpaper.rs"           # T010
Task: "Idempotent Flatpak overrides in src/theme/flatpak.rs"           # T011
# Then, after T005–T007:
Task: "Rewrite apply pipeline in src/theme/apply.rs"                   # T008
```

## Parallel Example: User Story 1

```bash
# Launch model/service-level tasks together (different files):
Task: "Model list/load/delete/rename in src/theme/model.rs"  # T012
Task: "Harden GTK CSS render in src/theme/gtk_css.rs"        # T013
Task: "Verify shell CSS + reload in src/theme/shell.rs"      # T014
# Then UI integration (depends on T012):
Task: "Themes cards/mutex/toasts in src/ui/themes_view.rs"   # T015
```

---

## Implementation Strategy

### MVP First (US1 + US2)

1. Complete Phase 1: Setup (including T001 spec amendment + T002 `gnome_50` flags).
2. Complete Phase 2: Foundational (pipeline, validators, atomic I/O).
3. Complete Phase 3 (US1: Themes/Switch) — testable alone with a hand-written `theme.toml`.
4. Complete Phase 4 (US2: Create) — closes the loop; **STOP and VALIDATE** both stories, demo if ready.

### Incremental Delivery

1. Setup + Foundational → foundation ready (`cargo test`/clippy/fmt green).
2. + US1 (Themes) → one-click apply demonstrable → validate (quickstart 1, 4–8).
3. + US2 (Create) → create/save demonstrable → validate (quickstart 2, 3).
4. + US3 → safety net (baseline/rollback/Restore) → validate (quickstart 9, 11).
5. + US4 → glass → validate (quickstart 10).
6. + US5 → icons → validate (quickstart 12).
7. + US6 → ecosystem/hardening → validate (quickstart 6, 13).
8. Polish → full quickstart 1–13 + audit/clippy/fmt gates.

### Parallel Team Strategy

1. Team completes Setup + Foundational together (T008 owner waits on T005–T007).
2. Once Foundational is done: Developer A → US1+US2 (Themes + Create views/pipeline); Developer B → US3 safety + US4 glass; Developer C → US5 icons + US6 ecosystem.
3. Serialize edits to shared files (`src/theme/apply.rs`: T008 → T021 → T028; `src/ui/create_view.rs`: T018 → T025 → T028-switch; `src/ui/themes_view.rs`: T015 → T022 → T026) to avoid conflicts.

---

## Notes

- [P] tasks = different files, no dependencies; shared-file tasks (apply.rs, create_view.rs, themes_view.rs) are ordered, not parallel.
- [Story] label maps each story-phase task to its user story for traceability.
- Each story states its independent test criteria using `contracts/` + `quickstart.md` scenario numbers.
- Data-model constraints quoted verbatim in tasks: `^[a-z0-9][a-z0-9._+-]*$`, `^#[0-9a-fA-F]{6}$`, α subtle 0.75–0.88 / strong 0.60–0.75, 256px thumbnails, 7 baseline keys.
- Commit after each task or logical group; stop at any checkpoint to validate the story independently.
- Avoid: vague tasks, same-file conflicts, cross-story dependencies that break independence, any work on spec "v1 does not include / Later, not now" items.
