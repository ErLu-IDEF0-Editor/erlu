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
Saved state is a document baseline; navigation, selection, language and theme do
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
