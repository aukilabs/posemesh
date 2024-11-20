function(SET_APPLE_INFO NAME INFO_PLIST_PATH)
    list(
        APPEND OPTION_KEYWORDS
            CONFIGURE
            @ONLY
    )
    list(APPEND SINGLE_VALUE_KEYWORDS)
    list(APPEND MULTI_VALUE_KEYWORDS)
    cmake_parse_arguments(ARG "${OPTION_KEYWORDS}" "${SINGLE_VALUE_KEYWORDS}" "${MULTI_VALUE_KEYWORDS}" ${ARGN})
    if(DEFINED ARG_UNPARSED_ARGUMENTS)
        list(GET ARG_UNPARSED_ARGUMENTS 0 FIRST_UNKNOWN_KEYWORD)
        message(FATAL_ERROR "Unknown '${FIRST_UNKNOWN_KEYWORD}' keyword.")
    endif()

    if(NOT TARGET ${NAME})
        message(FATAL_ERROR "Target '${NAME}' does not exist.")
    endif()

    file(REAL_PATH "${INFO_PLIST_PATH}" INFO_PLIST_PATH_ABSOLUTE BASE_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
    if(NOT EXISTS "${INFO_PLIST_PATH_ABSOLUTE}")
        message(FATAL_ERROR "File '${INFO_PLIST_PATH_ABSOLUTE}' does not exist.")
    endif()
    if(IS_DIRECTORY "${INFO_PLIST_PATH_ABSOLUTE}")
        message(FATAL_ERROR "File '${INFO_PLIST_PATH_ABSOLUTE}' is a directory.")
    endif()

    if(ARG_CONFIGURE)
        set(INFO_PLIST_PATH_ACTUAL "${CMAKE_CURRENT_BINARY_DIR}/${NAME}Info.plist")
        if(ARG_@ONLY)
            configure_file("${INFO_PLIST_PATH_ABSOLUTE}" "${INFO_PLIST_PATH_ACTUAL}" @ONLY)
        else()
            configure_file("${INFO_PLIST_PATH_ABSOLUTE}" "${INFO_PLIST_PATH_ACTUAL}")
        endif()
    elseif(ARG_@ONLY)
        message(FATAL_ERROR "Option keyword '@ONLY' is only available when option keyword 'CONFIGURE' is specified.")
    else()
        set(INFO_PLIST_PATH_ACTUAL "${INFO_PLIST_PATH_ABSOLUTE}")
    endif()

    get_target_property(CURRENT_MACOSX_FRAMEWORK_INFO_PLIST ${NAME} MACOSX_FRAMEWORK_INFO_PLIST)
    if(CURRENT_MACOSX_FRAMEWORK_INFO_PLIST)
        message(FATAL_ERROR "Apple framework Info.plist was already set to '${CURRENT_MACOSX_FRAMEWORK_INFO_PLIST}' file previously.")
    endif()

    set_target_properties(
        ${NAME}
        PROPERTIES
            MACOSX_FRAMEWORK_INFO_PLIST "${INFO_PLIST_PATH_ACTUAL}"
    )
endfunction()
