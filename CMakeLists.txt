cmake_minimum_required(VERSION 3.21)
project(idef0_editor LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

find_package(Qt6 REQUIRED COMPONENTS Widgets)

qt_standard_project_setup()

qt_add_executable(idef0_editor
  src/main.cpp
  src/MainWindow.h src/MainWindow.cpp
  src/items/DiagramScene.h src/items/DiagramScene.cpp
  src/items/BlockItem.h   src/items/BlockItem.cpp
  src/items/ArrowItem.h   src/items/ArrowItem.cpp
  src/items/JunctionItem.h src/items/JunctionItem.cpp
  src/items/HeaderFooterItem.h src/items/HeaderFooterItem.cpp
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

target_include_directories(idef0_editor PRIVATE src)

target_link_libraries(idef0_editor PRIVATE Qt6::Widgets)

install(TARGETS idef0_editor
  BUNDLE DESTINATION .
  RUNTIME DESTINATION bin
)

if(UNIX AND NOT APPLE)
  install(FILES ${CMAKE_CURRENT_SOURCE_DIR}/packaging/linux/idef0-editor.desktop
          DESTINATION share/applications)
  install(FILES ${CMAKE_CURRENT_SOURCE_DIR}/packaging/linux/idef0.xml
          DESTINATION share/mime/packages)
endif()
