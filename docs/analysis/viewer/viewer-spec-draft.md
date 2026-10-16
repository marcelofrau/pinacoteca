# Viewer Behavioral Spec (PicasaPhotoViewer.exe)

## Goal
Clean-room behavioral spec for image viewing: zoom, navigation, filmstrip, slideshow, fullscreen, commands.

## Evidence (M1 pass 2, REA/Ghidra)
- Binary: PicasaPhotoViewer.exe (internal name GooglePhotoViewer.exe), x86 PE, PDB available but not admitted.
- Command dispatcher: single function resolves viewer commands by exact string match (event 0x8000004), then UI-event names (event 0x8000002). Recovered from decompile + string xrefs; addresses cited for traceability.
- Resource strings: slingshot/respack.yt `ss/*` keys; RTTI classes FilmstripHandler, C3DFilmstrip, SlideshowTransitionHandler, ZoomStatus@GrowHandler.

## Command set (ss/* strings, dispatcher-confirmed)
| Command | Observed behavior notes |
|---|---|
| `ss/next` / `ss/prev` | Navigate to next/previous image in current sequence. |
| `ss/zoom_plus` / `ss/zoom_minus` | Discrete zoom step in/out (relative step, not absolute). |
| `ss/actualsize` | Sets zoom factor to exactly 1.0 (100%); animated transition observed. |
| `ss/zoomtext` | Zoom indicator UI element (percent/rect display), wired in render path. |
| `ss/rotate_cw` / `ss/rotate_ccw` | Rotate current image 90° in viewer (display-side). |
| `ss/fullscreen` | Toggle fullscreen presentation. |
| `ss/maximize` | Maximize window (distinct from fullscreen). |
| `ss/play_slideshow` | Start slideshow from current image; transitions via SlideshowTransitionHandler. |
| `ss/star` | Star/rating toggle on current photo; UI animates star icon (scale 0.6→1.0 observed). |
| `ss/launch_picasa` | Open current photo context in main Picasa app ("locate" action attached). |
| `ss/to_web` | Share-to-web command (out of scope for Pinacoteca; noted for completeness). |
| `ss/config` / `ss/more` | Viewer settings dialog / overflow popup menu. |
| `ss/exit` / `ss/close` | Close viewer window. |

## UI event names (dispatcher phase 1)
- `filmstripmove`: filmstrip drag/move interaction on C3DFilmstrip.
- `3dfilmstrip_clicked` (resource): filmstrip thumbnail click -> navigate to that photo.
- `viewermousewheelzoom` (resource): mouse wheel zoom-in/out mapping.
- `slingshot_minimize` / `slingshot_maximize` / `slingshot_exit`: window-control events.
- `contextmenu`: right-click/context menu open on viewer.
- `lb_beginpopup` / `lb_hover` / `lb_endpopup` / `lb_cancelpopup`: listbox/popup lifecycle (filmstrip overlay menus).

## Filmstrip model
- Filmstrip enumerates sibling files in the photo's directory at open time (FindFirstFile pattern + FSHEntry array + CacheIndex class names). Navigation order = directory enumeration order.
- Two presentations: flat filmstrip and 3D filmstrip (`layer:ss/3dfilmstrip`, C3DFilmstrip class, filmstripmove/3dfilmstrip_clicked interactions).

## Zoom model (derived)
- Zoom is a continuous factor with discrete step commands (+1 step / -1 step), 1.0 = actual size, animated transitions between factors (transition handler observed in actualsize path).
- Wheel zoom mapped via `viewermousewheelzoom`.
- Zoom text element reflects current factor.

## Behaviors still to confirm (pass 3 candidates)
- Keyboard mapping (arrows/space/ESC) — not in command strings; likely low-level key handler, needs WM_KEYDOWN trace.
- Slideshow timing/interval defaults and pause/resume keys.
- Pan-when-zoomed drag behavior specifics.
- Generations buttons (ss/generations strings seen in Picasa3 side, viewer wiring unconfirmed).

## Clean-room notes
- Behavior derived from string inventories, xrefs, RTTI names, and decompiled control flow structure. No code copied; implementation must be independent.
