function(LINK_PLATFORM_LIBRARIES NAME)
    list(
        APPEND OPTION_KEYWORDS
            HIDE_SYMBOLS
    )
    list(APPEND SINGLE_VALUE_KEYWORDS)
    list(
        APPEND MULTI_VALUE_KEYWORDS
            PUBLIC
            INTERFACE
            PRIVATE
    )
    cmake_parse_arguments(ARG "${OPTION_KEYWORDS}" "${SINGLE_VALUE_KEYWORDS}" "${MULTI_VALUE_KEYWORDS}" ${ARGN})
    if(DEFINED ARG_UNPARSED_ARGUMENTS)
        list(GET ARG_UNPARSED_ARGUMENTS 0 FIRST_UNKNOWN_KEYWORD)
        message(FATAL_ERROR "Unknown '${FIRST_UNKNOWN_KEYWORD}' keyword.")
    endif()

    if(NOT TARGET ${NAME})
        message(FATAL_ERROR "Target '${NAME}' does not exist.")
    endif()

    if(NOT ARG_HIDE_SYMBOLS OR EMSCRIPTEN)
        if(ARG_PUBLIC)
            target_link_libraries(
                ${NAME}
                PUBLIC
                    ${ARG_PUBLIC}
            )
        endif()
        if(ARG_INTERFACE)
            target_link_libraries(
                ${NAME}
                INTERFACE
                    ${ARG_INTERFACE}
            )
        endif()
        if(ARG_PRIVATE)
            target_link_libraries(
                ${NAME}
                PRIVATE
                    ${ARG_PRIVATE}
            )
        endif()
        return()
    endif()

    if(LINUX OR APPLE)
        foreach(LIBRARY ${ARG_PUBLIC})
            if(TARGET "${LIBRARY}")
                get_target_property(LIBRARY_TYPE ${LIBRARY} TYPE)
                if(NOT "${LIBRARY_TYPE}" STREQUAL "STATIC_LIBRARY")
                    message(FATAL_ERROR "Library '${LIBRARY}' is not a static library.")
                endif()
                get_target_property(LIBRARY_PATH ${LIBRARY} LOCATION)
                add_dependencies(${NAME} ${LIBRARY})
            else()
                file(REAL_PATH "${LIBRARY}" LIBRARY_PATH BASE_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
                if(NOT EXISTS "${LIBRARY_PATH}")
                    message(FATAL_ERROR "Library '${LIBRARY_PATH}' does not exist.")
                endif()
                if(IS_DIRECTORY "${LIBRARY_PATH}")
                    message(FATAL_ERROR "Library '${LIBRARY_PATH}' is a directory.")
                endif()
            endif()
            get_filename_component(LIBRARY_DIR "${LIBRARY_PATH}" DIRECTORY)
            get_filename_component(LIBRARY_FILE_NAME_WLE "${LIBRARY_PATH}" NAME_WLE)
            if("${LIBRARY_FILE_NAME_WLE}" MATCHES "^lib")
                string(SUBSTRING "${LIBRARY_FILE_NAME_WLE}" 3 -1 LIBRARY_NAME)
            else()
                set(LIBRARY_NAME "${LIBRARY_FILE_NAME_WLE}")
            endif()
            if(LINUX)
                target_link_options(
                    ${NAME}
                    PUBLIC
                        "SHELL:-L\"${LIBRARY_DIR}\" -l${LIBRARY_NAME} -Wl,--exclude-libs,${LIBRARY_FILE_NAME_WLE}.a"
                )
            elseif(APPLE)
                target_link_options(
                    ${NAME}
                    PUBLIC
                        "SHELL:-L\"${LIBRARY_DIR}\" -Wl,-hidden-l${LIBRARY_NAME}"
                )
            else()
                message(FATAL_ERROR "Missing logic.")
            endif()
        endforeach()

        foreach(LIBRARY ${ARG_INTERFACE})
            if(TARGET "${LIBRARY}")
                get_target_property(LIBRARY_TYPE ${LIBRARY} TYPE)
                if(NOT "${LIBRARY_TYPE}" STREQUAL "STATIC_LIBRARY")
                    message(FATAL_ERROR "Library '${LIBRARY}' is not a static library.")
                endif()
                get_target_property(LIBRARY_PATH ${LIBRARY} LOCATION)
                add_dependencies(${NAME} ${LIBRARY})
            else()
                file(REAL_PATH "${LIBRARY}" LIBRARY_PATH BASE_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
                if(NOT EXISTS "${LIBRARY_PATH}")
                    message(FATAL_ERROR "Library '${LIBRARY_PATH}' does not exist.")
                endif()
                if(IS_DIRECTORY "${LIBRARY_PATH}")
                    message(FATAL_ERROR "Library '${LIBRARY_PATH}' is a directory.")
                endif()
            endif()
            get_filename_component(LIBRARY_DIR "${LIBRARY_PATH}" DIRECTORY)
            get_filename_component(LIBRARY_FILE_NAME_WLE "${LIBRARY_PATH}" NAME_WLE)
            if("${LIBRARY_FILE_NAME_WLE}" MATCHES "^lib")
                string(SUBSTRING "${LIBRARY_FILE_NAME_WLE}" 3 -1 LIBRARY_NAME)
            else()
                set(LIBRARY_NAME "${LIBRARY_FILE_NAME_WLE}")
            endif()
            if(LINUX)
                target_link_options(
                    ${NAME}
                    INTERFACE
                        "SHELL:-L\"${LIBRARY_DIR}\" -l${LIBRARY_NAME} -Wl,--exclude-libs,${LIBRARY_FILE_NAME_WLE}.a"
                )
            elseif(APPLE)
                target_link_options(
                    ${NAME}
                    INTERFACE
                        "SHELL:-L\"${LIBRARY_DIR}\" -Wl,-hidden-l${LIBRARY_NAME}"
                )
            else()
                message(FATAL_ERROR "Missing logic.")
            endif()
        endforeach()

        foreach(LIBRARY ${ARG_PRIVATE})
            if(TARGET "${LIBRARY}")
                get_target_property(LIBRARY_TYPE ${LIBRARY} TYPE)
                if(NOT "${LIBRARY_TYPE}" STREQUAL "STATIC_LIBRARY")
                    message(FATAL_ERROR "Library '${LIBRARY}' is not a static library.")
                endif()
                get_target_property(LIBRARY_PATH ${LIBRARY} LOCATION)
                add_dependencies(${NAME} ${LIBRARY})
            else()
                file(REAL_PATH "${LIBRARY}" LIBRARY_PATH BASE_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
                if(NOT EXISTS "${LIBRARY_PATH}")
                    message(FATAL_ERROR "Library '${LIBRARY_PATH}' does not exist.")
                endif()
                if(IS_DIRECTORY "${LIBRARY_PATH}")
                    message(FATAL_ERROR "Library '${LIBRARY_PATH}' is a directory.")
                endif()
            endif()
            get_filename_component(LIBRARY_DIR "${LIBRARY_PATH}" DIRECTORY)
            get_filename_component(LIBRARY_FILE_NAME_WLE "${LIBRARY_PATH}" NAME_WLE)
            if("${LIBRARY_FILE_NAME_WLE}" MATCHES "^lib")
                string(SUBSTRING "${LIBRARY_FILE_NAME_WLE}" 3 -1 LIBRARY_NAME)
            else()
                set(LIBRARY_NAME "${LIBRARY_FILE_NAME_WLE}")
            endif()
            if(LINUX)
                target_link_options(
                    ${NAME}
                    PRIVATE
                        "SHELL:-L\"${LIBRARY_DIR}\" -l${LIBRARY_NAME} -Wl,--exclude-libs,${LIBRARY_FILE_NAME_WLE}.a"
                )
            elseif(APPLE)
                target_link_options(
                    ${NAME}
                    PRIVATE
                        "SHELL:-L\"${LIBRARY_DIR}\" -Wl,-hidden-l${LIBRARY_NAME}"
                )
            else()
                message(FATAL_ERROR "Missing logic.")
            endif()
        endforeach()
    else()
        message(FATAL_ERROR "TODO") # TODO: this needs to be implemented
    endif()
endfunction()
