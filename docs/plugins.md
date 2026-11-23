# Plugin SDK v1

The portable unit is a WebAssembly Component exporting the `erlu:plugin@1.0.0`
world in `plugins/colors/wit/plugin.wit`. There are no host/WASI imports. The pure
`run(action, context)` export receives a JSON snapshot containing `api_version`,
`revision`, `document`, `view`, and `arguments`, and returns JSON:

```json
{"commands":[{"action":"colors_set","args":{"ids":[2],"color":"#ff0000"}}],"result":{}}
```

Commands are proposals, never direct mutations. The host checks the whole transaction
and commits only against the snapshot revision. `validation` and `export` actions
must return no commands; `command` actions may propose edits. Results are JSON and
may contain diagnostic lists or exported text/bytes. GUI forms are generated from
action input schemas (string, number, boolean, enum and JSON values). Complex nested
inputs use JSON fields. The result is available through MCP/web and the GUI report.

A manifest declares `id`, `api_version`, package `version`, component path, optional
browser binding path and action declarations with name, title, kind and input_schema.
`plugins/colors/plugin.json` is the working example. Plugin action names are exposed
as `plugin_<id>_<action-with-dots-replaced-by-underscores>` and are discoverable at
connection time. Query the tool list again after loading a plugin in the GUI.

Build with `npm --prefix web run plugins`. The SDK builds the Rust guest for
`wasm32-unknown-unknown`, caps its linear memory at 32 MiB, wraps it with wasm-tools,
and generates browser bindings with jco. Native loads the component through Wasmtime;
web loads its SDK-generated bindings in a fresh Worker. The colors, Responsibility Map and Semantic Dictionary plugins
are deployed with the application on both platforms.

Native imposes 32 MiB memory, fuel and a two-second epoch deadline. Browser execution
has a two-second Worker deadline; SDK components have the declared memory cap.
Neither host gives a component filesystem, network or DOM imports. A trapped/timed-out
plugin never applies partial edits. Package-generated JavaScript is trusted host glue:
web installations are restricted to SDK-built packages deployed on the application's
own origin, not arbitrary remote JavaScript. Native accepts a selected manifest;
web's Load plugin takes a same-origin manifest URL. Package installation itself is
an explicit user/deployment action.

Own data belongs under `extensions[plugin-id]`, with a positive version and `data`.
Use `extension_set` in a transaction. Preserve other plugins' namespaces. Declare
model-affecting capabilities as required; optional missing plugin data is roundtripped.
Custom native widgets and plugin-provided notations are outside SDK v1.

## Responsibility Map

The bundled `responsibility_map` component provides a role registry, explicit RACI
assignments to selected function blocks, plugin-specific responsibility diagnostics
and Markdown/CSV/JSON matrix exports. Its data is optional, namespaced and undoable.
See the [usage and API guide](../plugins/responsibility_map/README.md).

Plugin results with a `text` string are shown as readable reports. Results that
also provide `filename` and `mime` expose Save report in desktop and web; structured
results are retained unchanged in the MCP/web response. Reports can be copied.

## Semantic Dictionary and preview confirmation

The bundled `semantic_dictionary` component adds concepts, synonyms, arrow bindings,
terminology diagnostics, catalog exchange and explicit label normalization. See its
[usage and API guide](../plugins/semantic_dictionary/README.md).

A read-only preview may return `result.confirm_action` with exactly `name`,
`arguments` (an object), and `expected_revision`. This does not apply edits. The
host verifies that the target is a registered command of the originating plugin
and that the revision equals the evaluated snapshot. The GUI offers Apply previewed
changes; clicking invokes that command against the saved revision. A stale report
is rejected and must be regenerated. MCP/web clients receive the same metadata and
perform a separate explicit call.

Command input schemas may set `x-erlu-confirm-only: true` to hide apply-only actions
from the ordinary GUI menu without hiding them from API discovery. This annotation
is presentation-only, not authorization; apply actions must check their own preview
arguments and revision. Preview arguments must freeze object IDs rather than depend
on later selection/navigation, which do not change the document revision.
