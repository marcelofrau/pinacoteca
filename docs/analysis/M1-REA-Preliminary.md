# M1 - REA Preliminary Analysis

## Scope
Extract viewer/albums/maintenance/thumbnails/PMP behavioral specs.

## Findings
- Native PEs. REA Ghidra 12.1.4 healthy. Using temp copies for ACL/bridge.
- PhotoViewer strings show fullscreen/zoom/filmstrip/slideshow.
- Picasa3 shows slideshow/editpanel/maintenance UI.
- UI in .fen/.ui/.yt; EXIF in properties.xml. No ".pmp" literal found.
- Need Ghidra symbol search (slow for Picasa3.exe).

## Next
- Focused Ghidra symbol search: View, Image, Thumb, Album, SlideShow, Rotate, Crop, Enhance, Effect, Filmstrip, Grid, RedEye, Sharpen.
- Extract .fen/.yt state machines.
- Catalog maintenance actions.
