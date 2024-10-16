function(SET_APPLE_PLATFORM_UMBRELLA_HEADER NAME UMBRELLA_HEADER MODULEMAP_FILE)
    if(NOT APPLE)
        message(FATAL_ERROR "Objective-C to Swift umbrella header is only available on Apple platforms.")
    endif()

    if(NOT TARGET ${NAME})
        message(FATAL_ERROR "Target '${NAME}' does not exist.")
    endif()

    file(REAL_PATH "${UMBRELLA_HEADER}" UMBRELLA_HEADER_ABSOLUTE BASE_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
    if(NOT EXISTS "${UMBRELLA_HEADER_ABSOLUTE}")
        message(FATAL_ERROR "File '${UMBRELLA_HEADER_ABSOLUTE}' does not exist.")
    endif()
    if(IS_DIRECTORY "${UMBRELLA_HEADER_ABSOLUTE}")
        message(FATAL_ERROR "File '${UMBRELLA_HEADER_ABSOLUTE}' is a directory.")
    endif()

    file(REAL_PATH "${MODULEMAP_FILE}" MODULEMAP_FILE_ABSOLUTE BASE_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
    if(NOT EXISTS "${MODULEMAP_FILE_ABSOLUTE}")
        message(FATAL_ERROR "File '${MODULEMAP_FILE_ABSOLUTE}' does not exist.")
    endif()
    if(IS_DIRECTORY "${MODULEMAP_FILE_ABSOLUTE}")
        message(FATAL_ERROR "File '${MODULEMAP_FILE_ABSOLUTE}' is a directory.")
    endif()

    get_target_property(CURRENT_MODULEMAP_FILE ${NAME} XCODE_ATTRIBUTE_MODULEMAP_FILE)
    if(CURRENT_MODULEMAP_FILE)
        message(FATAL_ERROR "Objective-C to Swift umbrella header was already set to '${CURRENT_MODULEMAP_FILE}' file previously.")
    endif()

    get_target_property(CURRENT_SWIFT_OBJC_BRIDGING_HEADER ${NAME} XCODE_ATTRIBUTE_SWIFT_OBJC_BRIDGING_HEADER)
    if(CURRENT_SWIFT_OBJC_BRIDGING_HEADER)
        message(FATAL_ERROR "Objective-C to Swift umbrella header cannot be set because the bridging header was already set.")
    endif()

    target_sources(
        ${NAME}
        PRIVATE
            "${UMBRELLA_HEADER_ABSOLUTE}"
            "${MODULEMAP_FILE_ABSOLUTE}"
    )
    set_source_files_properties(
        "${UMBRELLA_HEADER_ABSOLUTE}"
        TARGET_DIRECTORY ${NAME}
        PROPERTIES
            MACOSX_PACKAGE_LOCATION "Headers"
    )

    set_target_properties(
        ${NAME}
        PROPERTIES
            XCODE_ATTRIBUTE_CLANG_ENABLE_MODULES YES
            XCODE_ATTRIBUTE_DEFINES_MODULE YES
            XCODE_ATTRIBUTE_MODULEMAP_FILE "${MODULEMAP_FILE_ABSOLUTE}"
    )
endfunction()
