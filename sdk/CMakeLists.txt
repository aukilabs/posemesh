cmake_minimum_required(VERSION 3.30.2 FATAL_ERROR)

list(APPEND CMAKE_MODULE_PATH "${CMAKE_CURRENT_LIST_DIR}/cmake")
include(Platform)

project(Posemesh VERSION 0.1.0 LANGUAGES ${PLATFORM_LANGUAGES})
include(PlatformCheck)
include(ToolchainCheck)

string(TIMESTAMP POSEMESH_CURRENT_YEAR "%Y")
if(POSEMESH_CURRENT_YEAR EQUAL 2024)
    set(POSEMESH_COPYRIGHT_YEAR "2024")
else()
    set(POSEMESH_COPYRIGHT_YEAR "2024-${POSEMESH_CURRENT_YEAR}")
endif()

include(GetCommitId)
get_commit_id(POSEMESH_COMMIT_ID)

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
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/C/API.h"
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/C/Config.h"
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/C/Posemesh.h"
    PUBLIC_CXX_HEADERS
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/API.hpp"
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/Config.hpp"
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/Posemesh.hpp"
    CXX_SOURCES
        "${CMAKE_CURRENT_LIST_DIR}/src/C/Config.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/C/Posemesh.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/Config.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/Posemesh.cpp"
    ${ADD_PLATFORM_LIBRARY_EXTRA_FLAGS}
)

target_compile_definitions(
    Posemesh
    PRIVATE
        POSEMESH_VERSION="${CMAKE_PROJECT_VERSION_MAJOR}.${CMAKE_PROJECT_VERSION_MINOR}.${CMAKE_PROJECT_VERSION_PATCH}"
        POSEMESH_COMMIT_ID="${POSEMESH_COMMIT_ID}"
)

if(APPLE)
    set_apple_info(Posemesh "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/Info.plist.in" CONFIGURE @ONLY)
    add_platform_sources(
        Posemesh
        PUBLIC_HEADER_DIR "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/include"
        PUBLIC_OBJC_HEADERS
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/include/Posemesh/API.h"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/include/Posemesh/Config.h"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/include/Posemesh/Posemesh.h"
        OBJCXX_SOURCES
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/src/Config.mm"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/src/Posemesh.mm"
        Swift_SOURCES
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/src/Config.swift"
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
            "${CMAKE_CURRENT_LIST_DIR}/platform/Web/src/API.cpp"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Web/src/Config.cpp"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Web/src/Posemesh.cpp"
    )
endif()

target_compile_definitions(Posemesh PRIVATE POSEMESH_BUILD)

include(LinkNetworkingLibrary)
link_networking_library(Posemesh)

include(LinkOpenCVLibrary)
link_opencv_library(Posemesh)

if(EMSCRIPTEN)
    configure_file(
        "${CMAKE_CURRENT_LIST_DIR}/platform/Web/VerifyCommitId.js.in"
        "${CMAKE_BINARY_DIR}/VerifyCommitId.js"
        @ONLY
    )
    include(MergeFiles)
    merge_files(
        Posemesh_Merged
        OUTPUT "${CMAKE_CURRENT_BINARY_DIR}/Posemesh_Merged.js"
        PREPEND_STRING "/* Copyright (c) Auki Labs Limited ${POSEMESH_COPYRIGHT_YEAR}, ${CMAKE_PROJECT_VERSION_MAJOR}.${CMAKE_PROJECT_VERSION_MINOR}.${CMAKE_PROJECT_VERSION_PATCH}, ${POSEMESH_COMMIT_ID} */"
        NEW_LINE_AFTER_PREPENDED_STRING
        INPUTS
            "${CMAKE_CURRENT_BINARY_DIR}/PosemeshNetworking_TextReplaced.js"
            Posemesh
            "${CMAKE_CURRENT_LIST_DIR}/platform/Web/API.js"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Web/Config.js"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Web/Posemesh.js"
            "${CMAKE_BINARY_DIR}/VerifyCommitId.js"
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
    include(TransformTypeScriptDefinitions)
    transform_typescript_definitions(
        Posemesh_TransformedTypeScriptDefinitions
        "${CMAKE_CURRENT_BINARY_DIR}/Posemesh_TransformedTypeScriptDefinitions.d.ts"
        "${CMAKE_CURRENT_BINARY_DIR}/Posemesh.d.ts"
    )
    install(
        FILES
            "${CMAKE_CURRENT_BINARY_DIR}/Posemesh_TransformedTypeScriptDefinitions.d.ts"
        DESTINATION "${CMAKE_INSTALL_PREFIX}"
        RENAME "Posemesh.d.ts"
    )
endif()
