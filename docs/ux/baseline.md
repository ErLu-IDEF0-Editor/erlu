# Qt baseline

Historical reference: Git revision `58198c6`. The Qt source and its build files
were removed after the Rust desktop/web Nix-shell smoke checks passed.
The observed workflows and screenshots below remain the UX reference.

Observed screenshots: startup choice, metadata tabs, initial A0 document, decomposition.
The virtual display is 1280 × 900; the main window is 1150 × 850. These screenshots
are evidence for the source behavior, not acceptance screenshots of the Rust app.

| Area | Trigger | Expected behavior |
| --- | --- | --- |
| Start | Launch without a path | Choose Open/New/Cancel; New opens metadata tabs |
| Initial document | Accept General/ABC Units/Page Setup | One A0 block centered in the content frame |
| Blocks | RMB within content | New centered block, clamped within frame, automatic number |
| Properties | Double-click block | Multiline name, optional price 0–1e9 with two decimals |
| Decomposition indicator | Block without populated child | Fold in upper-left corner; populated child removes fold |
| Arrow start | LMB on output side, input/control/mechanism boundary, junction | Preview connection until release; Esc cancels |
| Arrow target | Release on input/control/mechanism side, junction, arrow, boundary | Attach endpoint; joining an arrow creates a junction; invalid target cancels |
| Branch | Ctrl + LMB on arrow, drag | Split arrow, create branch, inherit root label |
| Labels | Double-click editable arrow | Edit label; inherited interface label remains locked |
| Geometry | Drag label, arrow segment or endpoint | Move only that part; reconnect endpoints along their side |
| Interfaces | Decompose connected block | Boundary stubs inherit parent labels and port positions |
| Interface position | Shift + drag interface circle | Move along its boundary |
| Tunnel | Interface deletion / tunnel state | Preserve parent/child correspondence and tunnel markers |
| Navigation | MMB on block / background | Enter decomposition / go up |
| Navigation | Ctrl+Down/PageDown; Ctrl+Up/PageUp | Enter selected block / go up |
| Touchpad | Pinch out > 1.15; pinch in < .85 | Enter block under pointer / go up; this is not canvas zoom |
| Scale | Fit/F, Scale+/Scale− | Fit scene; change scale by factor 1.1 |
| Numbering | Root; A0 children; deeper children | A0, A00…; A1, A2…; A1.1, A1.2… |
| Calls | Select one block, choose reference, click bottom frame | Mechanism arrow labelled with referenced number and title |
| Validation | Tools → Validation | Errors and warnings across all decompositions |
| Costs | Tools → Calculate Prices | Bottom-up sum of specified child costs; absent values remain absent |
| Export | Current/all, page numbering, force light | Vector PDF with original page sizes; Markdown model explanation |
| Colors | Plugin menu | Set/clear per-item overrides on selected blocks/arrows |
| Languages | System locale | RU, EN, FR; explicit language selection added in Rust |

Intentional changes agreed for migration: document-wide undo/redo, versioned format,
atomic edits shared with AI/plugins, errors do not replace the current document.
Browser file operations use picker/download, recovery uses IndexedDB; browser-reserved
shortcuts have menu alternatives. Qt widgets need not be pixel-identical.

Remaining manual acceptance work is tracked in `docs/acceptance.md`; source inspection
alone is not evidence that a scenario passed interactively on every target platform.

Source-level defects deliberately corrected: loading previously reset the scene before
checking document structure; local history was reset by navigation; branch/source IDs
were not persisted as stable document identities. `startCallMechanism` created an arrow
immediately at a temporary point despite asking the user to click the lower frame;
the Rust UI consistently uses the requested lower-frame placement step.
