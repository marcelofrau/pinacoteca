# M1 Completeness Review — Clean-Room Readiness (pass 8)

Final gate for M1: verify the behavioral specs are **sufficient for clean-room
reimplementation** of the project's initial scope — image viewing, album
organization, and photo maintenance — and record what is deliberately deferred.

## 1. Scope coverage

| Initial scope area | Spec | Verdict |
|--------------------|------|---------|
| Image viewing | `viewer/viewer-spec.md` | Complete (command set, filmstrip, zoom, slideshow, pan, framework); 4 items need runtime confirmation — see §3. |
| Album organization | `albums/albums-spec.md` | Complete (view modes, album model + actions, special albums, star/keywords, grid layout, thumbnail pipeline). |
| Photo maintenance | `maintenance/maintenance-spec.md` + `maintenance/geometry-spec.md` | Complete at the **catalog/interaction** level (84 filters, modes, sliders, controls, edit engine, geometry model). Per-filter pixel math is an M5 design task by design. |
| Data & metadata | `formats/pmp-notes.md` | Complete at the level required (import source identified + schema; `db3` container characterised). |

Out-of-scope areas (explicitly excluded from the project) are *noted but not
specified*: face recognition, import/export, share-to-web, Picnik/creative-kit
web tools, printer themes.

## 2. Clean-room compliance

- Specs describe **observable behavior, layout, and catalog surface only** — no
  decompiled/disassembled code and no Google source is reproduced.
- The Glimmer **effect recipes** in `filterdesc.xml` are Google's literal code and
  are kept **reference-only** (PROJECT_RULES #1204); only names/tooltips/slider
  ranges/concepts are documented.
- Raw tool output (string dumps, xref lists, early notes) lives in the git-ignored
  `analysis-scratch/`; superseded drafts are preserved in
  `analysis-scratch/superseded/`.
- No Picasa binaries or personal photo metadata are committed (`/binaries/` and
  `analysis-scratch/` are git-ignored).

## 3. Residual gaps (all non-blocking)

| Gap | Spec | Why open | Resolution |
|-----|------|----------|-----------|
| Keyboard bindings | viewer | Not statically enumerable (Fen window-proc switch) | Runtime probe in M3 (inject keys, log dispatcher) |
| Slideshow interval default | viewer | No compile-time constant; config/DB-driven | Define a sane default in Pinacoteca (≈5 s) |
| Pan-when-zoomed mapping | viewer | Drag interaction, no command string | Runtime probe in M3 |
| `rect64` bit packing (corner order, Q16/Q15, rounding) | geometry | Not pinned from shipped code | Verify against a sample `.picasa.ini` crop value (available locally) |
| `fr=` field meaning in geometry record | geometry | Ambiguous (frame vs ratio id) | Confirm from a real op sample |
| Per-filter parameter→pixel math | maintenance | Implementation-level study, not shipped recipes | M5 design |
| `real=` / `Variable=` → visible control mapping | maintenance | Recipe read | Study recipe *interface* only, M5 |
| `albums_0.db` payload encoding | formats | Reader RE blocked (Ghidra/rizin constraints) | Not on critical path; revisit if a one-shot migration is wanted |
| `.picasa.ini` forward-migration policy | albums | Design task | M4 design |

None of these block architecture (M2) or the initial implementation; all are
either runtime probes scheduled into M3 or design tasks scheduled into M4/M5.

## 4. Blocked / dead-end approaches (recorded so they do not recur)

- **`Picasa3.exe` full auto-analysis is not viable** with the REA Ghidra provider:
  each op re-imports ephemerally and the 10 MB binary exceeds the 330 s startup
  deadline. Workaround: use the REA **string/RTTI corpus** + **shipped resources**
  (`.fen`, `constants.ui`, `properties.xml`, `filterdesc.xml`, `.picasa.ini`) —
  this proved sufficient for the catalog-level specs.
- **rizin provider unavailable** on Windows native (`ERR_REA_WINDOWS_NATIVE`).
- **Viewer keyboard map** is not statically recoverable (Fen framework window
  proc); do not re-chase it statically — go to runtime.
- **`db3` is not SQLite** — do not attempt SQLite readers on `%LOCALAPPDATA%\
  Google\Picasa2\db3\*.db`.

## 5. Verdict

**M1 is complete.** The specs are sufficient to drive M2 (architecture spike) and
the M3/M4/M5 implementations for the initial scope. Runtime-only gaps do not block
design and are scheduled into M3; pixel-level filter math and `.picasa.ini`
forward-migration are intentionally deferred to M5/M4 respectively.

**Next milestone: M2 — architecture spike** (decoder benchmarks on real photos,
GPU render pipeline, ADRs, final architecture).
