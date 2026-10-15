# Behavioral analysis

This directory holds the clean-room behavioral specifications produced in M1.

Rules (see [../cleanroom-protocol.md](../cleanroom-protocol.md)):

- Documents describe **observable behavior**: inputs, outputs, visible effects,
  formats, timings, edge cases.
- Documents do **not** contain decompiled or disassembled code, and do not
  describe internal implementation details of the reference application.
- Raw tool output does **not** belong here. It goes to the git-ignored
  `analysis-scratch/` directory.

## Planned documents

| Document | Contents |
|----------|----------|
| `viewer-behavior.md` | Grid, fullscreen viewer, zoom/pan, slideshow, keyboard map |
| `album-model.md` | Albums, tags, ratings, folder views and their semantics |
| `maintenance-operations.md` | Each edit operation: inputs, effect, parameters, ordering |
| `thumbnail-pipeline.md` | Observed thumbnail sizes, generation and caching behavior |
| `library-format-notes.md` | Neutral notes on the reference library/database format |

Status: not started (M1).
