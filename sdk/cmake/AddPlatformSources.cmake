function(ADD_PLATFORM_SOURCES NAME)
    list(APPEND OPTION_KEYWORDS)
    list(
        APPEND SINGLE_VALUE_KEYWORDS
            PUBLIC_HEADER_DIR
    )
    list(
        APPEND MULTI_VALUE_KEYWORDS
            PUBLIC_C_HEADERS
            PUBLIC_CXX_HEADERS
            PUBLIC_OBJC_HEADERS
            PUBLIC_OBJCXX_HEADERS
            C_HEADERS
            CXX_HEADERS
            OBJC_HEADERS
            OBJCXX_HEADERS
            C_SOURCES
            CXX_SOURCES
            OBJC_SOURCES
            OBJCXX_SOURCES
            Swift_SOURCES
    )
    cmake_parse_arguments(ARG "${OPTION_KEYWORDS}" "${SINGLE_VALUE_KEYWORDS}" "${MULTI_VALUE_KEYWORDS}" ${ARGN})
    if(DEFINED ARG_UNPARSED_ARGUMENTS)
        list(GET ARG_UNPARSED_ARGUMENTS 0 FIRST_UNKNOWN_KEYWORD)
        message(FATAL_ERROR "Unknown '${FIRST_UNKNOWN_KEYWORD}' keyword.")
    endif()

    if(NOT TARGET ${NAME})
        message(FATAL_ERROR "Target '${NAME}' does not exist.")
    endif()

    list(APPEND PUBLIC_HEADERS)
    list(APPEND C_SOURCES)
    list(APPEND CXX_SOURCES)
    list(APPEND OBJC_SOURCES)
    list(APPEND OBJCXX_SOURCES)
    list(APPEND Swift_SOURCES)
    list(APPEND SOURCES)

    foreach(LANGUAGE C CXX OBJC OBJCXX Swift)
        if(DEFINED ARG_PUBLIC_${LANGUAGE}_HEADERS)
            if(NOT APPLE AND "${LANGUAGE}" MATCHES "^(OBJC|OBJCXX)$")
                message(FATAL_ERROR "Language '${LANGUAGE}' is exclusive to Apple platforms.")
            endif()
            foreach(SOURCE ${ARG_PUBLIC_${LANGUAGE}_HEADERS})
                file(REAL_PATH "${SOURCE}" SOURCE_ABSOLUTE BASE_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
                if(NOT EXISTS "${SOURCE_ABSOLUTE}")
                    message(FATAL_ERROR "File '${SOURCE_ABSOLUTE}' does not exist.")
                endif()
                if(IS_DIRECTORY "${SOURCE_ABSOLUTE}")
                    message(FATAL_ERROR "File '${SOURCE_ABSOLUTE}' is a directory.")
                endif()
                if("${SOURCE_ABSOLUTE}" IN_LIST SOURCES)
                    message(FATAL_ERROR "File '${SOURCE_ABSOLUTE}' was specified multiple times.")
                endif()
                list(APPEND PUBLIC_HEADERS "${SOURCE_ABSOLUTE}")
                list(APPEND SOURCES "${SOURCE_ABSOLUTE}")
                list(APPEND ${LANGUAGE}_SOURCES "${SOURCE_ABSOLUTE}")
            endforeach()
        endif()

        if(DEFINED ARG_${LANGUAGE}_HEADERS)
            if(NOT APPLE AND "${LANGUAGE}" MATCHES "^(OBJC|OBJCXX)$")
                message(FATAL_ERROR "Language '${LANGUAGE}' is exclusive to Apple platforms.")
            endif()
            foreach(SOURCE ${ARG_${LANGUAGE}_HEADERS})
                file(REAL_PATH "${SOURCE}" SOURCE_ABSOLUTE BASE_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
                if(NOT EXISTS "${SOURCE_ABSOLUTE}")
                    message(FATAL_ERROR "File '${SOURCE_ABSOLUTE}' does not exist.")
                endif()
                if(IS_DIRECTORY "${SOURCE_ABSOLUTE}")
                    message(FATAL_ERROR "File '${SOURCE_ABSOLUTE}' is a directory.")
                endif()
                if("${SOURCE_ABSOLUTE}" IN_LIST SOURCES)
                    message(FATAL_ERROR "File '${SOURCE_ABSOLUTE}' was specified multiple times.")
                endif()
                list(APPEND SOURCES "${SOURCE_ABSOLUTE}")
                list(APPEND ${LANGUAGE}_SOURCES "${SOURCE_ABSOLUTE}")
            endforeach()
        endif()

        if(DEFINED ARG_${LANGUAGE}_SOURCES)
            if(NOT APPLE AND "${LANGUAGE}" MATCHES "^(OBJC|OBJCXX|Swift)$")
                message(FATAL_ERROR "Language '${LANGUAGE}' is exclusive to Apple platforms.")
            endif()
            foreach(SOURCE ${ARG_${LANGUAGE}_SOURCES})
                file(REAL_PATH "${SOURCE}" SOURCE_ABSOLUTE BASE_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
                if(NOT EXISTS "${SOURCE_ABSOLUTE}")
                    message(FATAL_ERROR "File '${SOURCE_ABSOLUTE}' does not exist.")
                endif()
                if(IS_DIRECTORY "${SOURCE_ABSOLUTE}")
                    message(FATAL_ERROR "File '${SOURCE_ABSOLUTE}' is a directory.")
                endif()
                if("${SOURCE_ABSOLUTE}" IN_LIST SOURCES)
                    message(FATAL_ERROR "File '${SOURCE_ABSOLUTE}' was specified multiple times.")
                endif()
                list(APPEND SOURCES "${SOURCE_ABSOLUTE}")
                list(APPEND ${LANGUAGE}_SOURCES "${SOURCE_ABSOLUTE}")
            endforeach()
        endif()
    endforeach()

    if(NOT SOURCES)
        message(FATAL_ERROR "No source files specified.")
    endif()
    target_sources(${NAME} PRIVATE ${SOURCES})

    if(DEFINED ARG_PUBLIC_HEADER_DIR)
        file(REAL_PATH "${ARG_PUBLIC_HEADER_DIR}" PUBLIC_HEADER_DIR_ABSOLUTE BASE_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
        if(NOT EXISTS "${PUBLIC_HEADER_DIR_ABSOLUTE}")
            message(FATAL_ERROR "Directory '${PUBLIC_HEADER_DIR_ABSOLUTE}' does not exist.")
        endif()
        if(NOT IS_DIRECTORY "${PUBLIC_HEADER_DIR_ABSOLUTE}")
            message(FATAL_ERROR "Directory '${PUBLIC_HEADER_DIR_ABSOLUTE}' is a file.")
        endif()
        foreach(PUBLIC_HEADER ${PUBLIC_HEADERS})
            if(NOT "${PUBLIC_HEADER}" MATCHES "^${PUBLIC_HEADER_DIR_ABSOLUTE}(/|\\|$)")
                message(FATAL_ERROR "Public header '${PUBLIC_HEADER}' is located outside of the specified '${PUBLIC_HEADER_DIR_ABSOLUTE}' public header directory.")
            endif()
        endforeach()
        if(APPLE)
            target_include_directories(
                ${NAME}
                PRIVATE
                    "${PUBLIC_HEADER_DIR_ABSOLUTE}"
            )
        else()
            target_include_directories(
                ${NAME}
                PUBLIC
                    "${PUBLIC_HEADER_DIR_ABSOLUTE}"
            )
        endif()
    elseif(PUBLIC_HEADERS)
        message(FATAL_ERROR "Public header directory must be specified using 'PUBLIC_HEADER_DIR' when using public headers.")
    endif()

    foreach(LANGUAGE C CXX OBJC OBJCXX Swift)
        if(${LANGUAGE}_SOURCES)
            set_source_files_properties(
                ${LANGUAGE}_SOURCES
                TARGET_DIRECTORY ${NAME}
                PROPERTIES
                    LANGUAGE ${LANGUAGE}
            )
        endif()
    endforeach()

    if(LINUX)
        foreach(PUBLIC_HEADER ${PUBLIC_HEADERS})
            file(RELATIVE_PATH PUBLIC_HEADER_RELATIVE "${PUBLIC_HEADER_DIR_ABSOLUTE}" "${PUBLIC_HEADER}")
            get_filename_component(PUBLIC_HEADER_PREFIX "${PUBLIC_HEADER_RELATIVE}" DIRECTORY)
            install(
                FILES
                    ${PUBLIC_HEADER}
                DESTINATION "${CMAKE_INSTALL_PREFIX}/include/${PUBLIC_HEADER_PREFIX}"
            )
        endforeach()
    elseif(APPLE)
        foreach(PUBLIC_HEADER ${PUBLIC_HEADERS})
            set(PUBLIC_HEADER_DIR_APPLE "${PUBLIC_HEADER_DIR_ABSOLUTE}/${NAME}")
            if(NOT EXISTS "${PUBLIC_HEADER_DIR_APPLE}")
                message(FATAL_ERROR "Directory '${PUBLIC_HEADER_DIR_APPLE}' does not exist.")
            endif()
            if(NOT IS_DIRECTORY "${PUBLIC_HEADER_DIR_APPLE}")
                message(FATAL_ERROR "Directory '${PUBLIC_HEADER_DIR_APPLE}' is a file.")
            endif()
            file(RELATIVE_PATH PUBLIC_HEADER_RELATIVE "${PUBLIC_HEADER_DIR_APPLE}" "${PUBLIC_HEADER}")
            get_filename_component(PUBLIC_HEADER_PREFIX "${PUBLIC_HEADER_RELATIVE}" DIRECTORY)
            set_source_files_properties(
                "${PUBLIC_HEADER}"
                TARGET_DIRECTORY ${NAME}
                PROPERTIES
                    MACOSX_PACKAGE_LOCATION "Headers/${PUBLIC_HEADER_PREFIX}"
            )
        endforeach()
    elseif(EMSCRIPTEN)
        # Do nothing
    else()
        # TODO: install public headers
    endif()
endfunction()
