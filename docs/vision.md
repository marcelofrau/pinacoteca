# Vision

## One sentence

Pinacoteca is a fast, local, keyboard-driven photo viewer and organizer that
recreates the comfort of Picasa's browsing, album and light-editing workflow
without modifying the user's original files.

## The problem

Picasa was discontinued in 2016, yet for a large group of users it remains the
benchmark for local photo browsing:

- Thumbnails for a large folder appear almost immediately.
- Browsing, zooming and panning never feel like waiting.
- Albums are virtual: organizing photos never moves or renames files on disk.
- Basic edits are non-destructive and reversible, and "undo" always works.
- The application stays out of the way; the keyboard is enough to work quickly.

Modern alternatives tend to be either feature-heavy cloud-connected suites, or
minimal viewers that fall apart on large libraries. Few of them preserve the
"local, private, instant, keyboard-first" character that made Picasa comfortable.

## What we are building

A native desktop application that:

1. **Browses fast.** Opening a folder with thousands of photos is incremental:
   the grid appears immediately and fills in as thumbnails are produced. The
   viewer never blocks on decoding a large image.
2. **Feels immediate.** Rendering happens on the GPU. Zoom and pan are smooth
   regardless of the size of the source image.
3. **Organizes without touching disk.** Albums, tags and ratings live in a local
   database. Files are never moved, renamed or rewritten to represent organization.
4. **Edits non-destructively.** Every adjustment is recorded in an edit stack.
   The original file is opened read-only. Applying edits happens at render time.
5. **Stays private and offline.** No account, no network calls, no telemetry.

## Guiding principles

| Principle | Consequence |
|-----------|-------------|
| Performance is a feature | Perf budgets are defined and measured, not assumed. |
| The photo is the interface | UI chrome recedes; default is maximum image area. |
| Never mutate the user's data | Originals are read-only; state lives in a database. |
| Keyboard-first | Every primary action has a shortcut; mouse is optional. |
| No silent surprises | Failed operations surface visibly; nothing is swallowed. |
| Clean-room, always | Behavior is specified first, implemented second. |

## Anti-goals

Pinacoteca will not become:

- a cloud photo service or a social/sharing product;
- an importer for cameras and devices;
- a face-recognition or automatic-tagging system;
- an image editor for heavy, layered, destructive retouching.
