# Plugin Development Manual

This application loads runtime plugins as plain shared libraries located in `plugins/`. Plugins are isolated from internal types and talk to the host via the C ABI defined in `PluginApi.h`.

## Plugin API

- Header: `src/plugins/PluginApi.h`
- Entry point: a single exported C function
  ```c
  extern "C" bool idef0_plugin_init_v1(Idef0Host* host);
  ```
- You **must not** include or rely on any other app headers.

`Idef0Host` exposes function pointers:

| Function | Purpose |
| --- | --- |
| `size_t selected_items(void* opaque, Idef0SelectedItem* out, size_t max)` | Returns count of selected items, fills kind (`IDEF0_ITEM_BLOCK` / `IDEF0_ITEM_ARROW`) and `id`. |
| `bool set_item_color(void* opaque, int kind, int id, const char* color_hex)` | Set per-item color override as `#rrggbb`. |
| `bool clear_item_color(void* opaque, int kind, int id)` | Clear color override. |
| `void add_menu_action(void* opaque, const char* text, Idef0ActionCallback cb, void* user_data)` | Add an action under the Plugins menu; `cb` gets `(user_data, host)`. |
| `const char* plugin_dir(void* opaque)` | Absolute directory of this plugin library (use to load plugin-local resources/translations). |

## Minimal Example (C++)

```cpp
#include "plugins/PluginApi.h"
#include <QObject>      // for tr(), optional
#include <QColorDialog> // optional UI

static void onSetRed(void*, Idef0Host* host) {
  Idef0SelectedItem items[32];
  size_t n = host->selected_items(host->opaque, items, 32);
  for (size_t i = 0; i < n; ++i) {
    host->set_item_color(host->opaque, items[i].kind, items[i].id, "#ff0000");
  }
}

extern "C" bool idef0_plugin_init_v1(Idef0Host* host) {
  if (!host || !host->add_menu_action) return false;
  host->add_menu_action(host->opaque, "Set red", &onSetRed, nullptr);
  return true;
}
```

## Build with CMake

```cmake
cmake_minimum_required(VERSION 3.21)
project(myplugin LANGUAGES CXX)
find_package(Qt6 REQUIRED COMPONENTS Widgets) # only if you use Qt UI

add_library(myplugin MODULE myplugin.cpp)
target_include_directories(myplugin PRIVATE /path/to/app/src) # for plugins/PluginApi.h
target_link_libraries(myplugin PRIVATE Qt6::Widgets)          # optional
set_target_properties(myplugin PROPERTIES
  LIBRARY_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/plugins"
  RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/plugins"
  INSTALL_RPATH "\$ORIGIN/../lib"
)
install(TARGETS myplugin LIBRARY DESTINATION plugins RUNTIME DESTINATION plugins)
```

If you don’t need Qt widgets, omit `find_package(Qt6...)` and link against nothing extra.

## Deployment

- Place the built library next to the app in `plugins/`:
  - Linux: `plugins/libmyplugin.so`
  - Windows: `plugins/myplugin.dll`
  - macOS: `plugins/libmyplugin.dylib`
- Ensure it is built against the same `PluginApi.h` version as the app.

## Best Practices

- Keep plugin state in your own globals; the host provides only opaque pointers.
- Validate inputs (e.g., color strings) and return early on null host functions.
- Use `selected_items` first, then act via `set_item_color`/`clear_item_color`.
- Add menu actions via `add_menu_action` for user-visible triggers. The host calls your callbacks on activation.
