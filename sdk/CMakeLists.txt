cmake_minimum_required(VERSION 3.30.2 FATAL_ERROR)

set(CMAKE_MESSAGE_LOG_LEVEL TRACE) # for easier debugging

set(GENERATED_INTERFACE_FILES_CMAKE_FILE "${CMAKE_CURRENT_LIST_DIR}/cmake/GeneratedInterfaceFiles.cmake")
if(NOT EXISTS "${GENERATED_INTERFACE_FILES_CMAKE_FILE}" OR IS_DIRECTORY "${GENERATED_INTERFACE_FILES_CMAKE_FILE}")
    message(FATAL_ERROR "Required generated file 'cmake/GeneratedInterfaceFiles.cmake' is missing. Did you run 'npm run generate' in 'sdk/gentool' directory?")
endif()
include("${GENERATED_INTERFACE_FILES_CMAKE_FILE}")

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

find_package(ClangFormat)

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

include(AddProtobuf)
add_protobuf(Posemesh)

add_platform_library(
    Posemesh
    PUBLIC_HEADER_DIR "${CMAKE_CURRENT_LIST_DIR}/include"
    PUBLIC_C_HEADERS
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/C/API.h"
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/C/Config.h"
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/C/PoseEstimation.h"
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/C/QRDetection.h"
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/C/Posemesh.h"
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/C/Landmark.h"
        "${POSEMESH_GENERATED_C_HEADERS}"
    PUBLIC_CXX_HEADERS
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/API.hpp"
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/Config.hpp"
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/PoseEstimation.hpp"
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/QRDetection.hpp"
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/Posemesh.hpp"
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/Landmark.hpp"
        "${POSEMESH_GENERATED_PROTOBUF_CXX_HEADERS}"
        "${POSEMESH_GENERATED_CXX_HEADERS}"
    CXX_SOURCES
        "${CMAKE_CURRENT_LIST_DIR}/src/C/Config.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/C/PoseEstimation.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/C/QRDetection.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/C/Posemesh.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/C/Landmark.cpp"
        "${POSEMESH_GENERATED_C_SOURCES}"
        "${CMAKE_CURRENT_LIST_DIR}/src/Config.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/PoseEstimation.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/QRDetection.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/Posemesh.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/Vector2.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/Vector3.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/Vector4.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/Landmark.cpp"
        "${POSEMESH_GENERATED_PROTOBUF_CXX_SOURCES}"
        "${POSEMESH_GENERATED_CXX_SOURCES}"
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
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/include/Posemesh/PoseEstimation.h"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/include/Posemesh/Posemesh.h"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/include/Posemesh/QRDetection.h"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/include/Posemesh/Landmark.h"
            "${POSEMESH_GENERATED_OBJC_HEADERS}"
        CXX_HEADERS
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/src/Util.hpp"
        OBJCXX_SOURCES
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/src/Config.mm"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/src/PoseEstimation.mm"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/src/Posemesh.mm"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/src/Util.mm"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/src/QRDetection.mm"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/src/Landmark.mm"
            "${POSEMESH_GENERATED_OBJC_SOURCES}"
        Swift_SOURCES
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/src/Config.swift"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/src/PoseEstimation.swift"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/src/QRDetection.swift"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/src/Landmark.swift"
            "${POSEMESH_GENERATED_SWIFT_SOURCES}"
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
            "${CMAKE_CURRENT_LIST_DIR}/platform/Web/src/API2.cpp"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Web/src/Config.cpp"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Web/src/PoseEstimation.cpp"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Web/src/QRDetection.cpp"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Web/src/Posemesh.cpp"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Web/src/Landmark.cpp"
            "${POSEMESH_GENERATED_WEB_CXX_SOURCES}"
    )
endif()

target_compile_definitions(Posemesh PRIVATE POSEMESH_BUILD)

include(AddGLMHeaders)
add_glm_headers(Posemesh)

include(LinkBaseLibrary)
link_base_library(Posemesh)

include(LinkOpenCVLibrary)
link_opencv_library(Posemesh)

include(LinkProtobuf)
link_protobuf(Posemesh)

if(EMSCRIPTEN)
    include(ReplaceJavaScriptSymbols)
    replace_javascript_symbols(
        Posemesh_ReplacedJavaScriptSymbols
        "${CMAKE_CURRENT_BINARY_DIR}/Posemesh_ReplacedJavaScriptSymbols.js"
        Posemesh
    )
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
            "${CMAKE_CURRENT_LIST_DIR}/platform/Web/_Pre.js"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Web/_PosemeshBasePre.js"
            "${CMAKE_CURRENT_BINARY_DIR}/PosemeshBase_TextReplaced.js"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Web/_PosemeshBasePost.js"
            Posemesh_ReplacedJavaScriptSymbols
            "${CMAKE_CURRENT_LIST_DIR}/platform/Web/API.js"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Web/API2.js"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Web/Config.js"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Web/PoseEstimation.js"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Web/QRDetection.js"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Web/Posemesh.js"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Web/Landmark.js"
            "${POSEMESH_GENERATED_WEB_JS_SOURCES}"
            "${CMAKE_BINARY_DIR}/VerifyCommitId.js"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Web/_Post.js"
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

if(CLANG_FORMAT_FOUND)
    add_clang_format_target(PosemeshCodeFormat Posemesh)
endif()
