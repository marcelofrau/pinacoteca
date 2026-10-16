## M1 status (end of pass 1)
- Resource-driven + string-located: PhotoViewer zoomtext at 0x726b90 with xrefs; viewer UI commands identified.
- Draft specs created: viewer-spec-draft.md, albums-thumbnails-draft.md, maintenance-draft.md, pmp-notes.md.
- M1 checklist updated (structure done, resources done). Need deeper REA: trace xrefs to handlers, collect more string->xref for key actions (fullscreen, filmstrip, slideshow, rotate/crop/enhance).
- Plan next: PhotoViewer - open binary, trace xrefs from key strings to see handler names/structure; Picasa3 for maintenance/albums.

Overall M1 in progress. Draft specs sufficient for initial review; deeper behavioral extraction needed for clean-room impl.

## M1 status (end of pass 2 — PhotoViewer deep REA)
- Opened PicasaPhotoViewer.exe in Ghidra via REA. Confirmed PE identity: internal name GooglePhotoViewer.exe, comctl32 v6, PDB present but not admitted.
- RTTI haul: FilmstripHandler (FSHEntry, CacheIndex), C3DFilmstrip (+ClickHandler), SlideshowTransitionHandler, ZoomStatus@GrowHandler, ytArray wiring. Filmstrip enumerates directory at runtime (FindFirstFile pattern) -> navigation order = directory enumeration order.
- Recovered full viewer command set from dispatcher decompile + string xrefs: ss/next, ss/prev, ss/zoom_plus, ss/zoom_minus, ss/actualsize (zoom=1.0 animated), ss/rotate_cw, ss/rotate_ccw, ss/fullscreen, ss/maximize, ss/play_slideshow, ss/star (toggle + 0.6->1.0 anim), ss/launch_picasa (locate), ss/to_web, ss/config, ss/more, ss/exit, ss/close.
- UI events: filmstripmove, 3dfilmstrip_clicked, viewermousewheelzoom, slingshot_*, contextmenu, lb_*popup.
- viewer-spec-draft.md rewritten with command table + zoom/filmstrip models + pass-3 candidates (keyboard map, slideshow timing, pan drag).
- Pass 3 candidates logged; REA session closed.

M1 remains in progress; pass 3 (keyboard/slideshow timing) optional before M2 gate.
