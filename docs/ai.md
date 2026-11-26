# AI action API

All transports share action names, typed arguments, object IDs and engine errors.
Call `document_state` first. Pass `expected_revision` with edits to reject stale
work. A successful response is `{revision, dirty, result}`. An unsuccessful action
returns `{code, message}` without applying part of the edit. Plugin evaluation
uses a snapshot and always rechecks its revision before committing.

Action groups include document lifecycle/metadata, blocks, move/delete/selection,
number swapping, arrows/branches/interfaces/tunnels, call mechanisms, decomposition
and navigation, view/theme/language/grid, undo/redo, validation, costs, exports, extension
data and transactions. `erlu actions` emits the authoritative list and JSON schemas.
A transaction takes typed `{action,args}` commands and accepts document edits only.
Mouse move events and modal-dialog mechanics are represented by their semantic actions.

For DFD, use `dfd_new` and the typed operations of `dfd_edit`. Inspect `document.format` in `document_state` before editing. The DFD value is `erlu-dfd`. See the [DFD guide](dfd.md#file-format-and-automation).

## Desktop MCP

Use a process-launching client configuration, for example:

```json
{"mcpServers":{"erlu":{"command":"/absolute/path/to/erlu","args":["mcp","--headless"]}}}
```

To start with a saved model, append its path. GUI mode is explicitly enabled with
`erlu --mcp [model.idef0]`; its endpoint is `http://127.0.0.1:38479/mcp`. Copy the
connection configuration from AI in the application. Supply the displayed bearer
token. Only loopback is bound; unexpected Origin headers are rejected. Tokens are
process-local and are not written to documents. Multiple GUI instances should enable
MCP only in the instance being controlled; a port conflict is reported on stderr.

`rmcp` implements protocol negotiation. Stdio stdout is exclusively MCP JSON-RPC.
File dialogs are parameterized actions: `document_open` receives JSON text;
`document_serialize` returns JSON text. Exports return text or PDF bytes. The MCP
client decides where to persist returned artifacts. No action silently saves a model.

## Browser connector

In AI, enable this tab and enter the exact origin of the embedding page or extension
content script. Import `web/sdk/erlu-agent.js`:

```js
const agent = await connect(editorIframe.contentWindow, editorOrigin);
const state = await agent.call('document_state');
await agent.call('block_update', {id: 2, title: 'Model a process'}, state.revision);
const unsubscribe = agent.subscribe(state => console.log(state.revision));
agent.close();
```

The handshake uses `postMessage` to the exact target origin and transfers a dedicated
MessagePort. Requests include protocol `erlu.ai.v1`, the negotiated session ID and
request ID. No wildcard origins are accepted. Disabling access or changing the
allowed origin revokes all ports. HTTPS origins are required outside localhost.
The JS/TypeScript SDK has timeouts; a timed-out edit must not be blindly retried:
read the current state first. The example embedding page is `/sdk/example.html`.

The bridge is for an external agent in a parent page or extension. A remote agent
cannot directly address an arbitrary local tab; it needs an extension/embedding
adapter. Remote MCP hosting and an integrated AI chat are outside this release.

Interface locales: `language_set` accepts `en`, `ru`, `fr`, `es`, `ar`, and `zh`.
This changes view state, not document content. See [localization](localization.md).
