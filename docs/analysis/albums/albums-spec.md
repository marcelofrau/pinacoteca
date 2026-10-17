# Albums, Grid & Thumbnails Spec (M1)

Sources: `runtime/album.fen` (album metadata dialog), `runtime/constants.ui`
(album list / label / layout constants), and the `Picasa3.exe` ASCII string +
RTTI corpus (`p3-strings-ascii.txt`). Supersedes `albums-thumbnails-draft.md`.

Clean-room: describes **behavior and layout**, not code. Schema fields listed here
are to be **independently re-derived**; no Google implementation is copied.

## 1. Library view model (view modes)

Recovered `ID_VIEW_*` command set (menu: View). The library is a set of switchable
views over the same photo store:

| Command id                     | View |
|--------------------------------|------|
| `ID_VIEW_TIMELINE`             | Timeline (photos by date) |
| `ID_VIEW_FOLDERS`              | Folder tree / watched folders |
| `ID_VIEW_ALL`                  | All photos |
| `ID_VIEW_WATCHED`              | Watched folders only |
| `ID_VIEW_DESKTOP` / `ID_VIEW_MYPICTURES` / `ID_VIEW_MYDOCS` / `ID_VIEW_MYCOMPUTER` | Quick source roots |
| `ID_VIEW_PEOPLE`               | People (face albums) — out of M1 scope for detection, model noted |
| `ID_VIEW_PLACES`               | Places (geo) |
| `ID_VIEW_KEYWORDS`             | Keyword/tag view |
| `ID_VIEW_SEARCHVIEW`           | Search results |
| `ID_VIEW_LIGHTBOXVIEW`         | Lightbox (focused single-photo grid) |
| `ID_VIEW_THUMBNAILS`           | Thumbnail grid |
| `ID_VIEW_SMALLTHUMBNAILS`      | Small grid |
| `ID_VIEW_LARGETHUMBNAILS`      | Large grid |
| `ID_VIEW_PICTURE`              | Single picture view |
| `ID_VIEW_PROPERTIES`           | Properties/EXIF panel |
| `ID_VIEW_SLIDESHOW`            | Slideshow |
| `ID_VIEW_EDIT`                 | Edit mode |
| `ID_VIEW_COLOR_MANAGED`        | Colour-managed display toggle |
| `ID_VIEW_SHOWHIDDEN`           | Show hidden photos toggle |

Display-profile toggles (monitor calibration profiles):
`ID_VIEW_AUTO`, `ID_VIEW_NORMAL`, `ID_VIEW_16`, `ID_VIEW_RDESK` (Remote Desktop),
`ID_VIEW_LCD`, `ID_VIEW_PROJECTOR`, `ID_VIEW_OV`, `ID_VIEW_MAC`, `ID_VIEW_LINEAR`,
`ID_VIEW_SEPIA`, `ID_VIEW_BW`.

Wired by `eMenuView` / `eMenuViewWin` / `eMenuTools::ID_VIEW_EARTH`.

## 2. Album list (left sidebar)

Constants from `constants.ui [Picasa2]`:

- `alist_height=22`, `alist_fontsize_win=14`, `alist_indent=17`
- `alist_bgcolor=0xFFF3F3F3`
- highlight `alist_hicolor_win=0xFF83A7BD`, secondary `alist_hicolor2_win=0xFFE5E2DA`,
  selection `alist_selcolor_win=0xFF25648B`
- drag `alist_dragcolor=0xFF82A6BD`
- category `alist_catcolor=0xFFEDEAE4`, selected-category `alist_scatcolor=0xFF25648B`
- dot `alist_dotcolor=0xFFBEBEBE`, sticky `alist_stickycolor=0xFFEAE7DC`
- Mac variants (Aqua blue) supplied separately.

Album-list labels (`alabel_*`):

- folder bg `#e2e2e2`, album bg `#e2e2e2`, hover `#f2f2f2`
- button font `Praxis Semi Bold/Heavy` size 12
- light text `#FFFFFF`, dark text `#000000`
- icon offset x12/y8, subhead size 20
- action-offset markers: `webpubOffset=-16`, `burncdOffset=-224`

UI classes (RTTI): `CAlbumList`, `AlbumList@KeyHandler`, `AlbumScroll@KeyHandler`,
`CMultiAlbumNode`, `CAlbumSelectionNode`, `CAlbumLabel`, `CAlbumState`/`IAlbumState`,
`IAlbumStore`, `IAlbumSource`, `IAlbumPersistedCallback`.

## 3. Album model & actions

Album = a virtual collection over photos (a photo may belong to many). Persisted
via `IAlbumStore`; the album list is a tree of `CAlbumSelectionNode`s.

Album context commands recovered (`eMenuLabelFolder`, `Album`, `Folder`, `PplAlbum`):

| Command | Meaning |
|---------|---------|
| `ID_ALBUM_NEW` | Create album |
| `ID_ALBUM_EDITCAPTIONS` | Edit captions |
| `ID_ALBUM_DELETE` | Delete album (virtual; photos kept) |
| `ID_ALBUM_LOCATEONDISK` | Reveal backing folder on disk |
| `ID_ALBUM_SLIDESHOW` | Play slideshow from album |
| `ID_ALBUM_SELECTALLPICTURES` | Select all photos in album |
| `ID_ALBUM_MOVETOCOLLECTION` / `ID_ALBUM_MOVE_TO_NEW_COLLECTION` | Move to collection |
| `ID_ALBUM_COPYTOCOLLECTION` | Copy to collection |
| `ID_ALBUM_COMBINEWITHNEXT` / `ID_ALBUM_COMBINEWITHPREVIOUS` | Merge albums |
| `ID_ALBUM_MAKE_WEB` | Publish album to web |
| `ID_ALBUM_PRINTACONTACTSHEET` | Print contact sheet |
| `ID_ALBUM_REFRESHTHUMBNAILS` | Rebuild thumbnails |
| `ID_ALBUM_REMOVE` | Remove from album |
| `ID_ALBUM_FILTERFACES` | People filter (scope-excluded) |
| `ID_ALBUM_SENDTO_*` | Send to albums / CD project / Picasa Web / new collection |
| `ID_LABELS` / `ID_LABELSNEW` | Label management |

Album metadata dialog (`album.fen`) fields:

- **Name** (edit, `filter="filename"`)
- **Date** (date field + **Automatic date** button)
- **Music** (checkbox "Use music for Slideshow and Movie presentation" + Browse
  for `*.mp3;*.wma` on Windows)
- **Place taken** (optional text)
- **Description** (optional, 3-line multiline)
- OK / Cancel buttons

Persisted album XML classes: `AlbumXML`, `LHAlbumXML`/`LHAlbumEntryXML`, `AlbumParse`,
`CAlbumVarString`, `CAlbumVarDateConverter`, `CAlbumMultiConverter`.

## 4. Special (virtual) albums

From `CThumbDB::*` static names and related strings:

- `Starred Photos` (`CThumbDB::starredphotos`) — cannot be deleted.
- `Recent Updates` (`CThumbDB::RecentUpdate`)
- `Emailed` (`CThumbDB::Emailed`), `Exported` (`CThumbDB::Exported`)
- `Screensaver` album (`CThumbDB::ScreensaverAlbumName`)
- `UnknownPeople` / `IgnoredPeople` face albums (`unknownfacealbum`,
  `ignorefacealbum`, `UnknownPerson`, `NeedFaceUpdate`) — faces out of scope,
  model noted only.

## 5. Star / rating model

- `CThumbUI::sstarred`, `CStarredEnabler`, `CThumbDB::starredphotos`,
  `StarredPhotosTotal`.
- UI icons: `icons/star_on_icon`, `icons/star_off_icon`, `editpanel/star_icon`,
  `select_star`, menu `ID_SELECTSTAR`, "Select &Starred".
- EXIF/XMP backing: `XMP::Rating`, `EXIF::ISOSpeedRatings` (ISO surfaced adjacent).
- Selection: `ID_SELECTSTAR` selects all starred; per-photo star toggles rating.

## 6. Keywords / tags

- Classes: `KeywordJob` (async), `IKeywordUpdate`, `SyncLabelUpdater`, `ID_LABELS`.
- View mode `ID_VIEW_KEYWORDS`.
- Keywords stored as labels; sync updater propagates label changes.

## 7. Thumbnail grid & layout

Layout constants (`constants.ui` `[Picasa2]`):

- `alayout_gutter=24`, `alayout_thumbGutterX=12`, `alayout_thumbGutterY=22`
- title font `Georgia` size 20, colour `#634B45`, title offset X `28`
- body font `Georgia` size 14, body offset `0`
- selected-thumbnail outline: `color1=#009EFF` (outside), `color2=#FFFFFF` (inside)

Thumbnail size modes: Normal / Small / Large (`ID_VIEW_SMALLTHUMBNAILS`,
`ID_VIEW_LARGETHUMBNAILS`, `ID_VIEW_THUMBNAILS`) with `option_thumbsize`,
`ThumbnailSize`, `option_thumbsize` stored in prefs. `TileWidth`/`tileWidth` govern
grid tile metrics; `editpanel/gridtilecont` is the grid tile container.

Grid/layout engine classes: `picturegrid`, `regulargrid`, `framegrid`,
`FilterGridItemLoaderJob`, and printer-helper grid themes
(`CGridTheme`, `CRegularGridTheme`, `CFrameGridTheme`). `thumbui/albumview`,
`thumbui/viewswitch`, `thumbui/scalecontainer`, `thumbui/right_drawer`,
`thumbui/toggle_right_drawer` are the grid-side panel/widget keys.

## 8. Thumbnail pipeline

- Interfaces: `IThumbDB`, `CThumbDB` (photo+thumb DB), `IThumbnailSource`,
  `IThumbnailBase`, `CThumbnailProvider`/`IThumbnailProvider`.
- Timeline: `CTimelineThumbMaker`, `CTransTimeline`, `ThumbCache@CTransTimeline`.
- Progress UI: "Updating thumbnails...", `thumbui/modalprogressfloater`,
  `thumbui/progressbarfill`, `RefreshThumbnails` command.
- Option **"Use EXIF thumbnails"** — reuse the embedded EXIF thumbnail when present
  and large enough; otherwise decode the full image and downscale.
- Size diagnostics string: `Thumb size:  %4d x %d`; integrity check
  `Warning: thumbnail image size does not match data length %u`.

### Persisted stores (file names recovered)

| File | Purpose |
|------|---------|
| `thumbs.db`, `thumbs2.db` | primary thumbnail caches |
| `bigthumbs.db` | large thumbnails |
| `previews.db` | screen-size previews |
| `index-thumbs.db`, `thumbs_index.db`, `thumbindex.db` | thumbnail indices |
| `albums.db` | virtual album definitions |
| `profilephotos.db` | profile photos (people) |
| `facetemplatesV2.db`, `facetemplates_0.db`, `facetemplates_index.db` | face templates (scope-excluded) |
| `distims.db` | distance/hash index (dedup/duplicate detection) |
| `.picasa.ini` / `Picasa.ini` | per-folder metadata (captions, stars, album membership) |

Web-export URL conventions seen in the HTML templates (for the future web-export
surface, not this milestone):
`thumb/%s.jpg?size=-%d`, `image/%s.jpg?size=%d`, `sthumb/%s.jpg`.

## 9. Domain-model mapping (pinacoteca)

- **Album** = virtual collection; many-to-many with `Photo`. `Album.kind` covers
  Folder / Starred / RecentUpdates / Emailed / Exported / Screensaver / (People,
  Places excluded from initial scope).
- **View modes** map to a `LibraryView` enum (Timeline, Folders, All, Keywords,
  Search, Lightbox, Thumbnails{Small,Normal,Large}, Picture, Properties).
- **Rating/Star** = `Rating` on `Photo` backed by XMP::Rating semantics.
- **Keywords** = tags set on `Photo`.
- **Thumbnails** = derived cache keyed by (photo id, size class); store choice
  (own SQLite vs per-folder) to be decided in M4 design; must migrate
  `.picasa.ini` metadata forward (delicate-functionality preservation).

## 10. Open questions / follow-ups

- Exact `.db` SQLite schemas (tables/columns) — derive from DB file inspection,
  not string corpus. Tracked under `formats/`.
- Whether `thumbs.db` uses a size-column keyed schema and how size classes map.
- `.picasa.ini` key names and forward-migration rules (backward compatibility
  is a hard requirement).
- `TileWidth`/grid reflow math (square vs aspect tiles, caption rows).
- Timeline bucketing granularity (year/month/day) and ordering (date taken vs
  file date).
- Album sort/order semantics and `CAlbumVarDateConverter` date handling.
