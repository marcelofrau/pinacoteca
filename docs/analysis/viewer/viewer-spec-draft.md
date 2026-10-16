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

## Keyboard mapping (pass 3 — static investigation result: NOT statically enumerable)
Conclusion from a dedicated static pass: the viewer has no discoverable key table.
- No populated ACCELERATOR resource. The main module builds a *dummy* accelerator table (observed at main), so keys are not accelerators.
- Each `ss/*` command string has exactly one xref, from the command-dispatcher region (`FUN_00424e30`-family). There is no parallel string-driven key dispatch table.
- Window creation runs through the Fen framework wrapper: `FUN_004d23a0` is a thin `CreateWindowExW` wrapper (ANSI->wide marshalling + stack cookie). The `WNDCLASS.lpfnWndProc` is a Fen static dispatcher, so `WM_KEYDOWN` handling lives in Fen's window class, not in viewer-named code.
- Net: key bindings are a switch inside Fen's window-proc/event layer and cannot be cheaply enumerated statically.

Action: confirm bindings at runtime (see Runtime probe plan below). Working hypothesis to verify, mapping keys 1:1 to the recovered `ss/*` command set: arrows = `next`/`prev`; `+`/`-` = `zoom_plus`/`zoom_minus`; `1` or Enter = `actualsize`; Space = `play_slideshow`; `Esc` = `exit`; `F` = `fullscreen`; `R` = `rotate_cw`; `S` = `star`.

## Behaviors still to confirm (pass 3 candidates)
- Slideshow timing/interval defaults and pause/resume keys.
- Pan-when-zoomed drag behavior specifics.
- Generations buttons (ss/generations strings seen in Picasa3 side, viewer wiring unconfirmed).

## Runtime probe plan (for statically-unreachable behaviors)
Use `rea_capture_process_scenario` / `capture_native_ui_scenario` against `PicasaPhotoViewer.exe` with a fixture folder of images: inject single key presses and record resulting command dispatch (log the dispatcher entry), to resolve the keyboard hypothesis above. Repeat for slideshow interval measurement (timestamp between auto-advance) and pan drag (pointer move deltas while zoomed).

## UI framework & infrastructure (pass 3 string haul)
The viewer is built on the in-house Fen/yt UI framework rather than raw Win32:
- Window layer: Fen `Win32WindowImp`-style wrapper (`CreateWindowExW` wrapper `FUN_004d23a0`); `WM_KEYDOWN` handling therefore lives in the framework layer (see keyboard section).
- Panels: `Fen::TabPanel`, `Fen::MultiPanel`, `Fen::Win32TabPanelImp`, `ytPanel`, `ytBasePanel`, `ResCreate<ytPanel>` (resource-created panels).
- Selection/drag: `ytSelectionDragHandler`, `SelectionDragCreator`; resource events `selectiondrag`, `windrag`, `mousedown`.
- Transitions: `CTransitions` collection with `ITransitionNode`/`ytTransitionNode`; `UITransitions` resource group; `SlideshowTransitionHandler` drives slideshow advances through the same transition system.
- Sliders: `HSliderFastDrag`, `LabelHSlideFastDrag` (custom slider widget used by zoom/other controls).
- Wheel: `lb_controlmousewheel` (listbox/filmstrip), `viewermousewheelzoom` (photo area).

## Slideshow (pass 3)
- Advance is transition-based (`SlideshowTransitionHandler` + `ytTransitionNode`), so each step is an animated transition rather than a hard cut.
- No literal interval/frequency string exists, so the slideshow interval is a **setting** (config/DB-driven), not a compile-time constant. Default value not recoverable statically; note as a config item to define in Pinacoteca (recommend a sane default, e.g. 5 s, matching Picasa's UI default).

## Pan-when-zoomed (pass 3)
- No dedicated `viewerpan` command string; panning is a drag interaction. Candidate mechanism: `selectiondrag`/`windrag` + `mousedown` events routed to a drag handler while the photo is zoomed. Exact mapping not statically confirmed; fold into the runtime probe (drag while zoomed, measure canvas offset change).

## Clean-room notes
- Behavior derived from string inventories, xrefs, RTTI names, and decompiled control flow structure. No code copied; implementation must be independent.
