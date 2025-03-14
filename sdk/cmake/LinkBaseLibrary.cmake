include("${CMAKE_CURRENT_LIST_DIR}/CopyFileWithTextReplace.cmake")
include("${CMAKE_CURRENT_LIST_DIR}/GetRustTargetName.cmake")
include("${CMAKE_CURRENT_LIST_DIR}/LinkPlatformLibraries.cmake")

set(BASE_PREFIX "${CMAKE_CURRENT_LIST_DIR}/../../core")
set(BASE_PKG_PREFIX "${BASE_PREFIX}/pkg/base")
set(BASE_TARGET_PREFIX "${BASE_PREFIX}/target")

function(LINK_BASE_LIBRARY NAME)
    if(NOT TARGET ${NAME})
        message(FATAL_ERROR "Target '${NAME}' does not exist.")
    endif()

    get_rust_target_name(RUST_TARGET_NAME)
    set(BASE_TARGET_DIR "${BASE_TARGET_PREFIX}/${RUST_TARGET_NAME}")
    if("${CMAKE_BUILD_TYPE}" STREQUAL "Debug")
        set(BASE_PKG_PREFIX_WITH_BUILD_TYPE "${BASE_PKG_PREFIX}/Debug")
        set(BASE_TARGET_DIR_WITH_BUILD_TYPE "${BASE_TARGET_DIR}/debug")
    else()
        set(BASE_PKG_PREFIX_WITH_BUILD_TYPE "${BASE_PKG_PREFIX}/Release")
        set(BASE_TARGET_DIR_WITH_BUILD_TYPE "${BASE_TARGET_DIR}/release")
    endif()

    set(BASE_INCLUDE_DIR "${BASE_PREFIX}/include")

    if(LINUX OR APPLE)
        set(BASE_LIBRARY "${BASE_TARGET_DIR_WITH_BUILD_TYPE}/libbase_static.a")
    elseif(EMSCRIPTEN)
        set(BASE_LIBRARY_JS "${BASE_PKG_PREFIX_WITH_BUILD_TYPE}/PosemeshBase_bg.js")
        set(BASE_LIBRARY_WASM "${BASE_PKG_PREFIX_WITH_BUILD_TYPE}/PosemeshBase_bg.wasm")
    else()
        message(FATAL_ERROR "TODO") # TODO: this needs to be implemented
    endif()

    if(NOT EXISTS "${BASE_INCLUDE_DIR}" OR NOT IS_DIRECTORY "${BASE_INCLUDE_DIR}")
        message(FATAL_ERROR "Posemesh Base library is not built for targeted platform, architecture and configuration (build type): Includes directory is missing.")
    endif()
    if(EMSCRIPTEN)
        if(NOT EXISTS "${BASE_LIBRARY_JS}" OR IS_DIRECTORY "${BASE_LIBRARY_JS}")
            message(FATAL_ERROR "Posemesh Base library is not built for targeted platform, architecture and configuration (build type): JavaScript file is missing.")
        endif()
        if(NOT EXISTS "${BASE_LIBRARY_WASM}" OR IS_DIRECTORY "${BASE_LIBRARY_WASM}")
            message(FATAL_ERROR "Posemesh Base library is not built for targeted platform, architecture and configuration (build type): WebAssembly file is missing.")
        endif()
    else()
        if(NOT EXISTS "${BASE_LIBRARY}" OR IS_DIRECTORY "${BASE_LIBRARY}")
            message(FATAL_ERROR "Posemesh Base library is not built for targeted platform, architecture and configuration (build type): Archive file is missing.")
        endif()
    endif()

    target_include_directories(
        ${NAME}
        PRIVATE
            "${BASE_INCLUDE_DIR}"
    )
    if(EMSCRIPTEN)
        copy_file_with_text_replace(
            "${BASE_LIBRARY_JS}"
            "${CMAKE_CURRENT_BINARY_DIR}/PosemeshBase_TextReplaced.js"
            REPLACES
                "let wasm" "let wasm = undefined, wasmImports = { \"./PosemeshBase_bg.js\": {} }, regClsFuncs = []"
                "|REGEX|export[ \\t]+function[ \\t]+([a-zA-Z_][a-zA-Z0-9_]*)[ \\t]*\\(" "function \\1(...args) { return wasmImports[\"./PosemeshBase_bg.js\"].\\1(...args) }\nwasmImports[\"./PosemeshBase_bg.js\"].\\1 = function("
                "|REGEX|export[ \\t]+class[ \\t]+([a-zA-Z_][a-zA-Z0-9_]*)[ \\t]*\\{" "regClsFuncs.push(function() { wasmImports[\"./PosemeshBase_bg.js\"].\\1 = \\1 })\nclass \\1 {"
        )
        install(
            FILES
                "${BASE_LIBRARY_WASM}"
            DESTINATION "${CMAKE_INSTALL_PREFIX}"
            RENAME "PosemeshBase.wasm"
        )
    else()
        if(APPLE)
            target_link_libraries(
                ${NAME}
                PRIVATE
                    "-framework SystemConfiguration"
            )
        endif()
        link_platform_libraries(
            ${NAME}
            HIDE_SYMBOLS
            PRIVATE
                "${BASE_LIBRARY}"
        )
    endif()
endfunction()
