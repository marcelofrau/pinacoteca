## M1 status
- Ghidra healthy; PEs copied to temp. String search working for viewer strings.
- Collected viewer UI action names from .yt/.fen and string literals.
- PhotoViewer.exe: ss/zoomtext located at 0x726b90. Need to trace xrefs from this string to handlers (zoom behavior).
- Next steps: open PhotoViewer.exe, enumerate xrefs to 0x726b90 and nearby action strings; document viewer behaviors.

Still pending: full viewer spec, albums/maintenance/thumbnails specs, PMP/DB schema.
