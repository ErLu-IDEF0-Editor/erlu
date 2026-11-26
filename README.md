[English](README.md) · [Русский](README.ru.md) · [Français](README.fr.md) · [Español](README.es.md) · [العربية](README.ar.md) · [中文](README.zh.md)

# erlu

**An IDEF0 and DFD modeling editor for desktop and the browser, built with Rust and egui.**

erlu helps you describe a system's functions and their inputs, controls, outputs, and mechanisms. Start with a context diagram and progressively expand your model through decomposition.

It is intended for analysts, engineers, and architects who use IDEF0 to understand and design processes. Developers can extend the editor with WASM plugins and connect external tools and AI agents.

## Features

- **Data flow diagrams** support decomposition, a data dictionary, and interface balancing.
- **IDEF0 modeling:** function blocks, orthogonal arrows, branches, decomposition, inherited interfaces, tunnels, and function references.
- **Document-wide history:** undo and redo survive navigation between diagrams and cover changes made through the UI, AI, and plugins.
- **Model validation:** diagnostics for structure, connections, numbering, and IDEF0 semantic rules.
- **Cost analysis:** assign function costs and calculate totals through decomposition.
- **Export:** PDF, SVG, and Markdown.
- **Appearance:** light and dark themes, object colors, and an English, Russian, French, Spanish, Arabic, and Simplified Chinese interface.
- **Automation:** desktop MCP and an external-agent connector in the browser.

Built-in checks help find modeling errors. They do not constitute full certification of standards compliance; see the [validation scope](docs/validation.md).

## Desktop and web

The desktop application targets Linux, Windows, and macOS. The browser version shares the same model, history, validation, and export engine.

Documents are processed locally. Browser editing requires neither an account nor a document database server. Open files using the file picker and save them by downloading. IndexedDB recovery and offline operation after the application cache is installed are supported. Recovery does not replace saving your files.

Linux/NixOS and Chromium, Firefox, and WebKit have been tested locally. See the [acceptance record](docs/acceptance.md) for the status of other platforms and browser engines.

## Plugins

Three WASM plugins are included:

| Plugin | Purpose |
| --- | --- |
| Colors | Set colors on selected blocks and arrows |
| [Responsibility Map](plugins/responsibility_map/README.md) | Organizational roles, responsibility assignments, checks, and RACI matrix export |
| [Semantic Dictionary](plugins/semantic_dictionary/README.md) | Concepts and synonyms, bindings to arrows, usage search, and label consistency |

The same WASM component runs through Wasmtime on desktop and a Worker in the browser. Plugins can add commands, checks, exporters, and input forms.

Plugin edits pass through the shared transactional engine and support undo/redo. The first SDK does not provide registration of new notations or arbitrary custom widgets.

## Quick start from source

### Nix / NixOS

From the repository root:

```sh
nix-shell
npm --prefix web ci
npm --prefix web run plugins
```

Run the desktop app, open the example model, or start the web app at `http://127.0.0.1:8080`, respectively:

```sh
cargo run -p erlu
```

```sh
cargo run -p erlu -- examples/order-fulfilment.idef0
```

```sh
trunk serve
```

The Nix shell provides build tools and runtime libraries. You do not need to install Trunk again through Cargo. It uses a compiler-specific build directory to keep Nix and rustup artifacts separate.

### Without Nix

You need Rust, Node.js 22+, Python 3, Trunk, and wasm-tools. The Rust version is pinned in `rust-toolchain.toml`.

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-tools --version 1.263.0 --locked
cargo install trunk --version 0.21.14 --locked
npm --prefix web ci
npm --prefix web run plugins
```

Then use the run commands above. Linux also needs X11/Wayland libraries, libxkbcommon, and OpenGL.

## Basic controls

| Action | Input |
| --- | --- |
| Add a block | Right-click |
| Create an arrow | Drag from a port or diagram boundary |
| Create a branch | Ctrl + drag from an arrow |
| Edit properties | Double-click |
| Open decomposition | Middle-click a block |
| Go up one level | Middle-click the background |
| Undo / redo | Ctrl+Z / Ctrl+Shift+Z |
| Fit diagram to window | F |

On macOS, standard editing shortcuts use Cmd.

## AI and automation

Connect an MCP client to a standalone editor process, or control the document in an open desktop window:

```sh
cargo run -p erlu -- mcp --headless
```

```sh
cargo run -p erlu -- --mcp
```

Connection details are available in the **AI** menu. MCP edits use the same history and checks as user actions. Saving to disk is explicit.

In the browser, external agents connect through the [JavaScript/TypeScript SDK](web/sdk/erlu-agent.js). Users enable access for a specific origin and can revoke it. The integration supplies the agent adapter; an embedded AI chat is not required. See the [AI API](docs/ai.md).

## For developers

| Crate | Responsibility |
| --- | --- |
| `erlu-core` | Model, commands, transactions, history, validation, geometry, and export |
| `erlu-app` | egui interface, desktop/web adapters, and MCP |
| `erlu-plugins` | Desktop WASM component loading and execution |

The UI, MCP, browser connector, and plugins share an action registry. Revision checks detect conflicts during asynchronous editing.

Documents use a versioned JSON `.idef0` format with an explicit feature list. A migration registry and preservation of unknown optional data support future compatibility. Legacy Qt documents and binary Qt plugins are unsupported.

Choose **File → New DFD** to create a separate `.dfd` document with either Gane–Sarson or Yourdon–DeMarco symbols, a data dictionary and flow balancing. See the [DFD guide](docs/dfd.md) and [order fulfilment example](examples/order-fulfilment.dfd).

### Checks and builds

```sh
cargo test --workspace --locked
cargo build -p erlu --release --locked
trunk build --release --locked
npm --prefix web test
```

The static web build is written to `dist/web` and can be served over HTTPS. The [release workflow](.github/workflows/release.yml) builds desktop archives, a web ZIP, and checksums. It targets GitHub Actions; Forgejo requires CI adaptation. Release workflows have not been run remotely.

## Documentation

- [Architecture](docs/architecture.md)
- [Interface localization](docs/localization.md)
- [Browser tests and CI](docs/browser-tests.md)
- [Document format and compatibility](docs/format.md)
- [AI API and MCP](docs/ai.md)
- [Plugin SDK](docs/plugins.md)
- [IDEF0 validation](docs/validation.md)
- [Acceptance record](docs/acceptance.md)

## License

Project code is licensed under LGPL-3.0-or-later. The bundled font license is in [assets/fonts/LICENSE-DejaVu.txt](assets/fonts/LICENSE-DejaVu.txt).

[Noto Sans SC — SIL Open Font License](assets/fonts/LICENSE-NotoSansSC.txt)
