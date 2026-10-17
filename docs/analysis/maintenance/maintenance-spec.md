# Maintenance & Effects Spec (M1)

Source of truth: `runtime/filterdesc.xml` from the shipped Picasa 3 runtime tree.
Also cross-referenced with menu command IDs and RTTI class names recovered from the
`Picasa3.exe` ASCII string corpus (see `target-strings.txt`).

## Clean-room note

`filterdesc.xml` is a **shipped Google data asset**. This spec documents the
**catalog surface only**:

- effect/filter ids, display labels, tooltips;
- the interaction model (modes, slider labels, ranges, neutral offsets, defaults);
- conceptual behavior ("1960's = warm aged glow + rounded corners").

The actual **effect recipes** (the `<effect><cnt:EffectCanvas>…</cnt:EffectCanvas>`
Glimmer scripts, colour matrices, curve data, blend operations) are **reference-only**.
They are Google's literal code and MUST be re-derived independently, never copied into
this repository. See PROJECT_RULES #1204.

## Filter modes

Every filter entry declares a `mode` attribute describing its UI/interaction class:

| mode      | Meaning                                                              |
|-----------|---------------------------------------------------------------------|
| `history` | A recorded history step, not an interactive adjustment (Save, Crop, Rotate, Red Eye, Retouches). Appears in the edit history stack. |
| `soft`    | Multi-slider "soft" adjustment panel; controls several related parameters at once (Tuning, Lighting Fixes, Color Fixes). |
| `oneclick`| Single instantaneous action with no parameters (Auto Color, B&W, Sepia, Warmify, Film Grain, I'm Feeling Lucky). |
| `effect`  | Parameterised effect; may expose numeric sliders (`<sliders>`) and/or a Glimmer recipe (`<effect>`). |
| `tool`    | Interactive geometric/placement tool driven by direct canvas manipulation rather than sliders (Straighten, Rainbow). |

Optional attributes observed:

- `fullres="1"` — effect requires the full-resolution image (e.g. `unsharp2`, `glow2`).
- `slow="1"` — effect is computationally heavy; UI should expect latency (e.g. `glow2`, `unsharp2`).

## Zen/zero state (`zerostate`)

The neutral position of a filter's controls:

| zerostate  | Meaning                                                                 |
|------------|-------------------------------------------------------------------------|
| `zero`     | Neutral = slider at 0; moving away applies a delta.                     |
| `defaults` | Neutral = the declared `<default>` values (filter starts pre-applied).  |
| `none`     | No neutral concept (one-click / history / placement tools).             |

## Numeric sliders

Native (non-Glimmer) parameterised filters define sliders:

```xml
<slider id="0">
  <label>Amount</label>
  <range>2.0</range>     <!-- slider travel; with <offset> it maps to [offset, offset+range] -->
  <offset>1.0</offset>   <!-- neutral point inside the range (optional) -->
  <log>250.0</log>       <!-- logarithmic response base (optional) -->
  <default>0.1618</default>
</slider>
```

Semantics:

- `range` = total travel of the control. When `offset` is present the effective
  interval is `[offset, offset + range]` and the neutral value is `offset`
  (e.g. Saturation `range=2.0 offset=1.0` → 0.0…2.0 with 1.0 = no change).
- Without `offset`, the interval is `[0, range]` and 0 is the usual neutral.
- `log` = logarithmic interpolation base for perceptual controls (radius/intensity).
- `default` = value the control takes when the filter is opened or auto-applied.

### Slider catalog (native, parameterised filters)

| filter id    | mode   | label (UI)      | tooltip | sliders (label [range, offset, default]) |
|--------------|--------|-----------------|---------|-------------------------------------------|
| `unsharp`    | effect | Sharpen (Old)   | Sharpens edges | Amount [1.0, def 0.6] |
| `unsharp2`   | effect | Sharpen         | Sharpens edges | Amount [3.0, def 0.6] (fullres, slow) |
| `radblur`    | effect | Soft Focus      | Softens focus around a center point | Size [2.0, off 1.0]; Amount [2.0, off 1.0] |
| `radsat`     | effect | Focal B&W       | Desaturates around a center point | Size [2.0, off 1.0]; Sharpness [1.0, off 0.0] |
| `linblur`    | effect | Linear Blur     | — | Amount [10.0, def 2.0] |
| `tint`       | effect | Tint (Old)      | Makes a tinted look | Color Preservation [256.0, off 1.0] |
| `dir_tint`   | effect | Graduated Tint  | A graduated filter, useful for skies | Feather [1.0, def 0.25]; Shade [1.0, def 0.25] |
| `radtint`    | effect | Radial Tint     | Tints around a central point | Feather [1.0, def 0.25] |
| `glow`/`glow2`| effect | Glow           | Gives your photo a gauzy glow | Intensity [1.0, def 0.65]; Radius [1.0, log 250, def 3.0] |
| `sat`        | effect | Saturation      | Increases or decreases saturation | Amount [2.0, off 1.0, def 0.1618] |
| `colortemp`  | effect | Color Temperature | — | Cool to Warm [1.0, off 0.5, def 0.125]; White Shift [1.0] |
| `shadow`     | effect | Shadow & Highlight | — | Radius [1.0]; Shadow % [1.0]; Highlight % [1.0] |
| `blur`       | effect | Blur            | Softens your photo | Threshold [1.0, off 0.5, def 0.1] |
| `contrast`   | effect | Contrast        | — | Contrast [1.0, off 0.5, def 0.1] |
| `gamma`      | effect | Gamma Correct   | — | Level [2.0, off 1.0, def 0.1618] |
| `backlight`  | effect | Backlight Fix   | — | Amount [1.0, def 0.25] |
| `dir_sat`    | effect | Directional Saturation | — | Left to Right [2.0, off 1.0]; Top to Bottom [2.0, off 1.0] |
| `dir_brite`  | effect | Directional Brightness | — | Left to Right [2.0, off 1.0]; Top to Bottom [2.0, off 1.0] |
| `dir_sharp`  | effect | Directional Sharpen | — | Left to Right [2.0, off 1.0]; Top to Bottom [2.0, off 1.0] |
| `focalpixelate`| effect | Focal Pixelate | — | Pixel Size [100.0, def 15.0]; Focal Size [2.0, def 1.0]; Edge Hardness [0.95, def 0.25]; Fade [1.0, def 0.0] |
| `debug`      | effect | Debug           | For debugging | Size [100.0] |

### Soft-mode multi-control panels

These bundle several related adjustments behind one panel with a shared zerostate:

| filter id  | label (UI)      | zerostate | controls |
|------------|-----------------|-----------|----------|
| `triple`   | Lighting Fixes  | zero      | Brightness [2.0, off 1.0]; Contrast [1.0, off 0.5]; Fill Light [1.0] |
| `triple2`  | Lighting Fixes  | defaults  | Fill Light [1.0]; Black Point [1.0]; White Point [1.0, def 1.0] |
| `triple3`  | Lighting Fixes  | zero      | Fill Light [1.0]; Highlights [0.48]; Shadows [0.48] |
| `finetune` | Tuning          | zero      | Fill Light [1.0]; Highlights [0.48]; Shadows [0.48]; Color Temperature [1.0, off 0.5] |
| `finetune2`| Tuning          | zero      | Fill Light [1.0]; Highlights [0.48]; Shadows [0.48]; Color Temperature [2.0, off 1.0] (Kelvin-style range) |
| `colorfix` | Color Fixes     | zero      | Choose White Point [color picker]; Color Temperature [1.0, off 0.5] |
| `fill`     | Fill Light      | zero      | (no sliders declared) |

> `finetune` vs `finetune2`: `finetune2` widens Color Temperature to `[1.0, 3.0]`
> (offset 1.0 = neutral Kelvin), matching a newer Kelvin white-balance model.

### Non-slider control types

Some filters declare interactive controls beyond sliders:

| control attribute        | filter(s)                             | Meaning |
|--------------------------|---------------------------------------|---------|
| `colorcircle=0`          | `finetune`, `finetune2`, `colorfix`, `whitept` | On-canvas colour wheel/selector |
| `HSBColorPicker=_cpkr…`  | `Sixties`, `TwoTone`                  | HSB colour picker widget |
| `ImageColorSwatch=_clrsw`| `Matte`, `Neon`, `PicnikTint`, `Vignette` | Solid colour swatch widget |
| `BrushSizeAndEraserButton=_brshbtn` | `ReanimatedEyeColor`      | Brush size + eraser toggle |
| `real=N`                 | many `effect` filters                 | Numeric real parameter slot N (exposed when the Glimmer recipe reads it) |
| `Variable=<name>`        | `Cinemascope`, `Polaroid`, `PicnikGrain`, `HeatMap` | Named recipe variable |
| `Variable=_cxyBrush`     | `ReanimatedEyeColor`                  | Brush centre coordinate |

## One-click actions (no parameters)

Applied instantly, single history step:

| filter id      | label (UI)         | tooltip |
|----------------|--------------------|---------|
| `autobacklight`| Fill Light         | — |
| `autolight`    | Auto Contrast      | — |
| `autocontrast` | Auto Contrast      | — |
| `autocolor`    | Auto Color         | — |
| `enhance`      | I'm Feeling Lucky  | — |
| `warm`         | Warmify            | Improves skintones by boosting warm tones |
| `bw`           | B&W                | Makes your photo black and white |
| `sepia`        | Sepia              | Converts photo to sepia tone |
| `grain`        | Film Grain (Old)   | Adds film grain |
| `grain2`       | Film Grain         | Adds film grain |

## History / geometry / edit steps

| filter id | label (UI)  | note |
|-----------|-------------|------|
| `save`    | Save        | history step |
| `crop`    | Crop        | geometry; see Crop editor |
| `crop64`  | Crop        | geometry; large-canvas crop variant |
| `rot`     | Rotate      | geometry; see rotate CW/CCW menu commands |
| `redeye`  | Red Eye     | tool step (plugin `red.cfg`/`Red.dll`); see RedEyeEdit |
| `retouch` | Retouches   | patch/retouch tool step; see RetouchEdit, PatchEdit |
| `picnik`  | Creative Kit | external web tool entry |
| `tilt`    | Straighten  | tool-mode straighten (direct canvas manipulation) |
| `rainbow` | Rainbow     | tool-mode placement tool |

## Creative effects (Glimmer) — catalog only

These filters are **entirely recipe-driven** (no native `<sliders>`); their UI
controls come from widgets declared inside the Glimmer script
(`HSliderFastDrag`, `HSliderPlus`, `HSBColorPicker`, …). Recipe bodies are
reference-only. Conceptual behavior from labels/tooltips:

| filter id              | label (UI)     | tooltip / concept |
|------------------------|----------------|-------------------|
| `Boost`                | Boost          | Bring out colors and increase contrast |
| `Border`               | Border         | Add a frame to your photo |
| `Cinemascope`          | Cinemascope    | Classic movie letterbox bars (crop + borders) |
| `Comicize`             | Comic Book     | Comic book style half-toning |
| `CrossProcess`         | Cross Process  | Mimics film cross-processing |
| `DropShadow`           | Drop Shadow    | Photo appears floating above the background |
| `PicnikFocalPixelate`  | Focal Pixelate | Pixelate inside/outside a central area |
| `FocalZoom`            | Focal Zoom     | Zoom everything outside a central area |
| `PicnikGrain`          | Film Grain     | Simulate film grain |
| `HDR`                  | HDR-ish        | Emulate high dynamic range look |
| `HeatMap`              | Heat Map       | Simulate heat vision |
| `Holga`                | Holga-ish      | Plastic-camera look |
| `Invert`               | Invert Colors  | Negative |
| `IR`                   | Infrared Film  | B&W infrared film look |
| `LocalContrast`        | Local Contrast | Brings out image details |
| `Lomo`                 | Lomo-ish       | Lomo toy-camera look |
| `Matte`                | Matte          | Light glow at the edges |
| `MuseumMatte`          | Museum Matte   | Shadowed matte frame |
| `Neon`                 | Neon           | Neon look |
| `NightVision`          | Night Vision   | Infrared night-vision cameras |
| `Orton`                | Orton-ish      | Michael Orton effect |
| `PencilSketch`         | Pencil Sketch  | Drawn-with-pencil look |
| `Pixelate`             | Pixelate       | Blocky, low-res look |
| `Polaroid`             | Polaroid       | Instant-film look |
| `QuantizePalette`      | Posterize      | Reduce number of colours |
| `ReanimatedEyeColor`   | Ghoul Eye      | Icky glowing eyes (brush) |
| `RoundedEdges`         | Rounded Edges  | Rounded corners |
| `Sixties`              | 1960's         | Rounded corners + warm aged glow |
| `Soften`               | Soften         | Soft, glowy |
| `PicnikTint`           | Tint           | Change photo colour |
| `TwoTone`              | Duo-Tone       | Two-colour conversion |
| `Vignette`             | Vignette       | Darken the edges |

## Edit engine architecture (from RTTI)

- `IImageFilter` / `IImageFilterFactory` / `CImageFilterFactory` / `CImageFilterRegistrar` — filter registration + construction.
- `CBaseFilter`, `CGenericFilter`, `CSimpleFilter`, `CTransformFilter` — filter base classes.
- `CFilterStack` / `CFilterStackUI` / `EditStack` / `FilterUndoRedoShow` — non-destructive edit stack with undo/redo.
- `glimmer::CropImageOperation`, `SharpenImageOperation`, `BorderImageOperation`, `ColorMatrixImageOperation`, `SimpleColorMatrixImageOperation`, `MultiplyColorMatrixImageOperation` — image-operation primitives.
- `EffectParser@glimmer`, `ytEffectCreator`, `ytEffectNode` + effect-VM instructions (`ApplyInstruction`, `BlendInstruction`, `ClearVar`, `Dupe`, `GetVar`, `Mask`, `MaskWithSourceAlpha`, `NamedVar`, `Op`, `PartialMask`, `Pop`, `ReExecuting`, `SetVar`, `SetVarInstruction`) — data-driven effect-definition language.
- `RedEyeEdit`, `RetouchEdit`, `PatchEdit`, `CSmartCrop`, `CollagePanel`/`CCollageUI`/`ICOLLAGETHEME`, `CPicnikNode` — specialised edit modes.

Effects look data-driven: keys `fx%d`, `fx_%s`, `editpanel/fx_%s`, `fxthumbs`
index a filter/effect table, consistent with `filterdesc.xml` being that table.

## Menu command IDs (recovered)

`ID_PICTURE_AUTO_COLOR`, `ID_PICTURE_AUTO_LIGHTING`, `ID_PICTURE_AUTO_REDEYE`,
`ID_PICTURE_ENHANCE`, `ID_PICTURE_FILM_GRAIN`, `ID_PICTURE_RESET_FACES`,
`ID_PICTURE_REVERT`, `ID_PICTURE_ROTATECLOCKWISE`,
`ID_PICTURE_ROTATECOUNTERCLOCKWISE`, `ID_PICTURE_SHARPEN`, `ID_PICTURE_WARMIFY`.

## Mapping to our domain model

- Each catalog entry maps to an `EditOp` variant; parameterised filters carry
  their slider values, creative effects carry opaque parameter maps.
- The history/geometry steps (`save`, `crop`, `rot`, `redeye`, `retouch`) are the
  stack steps that record into `EditStack`; `soft`/`effect`/`oneclick` are the
  adjustments composited on top.
- Non-destructive: originals never mutated; `EditStack` serialises the applied
  ops so any change is revertable (matches `FilterUndoRedoShow`).

## Open questions / follow-ups

- Exact parameter→pixel math for each native slider (needs implementation-level
  study, not shipped recipes) — deferred to M5 design.
- Which `real=`/`Variable=` slots correspond to which visible control in the
  Glimmer effects — recover by studying the recipe *interface* only, not the body.
- Crop/Straighten/Rotate canvas coordinate model — document separately under
  `maintenance/` (geometry sub-spec).
- Red-eye detection data lives in `plugins/red.cfg` (2.2 MB) + `Red.dll` — separate
  study if red-eye enters scope.
