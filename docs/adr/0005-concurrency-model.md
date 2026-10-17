# 5. Concurrency and threading model

- **Status:** accepted
- **Date:** 2026-10-17
- **Milestone:** M2
- **Deciders:** Pinacoteca maintainers
- **Evidence:** [decoder benchmark](../m2/decoder-benchmark.md), [pyramid benchmark](../m2/pyramid-benchmark.md)

## Context

The UI thread draws at 60 fps (≈16 ms per frame) and must never block. Measured
unit costs of the work it must *not* do:

| Work | Cost (this machine) |
|------|---------------------|
| Full JPEG decode | ~64–71 ms |
| Pyramid build (per photo) | ~35 ms |
| EXIF orientation parse | ~0.015 ms |
| Embedded thumbnail extract | ~0.001 ms |

Even a single decode on the UI thread would drop ~4 frames. A folder scan adds
filesystem latency. All of this must happen off the UI thread, with results
streaming back as they become available.

## Options considered

1. **Single-threaded** — cannot meet the frame budget; rejected by measurement.
2. **`async`/`await` with a runtime (e.g. `tokio`)** — pulls in an executor and
   colours the whole codebase; the work here is CPU-bound with a small,
   known number of long tasks, which a thread pool models directly.
3. **A thread per task** — unbounded thread creation on a 10k-photo folder.
4. **A small fixed set of OS threads plus channels**, with priority-aware queues.

## Decision

- **No async runtime.** Concurrency is explicit OS threads communicating over
  `std::sync::mpsc` channels.
- **Thread roles:**
  - **UI thread** — egui input/state and GPU submission only; it never decodes,
    scans or builds a pyramid.
  - **Scanner thread** — walks folders, reads metadata (size, mtime, EXIF
    dimensions/orientation), streams catalog entries to the UI.
  - **Decode pool** — full decode and pyramid build, sized from the available
    physical cores (`min(cores - 1, N)`; 12 physical cores on the reference
    machine).
  - **Thumbnail workers** — drain the priority `RequestQueue` (ADR-0003), preferring
    cells near the viewport centre and the cheap EXIF-thumbnail path.
- **Bounded channels** provide backpressure: a fast scanner cannot outrun the UI
  and grow an unbounded queue.
- **Cancellation by generation token.** Work carries the id/epoch of what it was
  requested for; when the user scrolls or opens another photo, results from a
  stale generation are discarded on arrival rather than cancelled in flight (a
  decode already running is short and cheaper to finish than to interrupt).
- **Priority over fairness** for thumbnails; **FIFO** for full-resolution decode
  (one photo at a time is what the viewer needs).

## Consequences

- No executor dependency; the model is straightforward to reason about and to
  test (the queues and scheduling rules are pure logic, unit-tested in
  `pinacoteca-render::thumbs`).
- Cancellation and backpressure are **explicit** responsibilities, not hidden in
  a runtime — the generation-token rule must be applied consistently in M3.
- Worker count is a tuning knob; the measured per-image costs make the expected
  throughput predictable (a 12-core machine decodes ~12 photos in the time one
  takes).
- The thumbnail pool and decode pool are separate so a burst of grid thumbnails
  cannot starve the full-resolution decode the viewer is waiting for.
