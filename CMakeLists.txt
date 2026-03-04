cmake_minimum_required(VERSION 3.21)
project(erlu_idef0_editor LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

find_package(Qt6 REQUIRED COMPONENTS Widgets Svg)

qt_standard_project_setup()

set(APP_TS_FILES
  ${CMAKE_CURRENT_SOURCE_DIR}/translations/idef0_en.ts
  ${CMAKE_CURRENT_SOURCE_DIR}/translations/idef0_fr.ts
  ${CMAKE_CURRENT_SOURCE_DIR}/translations/idef0_ru.ts
)

set(COLORS_TS_FILES
  ${CMAKE_CURRENT_SOURCE_DIR}/src/plugins/color/translations/colors_en.ts
  ${CMAKE_CURRENT_SOURCE_DIR}/src/plugins/color/translations/colors_fr.ts
  ${CMAKE_CURRENT_SOURCE_DIR}/src/plugins/color/translations/colors_ru.ts
)

find_program(QT_LRELEASE_EXECUTABLE NAMES lrelease-qt6 lrelease REQUIRED)

function(generate_qm_files OUT_VAR OUT_DIR)
  set(_qm_files "")
  foreach(_ts_file IN LISTS ARGN)
    get_filename_component(_ts_name "${_ts_file}" NAME_WE)
    set(_qm_file "${OUT_DIR}/${_ts_name}.qm")
    add_custom_command(
      OUTPUT "${_qm_file}"
      COMMAND "${CMAKE_COMMAND}" -E make_directory "${OUT_DIR}"
      COMMAND "${QT_LRELEASE_EXECUTABLE}" "${_ts_file}" -qm "${_qm_file}"
      DEPENDS "${_ts_file}"
      VERBATIM
    )
    list(APPEND _qm_files "${_qm_file}")
  endforeach()
  set(${OUT_VAR} "${_qm_files}" PARENT_SCOPE)
endfunction()

generate_qm_files(
  APP_QM_FILES
  "${CMAKE_CURRENT_BINARY_DIR}/translations"
  ${APP_TS_FILES}
)

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

qt_add_executable(erlu_idef0_editor
  src/main.cpp
)

if(WIN32)
  target_sources(erlu_idef0_editor PRIVATE
    packaging/windows/erlu.rc
  )
endif()

qt_add_resources(erlu_idef0_editor "app_icons"
  PREFIX "/icons"
  BASE assets/icons
  FILES assets/icons/erlu.svg
)

if(APPLE)
  set(MACOS_BUNDLE_ID "com.gregorybednov.erlu-idef0-editor")
  set(MACOS_BUNDLE_NAME "erlu IDEF0 editor")
  configure_file(
    ${CMAKE_CURRENT_SOURCE_DIR}/cmake/Info.plist.in
    ${CMAKE_CURRENT_BINARY_DIR}/Info.plist
    @ONLY
  )
  set_target_properties(erlu_idef0_editor PROPERTIES
    MACOSX_BUNDLE TRUE
    MACOSX_BUNDLE_BUNDLE_NAME "${MACOS_BUNDLE_NAME}"
    MACOSX_BUNDLE_GUI_IDENTIFIER "${MACOS_BUNDLE_ID}"
    MACOSX_BUNDLE_INFO_PLIST "${CMAKE_CURRENT_BINARY_DIR}/Info.plist"
  )
endif()

if(WIN32)
  set_target_properties(erlu_idef0_editor PROPERTIES
    WIN32_EXECUTABLE TRUE
  )
endif()

target_compile_definitions(erlu_idef0_editor PRIVATE $<$<NOT:$<CONFIG:Debug>>:QT_NO_DEBUG_OUTPUT>)

target_include_directories(idef0_core PRIVATE src)
target_include_directories(erlu_idef0_editor PRIVATE src)

target_link_libraries(idef0_core PRIVATE Qt6::Widgets)
target_link_libraries(erlu_idef0_editor PRIVATE Qt6::Widgets Qt6::Svg idef0_core)
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
  generate_qm_files(
    COLORS_QM_FILES
    "${CMAKE_CURRENT_BINARY_DIR}/plugins/color/translations"
    ${COLORS_TS_FILES}
  )
endif()

add_custom_target(translations ALL DEPENDS ${APP_QM_FILES} ${COLORS_QM_FILES})
add_dependencies(erlu_idef0_editor translations)
if(BUILD_COLORS_PLUGIN)
  add_dependencies(colorsplugin translations)
endif()

set_target_properties(erlu_idef0_editor PROPERTIES
  INSTALL_RPATH "\$ORIGIN/../lib"
)

install(TARGETS erlu_idef0_editor
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

install(FILES ${APP_QM_FILES}
  DESTINATION translations
)
if(BUILD_COLORS_PLUGIN)
  install(FILES ${COLORS_QM_FILES}
    DESTINATION plugins/color/translations
  )
endif()

if(UNIX AND NOT APPLE)
  install(FILES ${CMAKE_CURRENT_SOURCE_DIR}/packaging/linux/erlu-idef0-editor.desktop
          DESTINATION share/applications)
  install(FILES ${CMAKE_CURRENT_SOURCE_DIR}/packaging/linux/idef0.xml
          DESTINATION share/mime/packages)
  install(FILES ${CMAKE_CURRENT_SOURCE_DIR}/assets/icons/erlu.svg
          DESTINATION share/icons/hicolor/scalable/apps)
endif()
