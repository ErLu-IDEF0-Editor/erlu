#pragma once

#include <cstddef>

extern "C" {

enum Idef0ItemKind {
  IDEF0_ITEM_BLOCK = 1,
  IDEF0_ITEM_ARROW = 2
};

struct Idef0SelectedItem {
  int kind; // Idef0ItemKind
  int id;
};

struct Idef0Host;
typedef void (*Idef0ActionCallback)(void* user_data, Idef0Host* host);

struct Idef0Host {
  void* opaque; // host-specific pointer
  // Fills up to max entries, returns actual count.
  size_t (*selected_items)(void* opaque, Idef0SelectedItem* out, size_t max);
  // Set hex color (#rrggbb) for item id/kind. Returns true on success.
  bool (*set_item_color)(void* opaque, int kind, int id, const char* color_hex);
  // Clear override color.
  bool (*clear_item_color)(void* opaque, int kind, int id);
  // Add menu action under Plugins; callback invoked on trigger.
  void (*add_menu_action)(void* opaque, const char* text, Idef0ActionCallback cb, void* user_data);
  // Absolute directory where this plugin library resides.
  const char* (*plugin_dir)(void* opaque);
};

// Plugin entry point must be exported with this name.
typedef bool (*Idef0PluginInit)(Idef0Host* host);

} // extern "C"
