cmake_minimum_required(VERSION 3.21)

project(tiss-greetd-ui LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)
set(CMAKE_AUTOUIC ON)

find_package(Qt6 REQUIRED COMPONENTS Core Gui Qml Quick QuickControls2)

qt_policy(SET QTP0001 NEW)
qt_policy(SET QTP0004 NEW)

qt_add_executable(tiss-greetd-ui
    src/main.cpp
    src/BackendProcess.cpp
    src/BackendProcess.h
)

qt_add_qml_module(tiss-greetd-ui
    URI TissGreetd
    VERSION 1.0
    QML_FILES
        qml/Main.qml
)

set_source_files_properties(
    qml/TissCompat/Token.qml
    PROPERTIES
        QT_QML_SINGLETON_TYPE TRUE
)

add_library(tiss-compat-qml STATIC)

qt_add_qml_module(tiss-compat-qml
    URI TissCompat
    VERSION 1.0
    OUTPUT_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}/TissCompat
    QML_FILES
        qml/TissCompat/Token.qml
        qml/TissCompat/Scene.qml
        qml/TissCompat/SceneHost.qml
        qml/TissCompat/Background.qml
        qml/TissCompat/Clock.qml
        qml/TissCompat/StatusBanner.qml
        qml/TissCompat/AuthCard.qml
)

target_link_libraries(tiss-greetd-ui
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Qml
        Qt6::Quick
        Qt6::QuickControls2
        tiss-compat-qml
)
