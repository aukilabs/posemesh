include("${CMAKE_CURRENT_LIST_DIR}/GetRustTargetName.cmake")
include("${CMAKE_CURRENT_LIST_DIR}/LinkPlatformLibraries.cmake")

set(NETWORKING_PREFIX "${CMAKE_CURRENT_LIST_DIR}/../../networking")
set(NETWORKING_TARGET_PREFIX "${NETWORKING_PREFIX}/target")

function(LINK_NETWORKING_LIBRARY NAME)
    if(NOT TARGET ${NAME})
        message(FATAL_ERROR "Target '${NAME}' does not exist.")
    endif()

    get_rust_target_name(RUST_TARGET_NAME)
    set(NETWORKING_TARGET_DIR "${NETWORKING_TARGET_PREFIX}/${RUST_TARGET_NAME}")

    if(EMSCRIPTEN)
        set(NETWORKING_INCLUDE_DIR "${NETWORKING_PREFIX}/include")
    else()
        set(NETWORKING_INCLUDE_DIR "${NETWORKING_TARGET_DIR}/cxxbridge")
    endif()

    if(APPLE OR EMSCRIPTEN)
        if("${CMAKE_BUILD_TYPE}" STREQUAL "Debug")
            set(NETWORKING_LIBRARY "${NETWORKING_TARGET_DIR}/debug/libposemesh_networking.a")
        else()
            set(NETWORKING_LIBRARY "${NETWORKING_TARGET_DIR}/release/libposemesh_networking.a")
        endif()
    else()
        message(FATAL_ERROR "TODO") # TODO: this needs to be implemented
    endif()

    if(NOT EXISTS "${NETWORKING_INCLUDE_DIR}" OR NOT IS_DIRECTORY "${NETWORKING_INCLUDE_DIR}" OR NOT EXISTS "${NETWORKING_LIBRARY}" OR IS_DIRECTORY "${NETWORKING_LIBRARY}")
        message(FATAL_ERROR "Posemesh Networking library is not built for targeted platform, architecture and configuration (build type).")
    endif()

    target_include_directories(
        ${NAME}
        PRIVATE
            "${NETWORKING_INCLUDE_DIR}"
    )
    link_platform_libraries(
        ${NAME}
        HIDE_SYMBOLS
        PRIVATE
            "${NETWORKING_LIBRARY}"
    )
endfunction()
