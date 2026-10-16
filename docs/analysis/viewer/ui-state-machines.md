# Viewer UI State Machines (from resources)

## Control bar actions (PhotoViewer)
- ss/play_slideshow -> start slideshow
- ss/fullscreen -> fullscreen toggle
- ss/to_web -> share to web (out of initial scope; note)
- ss/zoomtext, ss/zoomrect -> zoom controls/display
- ss/filmstrip -> toggle filmstrip
- layer:ss/3dfilmstrip -> 3D filmstrip variant

## Navigation
- ss/next, ss/prev (inferred from ss/next seen earlier)

## Interactions
- Sliders (HSlider, VertSlider) for zoom/scroll
- filmstripmove, 3dfilmstrip_clicked (mouse interactions)
