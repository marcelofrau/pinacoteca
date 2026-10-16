# M1 Progress
- Extracted setup, confirmed native x86 PEs (Picasa3.exe 10.2M, PicasaPhotoViewer.exe 4.8M). 
- REA Ghidra 12.1.4 healthy. Using temp copies for Windows bridge.
- Resources parsed (.fen, .ui, .yt, properties.xml). 
- Viewer UI commands identified (ss/fullscreen, ss/zoomtext, ss/filmstrip, ss/play_slideshow, 3dfilmstrip). 
- Target strings cataloged for maintenance (Rotate, Crop, RedEye, Enhance, AutoFix, Sharpen, Sepia, B&W, etc.).
- PMP/DB: no ".pmp" literal found; need schema from code references.
- Ghidra full symbol search slow on large PEs; will use string->xref + class names.

Next: PhotoViewer.exe first, collect string xrefs for Zoom/Pan/FullScreen/Filmstrip/Slideshow. Document behavioral specs.
