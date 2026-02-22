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
)

target_include_directories(idef0_editor PRIVATE src)

target_link_libraries(idef0_editor PRIVATE Qt6::Widgets)
