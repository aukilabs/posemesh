find_program(CLANG_FORMAT_EXECUTABLE clang-format DOC "Path of the Clang-Format executable")
mark_as_advanced(CLANG_FORMAT_EXECUTABLE)

include(FindPackageHandleStandardArgs)
find_package_handle_standard_args(ClangFormat REQUIRED_VARS CLANG_FORMAT_EXECUTABLE)
set(CLANG_FORMAT_FOUND ${ClangFormat_FOUND})

if(CLANG_FORMAT_FOUND)
    if(TARGET ClangFormat)
        get_target_property(INTERNAL_CLANG_FORMAT_FLAG ClangFormat INTERNAL_CLANG_FORMAT_FLAG)
        if(NOT INTERNAL_CLANG_FORMAT_FLAG OR NOT "${INTERNAL_CLANG_FORMAT_FLAG}" STREQUAL "3bcc1c04-59cd-4fb3-b8ab-4b7a22e79a8a")
            message(FATAL_ERROR "ClangFormat target was added outside of the find module.")
        endif()
        unset(INTERNAL_CLANG_FORMAT_FLAG)
    else()
        define_property(TARGET PROPERTY INTERNAL_CLANG_FORMAT_FLAG)
        add_executable(ClangFormat IMPORTED GLOBAL)
        set_target_properties(ClangFormat PROPERTIES INTERNAL_CLANG_FORMAT_FLAG "3bcc1c04-59cd-4fb3-b8ab-4b7a22e79a8a")
    endif()
    set_target_properties(ClangFormat PROPERTIES IMPORTED_LOCATION "${CLANG_FORMAT_EXECUTABLE}")

    function(ADD_CLANG_FORMAT_TARGET NAME ORIGINAL_TARGET)
        if(TARGET ${NAME})
            message(FATAL_ERROR "Target '${NAME}' already exists.")
        endif()
        if(NOT TARGET ${ORIGINAL_TARGET})
            message(FATAL_ERROR "Target '${ORIGINAL_TARGET}' does not exist.")
        endif()

        get_target_property(ORIGINAL_SOURCES ${ORIGINAL_TARGET} SOURCES)
        list(APPEND ORIGINAL_SOURCES_FILTERED)
        foreach(ORIGINAL_SOURCE ${ORIGINAL_SOURCES})
            if("${ORIGINAL_SOURCE}" MATCHES "\.([cC][cC]?|[cC][pP][pP]|[cC][xX][xX]|[hH][hH]?|[hH][pP][pP]|[hH][xX][xX]|[mM][mM]?)$")
                list(APPEND ORIGINAL_SOURCES_FILTERED "${ORIGINAL_SOURCE}")
            endif()
        endforeach()

        add_custom_target(
            ${NAME}
            ${CLANG_FORMAT_EXECUTABLE} -i ${ORIGINAL_SOURCES_FILTERED}
            WORKING_DIRECTORY "${CMAKE_CURRENT_LIST_DIR}"
            COMMENT "Apply the source code formatting style using Clang-Format"
            VERBATIM
        )
    endfunction()
endif()
