# 3. Thumbnail cache and background workers

- **Status:** accepted
- **Date:** 2026-10-17
- **Milestone:** M2
- **Deciders:** Pinacoteca maintainers
- **Evidence:** [decoder benchmark](../m2/decoder-benchmark.md)

## Context

The grid must show thousands of photos, and the budget is "time to first
thumbnails in a 10k folder < 1 s". Measured costs make plain regeneration
impossible:

- A full decode is ~64–71 ms per photo (ADR-0001).
- Reusing the embedded EXIF thumbnail costs ~0.001 ms to locate and extract, and
  is present on most JPEGs at a usable size (350×410 average) — but not all of
  them (some are as small as 38 px, and PNGs have none).

So thumbnails must be generated once and kept, the cheap path must be used
first, and the expensive path must never run on the UI thread.

## Options considered

1. **Regenerate on demand, no cache** — 70 ms per cell; a 10k folder is ~12
   minutes of CPU. Unusable.
2. **In-memory cache only** — fast while a session lasts, but re-decoding a
   large library on every launch misses the time-to-first-thumbnail budget.
3. **On-disk cache keyed by path** — survives restarts, but silently serves a
   stale thumbnail when a photo is re-exported over the same path.
4. **Content-keyed on-disk cache + priority deduplicating work queue.**

## Decision

Implemented as `pinacoteca-render::thumbs` (prototype; threaded pool lands in M3).

- **Content-keyed on-disk cache.** A `ThumbKey` is (source path, content
  fingerprint, size class). The fingerprint is the file's size + modification
  time — a cheap proxy that invalidates correctly when a photo is re-exported.
  Keys are hashed with a **stable FNV-1a**, not `std`'s `DefaultHasher`, which is
  not guaranteed stable across Rust versions (that would silently discard the
  whole cache on a toolchain upgrade).
- **Size classes.** `Small` 128, `Normal` 256, `Large` 512, `Preview` 1600 px
  longest side, each in its own subdirectory. A cell generates the smallest
  class that still covers it.
- **EXIF-thumbnail-first generation.** When a class is small enough for the
  embedded EXIF thumbnail (its size is checked first), use it: ~0.001 ms instead
  of ~70 ms. Otherwise decode the source once and downscale.
- **Deduplicating priority queue.** `RequestQueue` collapses repeated requests for
  the same key and keeps the best (lowest) priority, so a cell scrolled past and
  back, or hovered, is never queued twice. Priority is visibility-driven (cells
  nearest the viewport centre first), and `forget_missing` drops work for cells
  that have scrolled out of view.

## Consequences

- First run pays generation; every later open reuses the cache, which is what
  makes a warm folder re-open well under 300 ms.
- A re-exported photo is picked up automatically (fingerprint change), so the
  cache cannot go stale silently.
- The cache is derived data and fully regenerable — it can be deleted at any
  time, which keeps migration/back-up out of the way.
- The threaded pool, worker count and priority formula are M3 work; this
  milestone fixes the *cache shape* and the *scheduling rules* that the pool will
  honour.
- Grid tiles are drawn from `Normal` (256 px) cells; the lightbox uses `Large`.
