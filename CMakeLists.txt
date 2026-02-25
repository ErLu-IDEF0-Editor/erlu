cmake_minimum_required(VERSION 3.21)
project(idef0_editor LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

find_package(Qt6 REQUIRED COMPONENTS Widgets)

qt_standard_project_setup()

qt_add_library(idef0_core SHARED
  src/MainWindow.h src/MainWindow.cpp
  src/items/DiagramScene.h src/items/DiagramScene.cpp
  src/items/BlockItem.h   src/items/BlockItem.cpp
  src/items/ArrowItem.h   src/items/ArrowItem.cpp
  src/items/JunctionItem.h src/items/JunctionItem.cpp
  src/items/HeaderFooterItem.h src/items/HeaderFooterItem.cpp
  src/plugins/PluginApi.h
  src/plugins/PluginManager.h src/plugins/PluginManager.cpp
)

option(BUILD_COLORS_PLUGIN "Build the Colors plugin" ON)

qt_add_executable(idef0_editor
  src/main.cpp
)

if(APPLE)
  set(MACOS_BUNDLE_ID "com.gregorybednov.idef0-editor")
  set(MACOS_BUNDLE_NAME "IDEF0 Editor")
  configure_file(
    ${CMAKE_CURRENT_SOURCE_DIR}/cmake/Info.plist.in
    ${CMAKE_CURRENT_BINARY_DIR}/Info.plist
    @ONLY
  )
  set_target_properties(idef0_editor PROPERTIES
    MACOSX_BUNDLE TRUE
    MACOSX_BUNDLE_BUNDLE_NAME "${MACOS_BUNDLE_NAME}"
    MACOSX_BUNDLE_GUI_IDENTIFIER "${MACOS_BUNDLE_ID}"
    MACOSX_BUNDLE_INFO_PLIST "${CMAKE_CURRENT_BINARY_DIR}/Info.plist"
  )
endif()

target_compile_definitions(idef0_editor PRIVATE $<$<NOT:$<CONFIG:Debug>>:QT_NO_DEBUG_OUTPUT>)

target_include_directories(idef0_core PRIVATE src)
target_include_directories(idef0_editor PRIVATE src)

target_link_libraries(idef0_core PRIVATE Qt6::Widgets)
target_link_libraries(idef0_editor PRIVATE Qt6::Widgets idef0_core)
set_target_properties(idef0_core PROPERTIES
  WINDOWS_EXPORT_ALL_SYMBOLS ON
  LIBRARY_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}"
  RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}"
  INSTALL_RPATH "\$ORIGIN"
)

if(BUILD_COLORS_PLUGIN)
  add_library(colorsplugin MODULE
    src/plugins/color/ColorsPlugin.h
    src/plugins/color/ColorsPlugin.cpp
  )
  target_link_libraries(colorsplugin PRIVATE idef0_core Qt6::Widgets)
  target_include_directories(colorsplugin PRIVATE src)
  set_target_properties(colorsplugin PROPERTIES
    LIBRARY_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/plugins/color"
    RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/plugins/color"
    INSTALL_RPATH "\$ORIGIN/../../lib"
  )
endif()

set_target_properties(idef0_editor PROPERTIES
  INSTALL_RPATH "\$ORIGIN/../lib"
)

install(TARGETS idef0_editor
  BUNDLE DESTINATION .
  RUNTIME DESTINATION bin
)
install(TARGETS idef0_core
  LIBRARY DESTINATION lib
  RUNTIME DESTINATION bin
  ARCHIVE DESTINATION lib
)
if(BUILD_COLORS_PLUGIN)
  install(TARGETS colorsplugin
    LIBRARY DESTINATION plugins/color
    RUNTIME DESTINATION plugins/color
  )
endif()

if(UNIX AND NOT APPLE)
  install(FILES ${CMAKE_CURRENT_SOURCE_DIR}/packaging/linux/idef0-editor.desktop
          DESTINATION share/applications)
  install(FILES ${CMAKE_CURRENT_SOURCE_DIR}/packaging/linux/idef0.xml
          DESTINATION share/mime/packages)
endif()
