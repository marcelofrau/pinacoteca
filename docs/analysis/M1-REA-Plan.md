# M1 Analysis Plan (full REA)

## Approach
String-driven + resource-driven (avoid full procedure enum on large PEs).

## Targets
- PicasaPhotoViewer.exe: viewer focus (filmstrip, zoom/pan, fullscreen, slideshow)
- Picasa3.exe: albums, maintenance (rotate/crop/enhance/effects), thumbnails/grid
- Resources: runtime/*.fen, *.yt, *.ui, properties.xml

## Actions
1. Parse .fen/.yt: extract actions/commands (ss/fullscreen, ss/zoomtext, ss/play_slideshow, filmstrip). Write docs/analysis/viewer/ui-state-machines.md.
2. String extraction: key terms -> procedures via xrefs (string->xref). Focus set: viewer (Zoom, Pan, FullScreen, Filmstrip, Slideshow); maintenance (Rotate, Crop, RedEye, Enhance, AutoFix, Sharpen, Sepia, BlackWhite); albums (Album, Group); thumbnails (Thumbnail, Thumb). 
3. PhotoViewer first (smaller), document viewer behaviors.
4. Picasa3 second, document albums/maintenance/thumbnails.
5. Formats: scan for PMP/DB/index (pmp, preview, thumb, db). Document in formats.md.

## Output
Create spec files per domain with: behavioral steps, inputs/outputs, states, keyboard/mouse, observations, clean-room notes.
