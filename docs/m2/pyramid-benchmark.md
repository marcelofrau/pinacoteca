# Pyramid (mip) benchmark (M2)

Evidence for [ADR-0002](../adr/0002-gpu-zoom-render-pipeline.md). Measured by the
`pinacoteca-bench` harness's `== mip pyramid build ==` section, using
`pinacoteca_render::Pyramid::from_base` (min side 64) over a `zune-jpeg` RGBA
base.

## Method

- For each JPEG sample: decode to RGBA (untimed), then time `Pyramid::build`.
- Accumulate level count, base bytes and total pyramid bytes.
- Same machine and sample set as the decoder benchmark (Ryzen 9 5900X, 35 real
  photos, 29 JPEG at ~10.5 MP average).

## Result

```
images: 29, avg levels 6.2, build 34.84 ms/img,
memory 50.1 MiB base -> 66.8 MiB pyramid (1.33x)
```

## Findings

1. **Building the full half-resolution pyramid costs ~35 ms/img** — about half of
   a full decode (≈64–71 ms). It is real work and must be done off the UI thread.
2. **Memory is 4/3 of the base**, as expected for a halving series (1 + 1/4 +
   1/16 + … → 4/3). For a 12 MP photo that is ~50 MiB base → ~67 MiB pyramid.
3. **Level 0 dominates memory** — ~48 of the ~67 MiB is the full-resolution base.
   The small levels (1/4, 1/8, …) are cheap.
4. **6.2 levels per image on average** (down to a 64 px minimum side) is plenty of
   range to cover fitted → 100% zoom without ever upscaling a texture.

## Implication

The pyramid cannot be built and kept resident for a whole library (thousands of
photos × 67 MiB). It is built **lazily for the currently displayed photo**, and
the full-resolution level is only needed near 100% zoom — which is also where a
tiled upload would matter for very large images. See ADR-0002.
