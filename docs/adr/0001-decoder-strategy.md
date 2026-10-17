# 1. Image decoder strategy (pure-Rust default, native optional)

- **Status:** accepted
- **Date:** 2026-10-17
- **Milestone:** M2
- **Deciders:** Pinacoteca maintainers
- **Evidence:** [decoder benchmark](../m2/decoder-benchmark.md)

## Context

Pinacoteca must decode the common photo formats of its target era — JPEG above
all, plus PNG — fast enough to fill a thumbnail grid and open a full-screen
viewer on folders of thousands of photos. Two forces constrain the choice:

- **Distribution.** The application should be installable and buildable with a
  plain Rust toolchain, without requiring a C compiler, cmake or nasm on the
  end user's machine. Native image libraries pull exactly that in.
- **Performance.** JPEG files are large (≈10 MP common); a full decode costs
  real time and must not block the UI.

The `Decoder` trait in `pinacoteca-decoders` already isolates the choice, so the
decision is which implementations to ship by default.

## Options considered

1. **`zune-jpeg` (pure Rust, SIMD)** — 185 MP/s measured; no C toolchain.
2. **`jpeg-decoder` (pure Rust)** — 165 MP/s; the decoder the `image` crate uses
   internally for JPEG. Safe and widely used, but ~13% slower than `zune-jpeg`.
3. **`image` (facade over the above + `png`)** — convenient umbrella, but adds
   format negotiation and conversion overhead (~10% on PNG) and gives no JPEG
   speed advantage over its own underlying decoder.
4. **`mozjpeg` (native libjpeg-turbo)** — 218 MP/s; ~18% faster than the best
   pure-Rust option, but requires a C toolchain (cmake + MSVC/gcc + nasm) at
   build time.

## Decision

- **Default (always built): pure-Rust decoders.** JPEG via **`zune-jpeg`**, PNG
  via the **`png`** crate.
- **Native `mozjpeg` is an opt-in feature** (`native-mozjpeg`) for users who
  want the extra ~18% and can satisfy the C toolchain. It is never required and
  never enabled by default.
- **Do not use scaled IDCT as a thumbnail strategy.** Benchmarking showed only a
  1.2–1.3× gain; the thumbnail pipeline instead prefers the embedded EXIF
  thumbnail and falls back to a full decode (see ADR-0003).
- **EXIF parsing** uses a pure-Rust reader (`kamadak-exif`); orientation parsing
  measured at 0.015 ms/img.

## Consequences

- The default build has **zero native build dependencies** — a plain
  `cargo build` works anywhere Rust does.
- We accept **~18% slower JPEG decode** than libjpeg-turbo in the default
  configuration. For a 10 MP JPEG that is ~11 ms per image, recovered in
  practice by caching and parallelism.
- The native path stays available without restructuring: it is one `Decoder`
  implementation behind the existing trait, gated by a cargo feature.
- `image` is not used in the shipping decoder path, so its large dependency
  surface stays out of the product; it appears only in the benchmark harness.
- Revisit if end-to-end budgets (ADR-0002/0003) show JPEG decode as the
  bottleneck and the ~18% would clear a threshold — the trait makes that a
  contained change.
