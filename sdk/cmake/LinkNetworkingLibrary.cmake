include("${CMAKE_CURRENT_LIST_DIR}/CopyFileWithTextReplace.cmake")
include("${CMAKE_CURRENT_LIST_DIR}/GetRustTargetName.cmake")
include("${CMAKE_CURRENT_LIST_DIR}/LinkPlatformLibraries.cmake")

set(NETWORKING_PREFIX "${CMAKE_CURRENT_LIST_DIR}/../../core/networking")
set(NETWORKING_PKG_PREFIX "${NETWORKING_PREFIX}/pkg")
set(NETWORKING_TARGET_PREFIX "${NETWORKING_PREFIX}/target")

function(LINK_NETWORKING_LIBRARY NAME)
    if(NOT TARGET ${NAME})
        message(FATAL_ERROR "Target '${NAME}' does not exist.")
    endif()

    get_rust_target_name(RUST_TARGET_NAME)
    set(NETWORKING_TARGET_DIR "${NETWORKING_TARGET_PREFIX}/${RUST_TARGET_NAME}")
    if("${CMAKE_BUILD_TYPE}" STREQUAL "Debug")
        set(NETWORKING_PKG_PREFIX_WITH_BUILD_TYPE "${NETWORKING_PKG_PREFIX}/Debug")
        set(NETWORKING_TARGET_DIR_WITH_BUILD_TYPE "${NETWORKING_TARGET_DIR}/debug")
    else()
        set(NETWORKING_PKG_PREFIX_WITH_BUILD_TYPE "${NETWORKING_PKG_PREFIX}/Release")
        set(NETWORKING_TARGET_DIR_WITH_BUILD_TYPE "${NETWORKING_TARGET_DIR}/release")
    endif()

    set(NETWORKING_INCLUDE_DIR "${NETWORKING_PREFIX}/include")

    if(APPLE)
        set(NETWORKING_LIBRARY "${NETWORKING_TARGET_DIR_WITH_BUILD_TYPE}/libposemesh_networking_static.a")
    elseif(EMSCRIPTEN)
        set(NETWORKING_LIBRARY_JS "${NETWORKING_PKG_PREFIX_WITH_BUILD_TYPE}/PosemeshNetworking.js")
        set(NETWORKING_LIBRARY_WASM "${NETWORKING_PKG_PREFIX_WITH_BUILD_TYPE}/PosemeshNetworking_bg.wasm")
    else()
        message(FATAL_ERROR "TODO") # TODO: this needs to be implemented
    endif()

    if(NOT EXISTS "${NETWORKING_INCLUDE_DIR}" OR NOT IS_DIRECTORY "${NETWORKING_INCLUDE_DIR}")
        message(FATAL_ERROR "Posemesh Networking library is not built for targeted platform, architecture and configuration (build type): Includes directory is missing.")
    endif()
    if(EMSCRIPTEN)
        if(NOT EXISTS "${NETWORKING_LIBRARY_JS}" OR IS_DIRECTORY "${NETWORKING_LIBRARY_JS}")
            message(FATAL_ERROR "Posemesh Networking library is not built for targeted platform, architecture and configuration (build type): JavaScript file is missing.")
        endif()
        if(NOT EXISTS "${NETWORKING_LIBRARY_WASM}" OR IS_DIRECTORY "${NETWORKING_LIBRARY_WASM}")
            message(FATAL_ERROR "Posemesh Networking library is not built for targeted platform, architecture and configuration (build type): WebAssembly file is missing.")
        endif()
    else()
        if(NOT EXISTS "${NETWORKING_LIBRARY}" OR IS_DIRECTORY "${NETWORKING_LIBRARY}")
            message(FATAL_ERROR "Posemesh Networking library is not built for targeted platform, architecture and configuration (build type): Archive file is missing.")
        endif()
    endif()

    target_include_directories(
        ${NAME}
        PRIVATE
            "${NETWORKING_INCLUDE_DIR}"
    )
    if(EMSCRIPTEN)
        copy_file_with_text_replace(
            "${NETWORKING_LIBRARY_JS}"
            "${CMAKE_CURRENT_BINARY_DIR}/PosemeshNetworking_TextReplaced.js"
            REPLACES
                "|MATCH-WORD|wasm_bindgen" "__internalPosemeshNetworking"
                "script_src.replace(/\\.js$/, '_bg.wasm')" "'PosemeshNetworking.wasm'"
                "console.warn('using deprecated parameters for the initialization function" "// console.warn('using deprecated parameters for the initialization function" # HACK
        )
        install(
            FILES
                "${NETWORKING_LIBRARY_WASM}"
            DESTINATION "${CMAKE_INSTALL_PREFIX}"
            RENAME "PosemeshNetworking.wasm"
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
                "${NETWORKING_LIBRARY}"
        )
    endif()
endfunction()
