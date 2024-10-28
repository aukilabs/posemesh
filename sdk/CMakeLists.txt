cmake_minimum_required(VERSION 3.30.2 FATAL_ERROR)

list(APPEND CMAKE_MODULE_PATH "${CMAKE_CURRENT_LIST_DIR}/cmake")
include(Platform)

project(Posemesh VERSION 0.1.0 LANGUAGES ${PLATFORM_LANGUAGES})
include(PlatformCheck)
include(ToolchainCheck)

if(APPLE)
    option(USE_APPLE_UMBRELLA_HEADER "Use Objective-C to Swift umbrella header on Apple platforms" ON)
    option(USE_APPLE_BRIDGING_HEADER "Use Objective-C to Swift bridging header on Apple platforms" OFF)

    if(USE_APPLE_UMBRELLA_HEADER AND USE_APPLE_BRIDGING_HEADER)
        message(FATAL_ERROR "Objective-C to Swift umbrella and bridging headers cannot be used simultaneously.")
    endif()

    list(APPEND ADD_PLATFORM_LIBRARY_EXTRA_FLAGS)
elseif(EMSCRIPTEN)
    if("${CMAKE_BUILD_TYPE}" STREQUAL "Debug")
        option(USE_BABEL_JS "Use Babel.js to transpile and minify JavaScript web code" OFF)
    else()
        option(USE_BABEL_JS "Use Babel.js to transpile and minify JavaScript web code" ON)
    endif()

    list(
        APPEND ADD_PLATFORM_LIBRARY_EXTRA_FLAGS
            SKIP_INSTALL_JS
            SKIP_INSTALL_TSD
    )
endif()

add_platform_library(
    Posemesh
    PUBLIC_HEADER_DIR "${CMAKE_CURRENT_LIST_DIR}/include"
    PUBLIC_C_HEADERS
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/C/Config.h"
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/C/Posemesh.h"
    PUBLIC_CXX_HEADERS
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/Config.hpp"
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/Posemesh.hpp"
    CXX_SOURCES
        "${CMAKE_CURRENT_LIST_DIR}/src/C/Posemesh.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/Posemesh.cpp"
    ${ADD_PLATFORM_LIBRARY_EXTRA_FLAGS}
)

if(APPLE)
    add_platform_sources(
        Posemesh
        PUBLIC_HEADER_DIR "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/include"
        PUBLIC_OBJC_HEADERS
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/include/Posemesh/Config.h"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/include/Posemesh/Posemesh.h"
        OBJCXX_SOURCES
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/src/Posemesh.mm"
    )
    if(USE_APPLE_UMBRELLA_HEADER)
        set_apple_platform_umbrella_header(
            Posemesh
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/include/Posemesh/Posemesh-Umbrella-Header.h"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/module.modulemap"
        )
    endif()
    if(USE_APPLE_BRIDGING_HEADER)
        set_apple_platform_bridging_header(Posemesh "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/src/Posemesh-Bridging-Header.h")
    endif()
elseif(EMSCRIPTEN)
    add_platform_sources(
        Posemesh
        CXX_SOURCES
            "${CMAKE_CURRENT_LIST_DIR}/platform/Web/src/Posemesh.cpp"
    )
endif()

target_compile_definitions(Posemesh PRIVATE POSEMESH_BUILD)

include(LinkNetworkingLibrary)
link_networking_library(Posemesh)

if(EMSCRIPTEN)
    include(MergeFiles)
    merge_files(
        Posemesh_Merged
        OUTPUT "${CMAKE_CURRENT_BINARY_DIR}/Posemesh_Merged.js"
        INPUTS
            "${CMAKE_CURRENT_BINARY_DIR}/PosemeshNetworking_TextReplaced.js"
            Posemesh
            "${CMAKE_CURRENT_LIST_DIR}/platform/Web/Posemesh.js"
    )
    if(USE_BABEL_JS)
        include(TranspileMinifyJavaScript)
        transpile_minify_javascript(
            Posemesh_TranspiledMinified
            "${CMAKE_CURRENT_BINARY_DIR}/Posemesh_TranspiledMinified.js"
            "${CMAKE_CURRENT_BINARY_DIR}/Posemesh_Merged.js"
        )
        install(
            FILES
                "${CMAKE_CURRENT_BINARY_DIR}/Posemesh_TranspiledMinified.js"
            DESTINATION "${CMAKE_INSTALL_PREFIX}"
            RENAME "Posemesh.js"
        )
    else()
        install(
            FILES
                "${CMAKE_CURRENT_BINARY_DIR}/Posemesh_Merged.js"
            DESTINATION "${CMAKE_INSTALL_PREFIX}"
            RENAME "Posemesh.js"
        )
    endif()
endif()
