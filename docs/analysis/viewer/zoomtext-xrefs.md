# PhotoViewer: ss/zoomtext xrefs
String at 0x726b90 has xrefs to:
- 0x4251ed
- 0x42f11c
- 0x42f9b9
- 0x42fae7
- 0x42fd55

These are likely UI initialization/setup/handler sites (resource binding, event wiring). Next: inspect a few sites (read disasm around them) to infer zoom behavior (fit/actual, zoom in/out, slider). Also get xrefs for ss/fullscreen (0x726b40?), ss/filmstrip (0x726af8?), ss/play_slideshow.
