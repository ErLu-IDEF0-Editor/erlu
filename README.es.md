[English](README.md) · [Русский](README.ru.md) · [Français](README.fr.md) · [Español](README.es.md) · [العربية](README.ar.md) · [中文](README.zh.md)

# erlu

**Editor de modelos funcionales IDEF0 para escritorio y navegador, desarrollado en Rust con egui.**

erlu permite describir las funciones de un sistema, sus entradas, controles, salidas y mecanismos. Empiece con un diagrama de contexto y amplíe el modelo mediante descomposición.

Está dirigido a analistas, ingenieros y arquitectos que utilizan IDEF0 para comprender y diseñar procesos. Los desarrolladores pueden ampliar el editor con plugins WASM y conectar herramientas externas y agentes de IA.

## Funciones

- **Modelado IDEF0:** bloques funcionales, flechas ortogonales, ramificaciones, descomposición, interfaces heredadas, túneles y referencias a funciones.
- **Historial global:** deshacer y rehacer entre diagramas, incluidas las modificaciones de la interfaz, la IA y los plugins.
- **Validación:** diagnósticos de estructura, conexiones, numeración y reglas semánticas IDEF0.
- **Análisis de costes:** costes de funciones y cálculo mediante descomposición.
- **Exportación:** PDF, SVG y Markdown.
- **Presentación:** temas claro y oscuro, colores de objetos e interfaz en inglés, ruso, francés, español, árabe y chino simplificado.
- **Automatización:** MCP en escritorio y conector de agentes externos en el navegador.

Las comprobaciones ayudan a encontrar errores, pero no constituyen una certificación completa de conformidad. Consulte el [alcance de la validación](docs/validation.md).

## Escritorio y web

La aplicación está orientada a Linux, Windows y macOS. La versión web comparte el motor de modelo, historial, validación y exportación.

Los documentos se procesan localmente. La edición en el navegador no requiere cuenta ni servidor de almacenamiento de documentos. Los archivos se abren con un selector y se guardan descargándolos. Se incluye recuperación en IndexedDB y funcionamiento sin conexión tras instalar la caché. La recuperación no sustituye el guardado de archivos.

Linux/NixOS y los navegadores Chromium, Firefox y WebKit se han probado localmente. El [registro de aceptación](docs/acceptance.md) detalla el estado de otras plataformas y navegadores.

## Plugins

Se incluyen tres plugins WASM:

| Plugin | Función |
| --- | --- |
| Colors | Colores de bloques y flechas seleccionados |
| [Responsibility Map](plugins/responsibility_map/README.md) | Roles organizativos, responsabilidades, comprobaciones y exportación RACI |
| [Semantic Dictionary](plugins/semantic_dictionary/README.md) | Conceptos y sinónimos, vinculación con flechas, búsqueda de usos y coherencia de etiquetas |

El mismo componente WASM se ejecuta con Wasmtime en escritorio y en un Worker en el navegador. Los plugins añaden comandos, comprobaciones, exportadores y formularios.

Sus modificaciones pasan por el motor transaccional común y admiten deshacer/rehacer. El primer SDK no permite registrar nuevas notaciones ni widgets personalizados arbitrarios.

## Inicio desde el código fuente

### Nix / NixOS

Desde la raíz del repositorio:

```sh
nix-shell
npm --prefix web ci
npm --prefix web run plugins
```

Respectivamente: iniciar la aplicación, abrir el ejemplo o iniciar la web en `http://127.0.0.1:8080`:

```sh
cargo run -p erlu
```

```sh
cargo run -p erlu -- examples/order-fulfilment.idef0
```

```sh
trunk serve
```

El shell de Nix proporciona las herramientas y bibliotecas de ejecución. No es necesario reinstalar Trunk con Cargo. Un directorio por compilador separa los artefactos de Nix y rustup.

### Sin Nix

Se necesitan Rust, Node.js 22+, Python 3, Trunk y wasm-tools. La versión de Rust está fijada en `rust-toolchain.toml`.

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-tools --version 1.263.0 --locked
cargo install trunk --version 0.21.14 --locked
npm --prefix web ci
npm --prefix web run plugins
```

Utilice después los comandos de inicio anteriores. Linux también requiere bibliotecas X11/Wayland, libxkbcommon y OpenGL.

## Controles básicos

| Acción | Control |
| --- | --- |
| Añadir bloque | Clic derecho |
| Crear flecha | Arrastrar desde un puerto o borde del diagrama |
| Crear ramificación | Ctrl + arrastrar desde una flecha |
| Editar propiedades | Doble clic |
| Abrir descomposición | Clic central sobre un bloque |
| Subir un nivel | Clic central sobre el fondo |
| Deshacer / rehacer | Ctrl+Z / Ctrl+Shift+Z |
| Ajustar a la ventana | F |

En macOS, los atajos de edición habituales utilizan Cmd.

## IA y automatización

Conecte un cliente MCP a un proceso independiente o controle el documento de una ventana abierta:

```sh
cargo run -p erlu -- mcp --headless
```

```sh
cargo run -p erlu -- --mcp
```

Los parámetros están en el menú **AI**. Las modificaciones MCP utilizan el mismo historial y validación que la interfaz. El guardado en disco es explícito.

En el navegador, los agentes utilizan el [SDK JavaScript/TypeScript](web/sdk/erlu-agent.js). El usuario autoriza un origen concreto y puede revocar el acceso. La integración proporciona el adaptador del agente; no se necesita un chat de IA integrado. Consulte la [API de IA](docs/ai.md).

## Para desarrolladores

| Crate | Responsabilidad |
| --- | --- |
| `erlu-core` | Modelo, comandos, transacciones, historial, validación, geometría y exportación |
| `erlu-app` | Interfaz egui, adaptadores de escritorio/web y MCP |
| `erlu-plugins` | Carga y ejecución de componentes WASM en escritorio |

La interfaz, MCP, el conector web y los plugins comparten un registro de acciones. Las comprobaciones de revisión detectan conflictos durante la edición asíncrona.

Los documentos utilizan JSON `.idef0` con versión y una lista explícita de capacidades. Un registro de migraciones y la conservación de datos opcionales desconocidos facilitan la compatibilidad futura. No se admiten los antiguos documentos Qt ni plugins Qt binarios.

IDEF0 es la notación admitida. DFD, otras familias IDEF y BPMN no son modos de edición implementados.

### Comprobaciones y compilación

```sh
cargo test --workspace --locked
cargo build -p erlu --release --locked
trunk build --release --locked
npm --prefix web test
```

La web estática se genera en `dist/web` y puede alojarse mediante HTTPS. El [workflow de distribución](.github/workflows/release.yml) genera archivos de escritorio, un ZIP web y sumas de comprobación. Está diseñado para GitHub Actions; Forgejo requiere adaptar el CI. Este workflow aún no se ha ejecutado remotamente.

## Documentación

- [Arquitectura](docs/architecture.md)
- [Localización de la interfaz](docs/localization.md)
- [Pruebas de navegadores y CI](docs/browser-tests.md)
- [Formato y compatibilidad](docs/format.md)
- [API de IA y MCP](docs/ai.md)
- [SDK de plugins](docs/plugins.md)
- [Validación IDEF0](docs/validation.md)
- [Registro de aceptación](docs/acceptance.md)

## Licencia

El código se distribuye bajo LGPL-3.0-or-later. Licencia de la fuente incluida: [assets/fonts/LICENSE-DejaVu.txt](assets/fonts/LICENSE-DejaVu.txt).

[Noto Sans SC — SIL Open Font License](assets/fonts/LICENSE-NotoSansSC.txt)
