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

    if(NOT ARG_HIDE_SYMBOLS)
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

    if(APPLE)
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
            target_link_options(
                ${NAME}
                PUBLIC
                    "SHELL:-Wl,-hidden-l,\"${LIBRARY_PATH}\""
            )
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
            target_link_options(
                ${NAME}
                INTERFACE
                    "SHELL:-Wl,-hidden-l,\"${LIBRARY_PATH}\""
            )
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
            target_link_options(
                ${NAME}
                PRIVATE
                    "SHELL:-Wl,-hidden-l,\"${LIBRARY_PATH}\""
            )
        endforeach()
    else()
        message(FATAL_ERROR "TODO") # TODO: this needs to be implemented
    endif()
endfunction()
