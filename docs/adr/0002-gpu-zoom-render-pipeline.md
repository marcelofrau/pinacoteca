# 2. GPU rendering and zoom (lazy mip pyramid)

- **Status:** accepted
- **Date:** 2026-10-17
- **Milestone:** M2
- **Deciders:** Pinacoteca maintainers
- **Evidence:** [pyramid benchmark](../m2/pyramid-benchmark.md), [decoder benchmark](../m2/decoder-benchmark.md)

## Context

The viewer must keep zoom and pan smooth (target 60 fps) on large photos
(10+ megapixels) while holding a whole library. Two measured facts constrain the
design:

- A full JPEG decode is ~64–71 ms (ADR-0001).
- A full-resolution RGBA image is ~50 MiB; a full half-resolution pyramid is
  ~67 MiB (1.33×), and building it costs ~35 ms
  ([pyramid benchmark](../m2/pyramid-benchmark.md)).

Doing any of that work per zoom step, or keeping it for every photo, is
untenable. The strategy must make a zoom change O(1) at display time.

## Options considered

1. **Re-decode or resample the full image on every zoom change** — 35–70 ms per
   step; unusable for interactive zoom.
2. **Keep a full pyramid resident for every photo** — ~67 MiB each; cannot hold a
   library, and most of that is the rarely-needed full-resolution level.
3. **Lazy half-resolution pyramid for the currently displayed photo, with
   scale-based level selection** — build once per open (off the UI thread),
   switch level in O(1).
4. **Single full-resolution texture + GPU-generated mipmaps** — still requires
   uploading the ~50 MiB base texture per photo, and mip quality is fixed.

## Decision

- **Build a half-resolution pyramid (`pinacoteca-render::Pyramid`) lazily for the
  currently displayed photo only**, off the UI thread, at open time (~35 ms).
- **Select the level from the zoom scale**: at 100% or closer use level 0; below
  that use the largest level still not smaller than the displayed size, so GPU
  minification is at most ~2× and the result stays sharp. Level selection is pure
  arithmetic (`Pyramid::level_for_scale`) and unit-tested.
- **Upload only the levels the current view needs**, not the whole pyramid. The
  small levels are cheap; the full-resolution level is uploaded only near 100%.
- **The grid does not use the pyramid** — it uses the thumbnail cache (ADR-0003).
- **Rendering uses a wgpu texture with linear filtering** through eframe/egui's
  graphics backend (already in the dependency set).

## Consequences

- Zoom and pan become a level switch plus a texture sample — no CPU work per
  frame, which is what makes the 60 fps target reachable.
- We hold **one photo's pyramid (~67 MiB) at a time**, not one per library photo.
- Level 0 (~50 MiB upload) is still heavy for very large images. **Tiled upload of
  the full-resolution level is a documented follow-up**, not built in M2; it
  becomes relevant above ~24 MP or when several full-res views must coexist.
- The pyramid build (~35 ms) must stay on a worker thread; the viewer shows the
  best available level until the needed one is ready (this couples to the
  concurrency model, ADR-0005).
- `pinacoteca-render` stays graphics-free so the level arithmetic is testable
  headlessly; the GPU upload lives in `pinacoteca-app`.
