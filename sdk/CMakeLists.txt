cmake_minimum_required(VERSION 3.30.2 FATAL_ERROR)

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
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/C/ArucoDetection.h"
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/C/Posemesh.h"
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/C/CalibrationHelpers.h"
        "${POSEMESH_GENERATED_C_HEADERS}"
    PUBLIC_CXX_HEADERS
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/API.hpp"
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/Config.hpp"
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/PoseEstimation.hpp"
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/QRDetection.hpp"
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/ArucoDetection.hpp"
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/Posemesh.hpp"
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/PoseFactory.hpp"
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/PoseTools.hpp"
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/CalibrationHelpers.hpp"
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/BitMatrixTracker/Config.hpp"
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/BitMatrixTracker/Estimator.hpp"
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/BitMatrixTracker/Precompute.hpp"
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/BitMatrixTracker/Types.hpp"
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/Portals.hpp"
        "${POSEMESH_GENERATED_CXX_HEADERS}"
    CXX_SOURCES
        "${CMAKE_CURRENT_LIST_DIR}/src/C/Config.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/C/PoseEstimation.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/C/QRDetection.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/C/ArucoDetection.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/C/Posemesh.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/C/CalibrationHelpers.cpp"
        "${POSEMESH_GENERATED_C_SOURCES}"
        "${CMAKE_CURRENT_LIST_DIR}/src/Config.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/PoseEstimation.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/QRDetection.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/ArucoDetection.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/CompileTestFunctions.cpp" # UNIT TEST!
        "${CMAKE_CURRENT_LIST_DIR}/src/CompileTestMethods.cpp" # UNIT TEST!
        "${CMAKE_CURRENT_LIST_DIR}/src/Posemesh.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/Matrix2x2.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/Matrix3x3.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/Matrix4x4.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/Quaternion.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/Vector2.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/Vector3.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/Vector4.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/PoseFactory.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/PoseTools.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/CalibrationHelpers.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/BitMatrixTracker/Angles.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/BitMatrixTracker/Cluster.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/BitMatrixTracker/Config.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/BitMatrixTracker/CornerNet.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/BitMatrixTracker/Detect.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/BitMatrixTracker/Geometry.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/BitMatrixTracker/Impl.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/BitMatrixTracker/NearbyMask.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/BitMatrixTracker/Precompute.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/BitMatrixTracker/Preprocess.cpp"
        "${CMAKE_CURRENT_LIST_DIR}/src/BitMatrixTracker/Ransac.cpp"
        "${POSEMESH_GENERATED_CXX_SOURCES}"
        "${POSEMESH_GENERATED_PROTOBUF_CXX_HEADERS}"
        "${POSEMESH_GENERATED_PROTOBUF_CXX_SOURCES}"
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
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/include/Posemesh/ArucoDetection.h"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/include/Posemesh/CalibrationHelpers.h"
            "${POSEMESH_GENERATED_OBJC_HEADERS}"
        CXX_HEADERS
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/src/Util.hpp"
        OBJCXX_SOURCES
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/src/Config.mm"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/src/PoseEstimation.mm"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/src/Posemesh.mm"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/src/Util.mm"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/src/QRDetection.mm"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/src/ArucoDetection.mm"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/src/CalibrationHelpers.mm"
            "${POSEMESH_GENERATED_OBJC_SOURCES}"
        Swift_SOURCES
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/src/Config.swift"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/src/PoseEstimation.swift"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/src/QRDetection.swift"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/src/ArucoDetection.swift"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Apple/src/CalibrationHelpers.swift"
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
            "${CMAKE_CURRENT_LIST_DIR}/platform/Web/src/ArucoDetection.cpp"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Web/src/Posemesh.cpp"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Web/src/CalibrationHelpers.cpp"
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
            "${CMAKE_CURRENT_LIST_DIR}/platform/Web/ArucoDetection.js"
            "${CMAKE_CURRENT_LIST_DIR}/platform/Web/CalibrationHelpers.js"
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

# Standalone example binary for BitMatrixTracker
add_executable(
    BitMatrixTrackerExample
    "${CMAKE_CURRENT_LIST_DIR}/src/BitMatrixTracker/example_detect.cpp"
)
target_include_directories(
    BitMatrixTrackerExample
    PRIVATE
        "${CMAKE_CURRENT_LIST_DIR}/include/Posemesh/BitMatrixTracker"
)
target_link_libraries(
    BitMatrixTrackerExample
    PRIVATE
        Posemesh
)
link_opencv_library(BitMatrixTrackerExample)
set_target_properties(
    BitMatrixTrackerExample
    PROPERTIES
        MACOSX_BUNDLE OFF
)
target_compile_options(BitMatrixTrackerExample PRIVATE -g -O0 -fno-omit-frame-pointer)

