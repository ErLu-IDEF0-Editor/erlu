# Architecture

`erlu-core` owns the document, stable IDs, notation, action registry, structural
invariants, semantic diagnostics, command transactions, history, scene and exports.
It builds for native and wasm without a host filesystem, network or GUI runtime.
`erlu-app` owns egui interaction, dialogs and platform adapters. `erlu-plugins` is
only compiled on native targets. Browser adapters live in `web/runtime.js`.

A gesture has a transient preview and commits one typed command on release.
The engine snapshots before dispatch, applies the command, checks structural
invariants, and restores the snapshot on any failure. Compound edits use a single
non-nested transaction. Undo/redo restores the entire document, not the current view.
Saved state is a document baseline; navigation, selection, language, theme and grid preferences do
not dirty a document. The monotonic allocator is retained across undo and serialization
so an object ID is not reassigned to another object in that document's lifetime.

MCP, the browser bridge and plugin proposals all call this engine. Revision checks
prevent stale commits after asynchronous plugin evaluation or concurrent GUI/AI edits.
File writes happen in host adapters after serialization, and only a successful write
marks the matching revision saved. Reading, parsing and migration happen before
replacing the active document. Browser recovery stores both the working document
and its saved baseline, retaining the correct dirty status after restoration.

Presentation uses millimetres. Orthogonal routing uses a visibility grid around
inflated block bounds with bend penalties. Scene primitives feed both egui and SVG;
PDF embeds SVG form XObjects on physical pages. The bundled DejaVu Sans font avoids
system-font dependence and supports Cyrillic and French.

IDEF0 rules stay in the notation/model and validation layers. Another notation must
register its own semantics, port rules, diagnostics and presentation before becoming
a supported required feature. DFD and other IDEF variants are not placeholder modes;
unknown required notations are rejected. The first plugin SDK cannot register notations.

The browser app is statically hosted and processes documents locally. IndexedDB is
recovery storage; service-worker precaching includes hashed wasm and plugin assets.
It needs neither a document server nor an AI provider key. A future remote agent,
chat backend or MCP relay can reuse action schemas without changing the document model.

## Arrow geometry and drawing grid

`routing` shares geometry assistance between live drag previews and committed
commands. Routing removes retraced U-turns, tiny zigzags (up to 2 mm), and obsolete
waypoints covered by a moved block. Shortcuts must clear inflated block bounds;
large deliberate steps and obstacle detours remain. Junctions have no artificial
left/right lead-in, and their topology and label inheritance are unchanged.
Geometry changes and their normalization share one undo entry. Rendering old files
also simplifies the displayed path without silently editing their stored model.

A new block-to-frame connection within 2 mm of straight alignment moves the frame
anchor to the block's coordinate. The opposite drawing direction uses the same
rule; inherited interfaces retain their fixed parent boundary position. Dragging
a first or last segment slides its endpoint along the attached edge. Shared
junction endpoints remain fixed unless the junction itself is moved.

**View → Grid mode**, or **G** outside text fields, toggles a drawing grid with a
default 5 mm step; the menu accepts 1–50 mm. It snaps pointer-created blocks,
block/junction drags, arrow attachment points along their edges, and dragged arrow
segments. Grid mode does not rearrange existing objects or appear in exports.
Grid preferences are stored with the application settings, outside the document.
`grid_set` exposes the preference to agents; explicit action coordinates remain
exact instead of being rounded implicitly.
