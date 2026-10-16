# Viewer Behavioral Spec (PhotoViewer.exe)

## Goal
Capture clean-room spec for image viewing in PhotoViewer: fullscreen toggle, zoom (fit/actual/in/out/wheel), pan (drag), filmstrip show/hide/3D, slideshow start/pause/exit, navigation (arrows/space/next/prev).

## Evidence
- Resources: slingshot/respack.yt defines ss/fullscreen, ss/zoomtext, ss/filmstrip, ss/play_slideshow, layer:ss/3dfilmstrip; filmstripmove, 3dfilmstrip_clicked.
- Strings: ss/zoomtext at 0x726b90 with xrefs (4251ed, 42f11c, 42f9b9, 42fae7, 42fd55) indicating UI wiring.
- Classes: HSlider, VertSlider, C3DFilmstrip (from strings).

## Behaviors to document
- Navigation: next/prev image (keyboard arrows, mouse buttons if present, UI buttons). 
- Zoom: fit to window, actual size (100%), zoom in/out, mouse wheel zoom; zoom text/rect display updates.
- Pan: drag to pan when zoomed in.
- Filmstrip: toggle visibility; 3D filmstrip variant; filmstrip interaction (click thumbnail -> navigate).
- Fullscreen: toggle, ESC to exit (typical).
- Slideshow: start from current image; transitions (BigSlideshow2 mentioned in Picasa3). Exit on key/mouse.

## Clean-room notes
- Derive behavior from resource names + xrefs + observed UI strings. Do not copy decompiled code. Focus on observable behavior/state transitions.
