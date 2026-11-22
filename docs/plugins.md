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
web loads its SDK-generated bindings in a fresh Worker. The default colors plugin
is deployed with the application on both platforms.

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
