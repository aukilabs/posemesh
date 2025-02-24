include("${CMAKE_CURRENT_LIST_DIR}/CopyFileWithTextReplace.cmake")
include("${CMAKE_CURRENT_LIST_DIR}/GetRustTargetName.cmake")
include("${CMAKE_CURRENT_LIST_DIR}/LinkPlatformLibraries.cmake")

set(DOMAIN_PREFIX "${CMAKE_CURRENT_LIST_DIR}/../../core")
set(DOMAIN_PKG_PREFIX "${DOMAIN_PREFIX}/pkg/domain")
set(DOMAIN_TARGET_PREFIX "${DOMAIN_PREFIX}/target")

function(LINK_DOMAIN_LIBRARY NAME)
    if(NOT TARGET ${NAME})
        message(FATAL_ERROR "Target '${NAME}' does not exist.")
    endif()

    get_rust_target_name(RUST_TARGET_NAME)
    set(DOMAIN_TARGET_DIR "${DOMAIN_TARGET_PREFIX}/${RUST_TARGET_NAME}")
    if("${CMAKE_BUILD_TYPE}" STREQUAL "Debug")
        set(DOMAIN_PKG_PREFIX_WITH_BUILD_TYPE "${DOMAIN_PKG_PREFIX}/Debug")
        set(DOMAIN_TARGET_DIR_WITH_BUILD_TYPE "${DOMAIN_TARGET_DIR}/debug")
    else()
        set(DOMAIN_PKG_PREFIX_WITH_BUILD_TYPE "${DOMAIN_PKG_PREFIX}/Release")
        set(DOMAIN_TARGET_DIR_WITH_BUILD_TYPE "${DOMAIN_TARGET_DIR}/release")
    endif()

    set(DOMAIN_INCLUDE_DIR "${DOMAIN_PREFIX}/include")

    if(APPLE)
        set(DOMAIN_LIBRARY "${DOMAIN_TARGET_DIR_WITH_BUILD_TYPE}/libdomain_static.a")
    elseif(EMSCRIPTEN)
        set(DOMAIN_LIBRARY_JS "${DOMAIN_PKG_PREFIX_WITH_BUILD_TYPE}/PosemeshDomain.js")
        set(DOMAIN_LIBRARY_WASM "${DOMAIN_PKG_PREFIX_WITH_BUILD_TYPE}/PosemeshDomain_bg.wasm")
    else()
        message(FATAL_ERROR "TODO") # TODO: this needs to be implemented
    endif()

    if(NOT EXISTS "${DOMAIN_INCLUDE_DIR}" OR NOT IS_DIRECTORY "${DOMAIN_INCLUDE_DIR}")
        message(FATAL_ERROR "Posemesh Domain library is not built for targeted platform, architecture and configuration (build type): Includes directory is missing.")
    endif()
    if(EMSCRIPTEN)
        if(NOT EXISTS "${DOMAIN_LIBRARY_JS}" OR IS_DIRECTORY "${DOMAIN_LIBRARY_JS}")
            message(FATAL_ERROR "Posemesh Domain library is not built for targeted platform, architecture and configuration (build type): JavaScript file is missing.")
        endif()
        if(NOT EXISTS "${DOMAIN_LIBRARY_WASM}" OR IS_DIRECTORY "${DOMAIN_LIBRARY_WASM}")
            message(FATAL_ERROR "Posemesh Domain library is not built for targeted platform, architecture and configuration (build type): WebAssembly file is missing.")
        endif()
    else()
        if(NOT EXISTS "${DOMAIN_LIBRARY}" OR IS_DIRECTORY "${DOMAIN_LIBRARY}")
            message(FATAL_ERROR "Posemesh Domain library is not built for targeted platform, architecture and configuration (build type): Archive file is missing.")
        endif()
    endif()

    target_include_directories(
        ${NAME}
        PRIVATE
            "${DOMAIN_INCLUDE_DIR}"
    )
    if(EMSCRIPTEN)
        copy_file_with_text_replace(
            "${DOMAIN_LIBRARY_JS}"
            "${CMAKE_CURRENT_BINARY_DIR}/PosemeshDomain_TextReplaced.js"
            REPLACES
                "|MATCH-WORD|wasm_bindgen" "__internalPosemeshDomain"
                "script_src.replace(/\\.js$/, '_bg.wasm')" "'PosemeshDomain.wasm'"
                "console.warn('using deprecated parameters for the initialization function" "// console.warn('using deprecated parameters for the initialization function" # HACK
        )
        install(
            FILES
                "${DOMAIN_LIBRARY_WASM}"
            DESTINATION "${CMAKE_INSTALL_PREFIX}"
            RENAME "PosemeshDomain.wasm"
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
                "${DOMAIN_LIBRARY}"
        )
    endif()
endfunction()
