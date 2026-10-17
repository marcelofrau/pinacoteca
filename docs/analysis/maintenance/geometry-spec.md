# Geometry Sub-spec: Crop / Straighten / Rotate (M1)

Sources: `Picasa3.exe` string/RTTI corpus, `runtime/customaspectratio.fen`,
`runtime/bezier.txt`, `runtime/xhairs.png`, plus `filterdesc.xml` entries
(`crop`, `crop64`, `rot`, `tilt`). Clean-room: behavior only; schema/ops to be
re-derived independently.

## 1. Model

Three geometry edits, all **non-destructive** (recorded as edit-stack ops, the
original file is never rewritten):

| Op | filter id | Kind | Parameter |
|----|-----------|------|-----------|
| Crop | `crop`, `crop64` | rect selection | normalized rect (`rect64`) |
| Straighten | `tilt` (mode `tool`) | angle adjustment | rotation angle (+ optional auto-crop) |
| Rotate | `rot` | 90° step | clockwise / counter-clockwise |

`crop64` is the large-canvas crop variant; `crop` the normal one. Both write the
same serialized op shape.

## 2. Crop

### Serialized form

Crop is stored as a **normalized rectangle packed into a 64-bit value**:

```
crop=rect64(<64-bit hex>)
```

Seen alongside the complete geometry record:

```
width=%d,height=%d,crop=%s,fr=%s
```

where `crop` = the `rect64(...)` value and `fr` = the frame/aspect descriptor.
`rect64` = four 16-bit fixed-point (Q16) coordinates packed into one 64-bit
integer — the classic Picasa normalized-rect encoding. Coordinates are relative to
the image; the exact corner order and rounding must be **re-derived** from a real
op sample, not assumed.

Related literals: `crop=`, `/crop`, `crop=` (INI/XML keys), `lastCropRatio`
(remembered ratio), `DefaultCropSize`, `DefaultCropOrigin` (initial crop rect),
`radAngle`.

### Crop UI (`editpanel/*`)

- `editpanel/crop` — crop panel root
- `editpanel/cropselection` — the draggable selection
- `editpanel/croppreview` — live preview
- `editpanel/cropapply` / `editpanel/cropcancel` / `editpanel/cropdiscard`
- `editpanel/crop_aspect_menu` — aspect-ratio dropdown
- `editpanel/croprotatecrop` — rotate-then-crop affordance (align horizon, then
  crop keeps the rotation)
- `editpanel/crop_delete_custom` — remove a custom ratio
- `S` `IDS_CROP_LABEL`, `IDS_RECROP_LABEL` — button labels

Crosshair guides asset: `xhairs.png`. Curve guides: `bezier.txt` (control points,
used for a crop/straighten guide curve — format: `x y weight` per line, `#` = disabled).

### Aspect ratios

Class `CAspectRatioList`; item `SAspect@CAspectRatioList`; dialog
`CustomAspectRatioDlg` (`runtime/customaspectratio.fen`: width × height + name +
example). Persisted under `Preferences\AspectRatios`. StringTable keys
`AspectRatioList:<id>[:Description]`:

Standard presets: `4x6`, `5x7`, `5x8`, `8x10`, `9x13`, `10x15`, `Square`/`4x4`,
`16x9`/`HDTV16x9`/`Widescreen`, `16x10`, `5x3`/`WideFrame`, `FullPage`, `A4`
(`A4Page`), `Desktop4x3`, print variants `4x6m`, `9x13m`, `10x15m`,
`Crop13x18m`, `Crop20x25m`, `A4PageCollage`.
Meta entries: `CurrentAspect`, `CurrentRatio`, `CurrentDisplay`, `Manual`,
`Format`, `Other`, `CustomAspectRatios`, `AddCustomAspectRatio`.

### Smart crop (`CSmartCrop`)

Automatic crop-suggestion engine, surfaced as `editpanel/cropsug1..3` with
`editpanel/cropsug_preview%d`. Suggestion strategies evidenced by strings:

- "Crop by horizon line" — level the horizon
- "Crop by variance" — content-aware (detail density)
- "Crop by Red/Green" — colour-based subject detection
- "Close crop to faces" — face-aware (faces out of scope; concept noted)
- "Crop border" / "Crop to fit" (`crop_to_fit`, `croptofit`, `cropfit`)

Validation: "Invalid crop request".

## 3. Straighten (tilt)

- filter id `tilt`, mode `tool` — a direct-canvas angle tool (not a numeric filter).
- Angle control: `_sldrAngle`, `Angle`, `Angle: %d`; also `_sldrRotate`/`Rotate`.
- Uses `radAngle` (radians) internally; `ImageFilters::Angle` applies the rotation.
- Typical flow: user drags the horizon to level; the image is rotated by the
  resulting angle and **auto-cropped** to remove the resulting empty corners
  (hence `croprotatecrop` couples rotate + crop).
- Auto suggestion "Crop by horizon line" ties straighten to the crop engine.

## 4. Rotate

- Commands: `ID_PICTURE_ROTATECLOCKWISE`, `ID_PICTURE_ROTATECOUNTERCLOCKWISE`
  (also `OneUp::`, `AlbumPhoto::` variants) — 90° steps.
- Literals `rotate(-1)`, `rotate(0)`, `rotate(%d)` (step index),
  `Rotated 90/180/270 Degrees`, `EXIF::Rotate90/180/270`, `gphoto:rotate`,
  `ImgProf_rotation_angle`.
- EXIF orientation is honoured on load (see viewer spec); the rotate op records an
  additional 90° step on top of the decoded orientation.
- `IMAGE_rotate` / `imageOperations:RotateImageOperation` is the glimmer op.

## 5. Rendering ops (glimmer)

- `imageOperations:CropImageOperation`
- `imageOperations:RotateImageOperation`
- `ImageFilters::Angle`, `ImageFilters::Rotate`
- `CSmartCrop`, `CFrameGridTheme` etc. for contact-sheet frames.

## 6. Domain-model mapping (pinacoteca)

- `EditOp::Crop { rect: NormalizedRect }` — store rect as four normalized f32
  (0.0–1.0) rather than Q16; convert on `.picasa.ini` import/export.
- `EditOp::Straighten { angle_rad: f32 }` (+ implied auto-crop flag).
- `EditOp::Rotate { quarter_turns: i8 }` — cumulative 90° steps mod 4.
- All three compose in the `EditStack`; rendering applies them in order with the
  non-destructive contract (originals untouched).

## 7. Open questions / follow-ups

- Exact `rect64` bit packing (corner order, Q16 vs Q15, rounding) — verify against
  a sample `.picasa.ini` crop value.
- Meaning of `fr=` (frame id? ratio id?) in the geometry record.
- `bezier.txt` semantics (guide curve for which tool) — likely straighten/crop
  on-canvas guide; confirm at runtime.
- Straighten auto-crop policy (max angle before fallback, padding) — derive from
  behavior, not shipped code; matches `croprotatecrop`.
- Smart-crop suggestion ranking/scoring — leave unimplemented in pinacoteca M5
  unless cheap; document as optional.
