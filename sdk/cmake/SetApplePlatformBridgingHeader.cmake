include("${CMAKE_CURRENT_LIST_DIR}/AddPlatformSources.cmake")

function(SET_APPLE_PLATFORM_BRIDGING_HEADER NAME BRIDGING_HEADER)
    if(NOT APPLE)
        message(FATAL_ERROR "Objective-C to Swift bridging header is only available on Apple platforms.")
    endif()

    if(NOT TARGET ${NAME})
        message(FATAL_ERROR "Target '${NAME}' does not exist.")
    endif()
    file(REAL_PATH "${BRIDGING_HEADER}" BRIDGING_HEADER_ABSOLUTE BASE_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
    if(NOT EXISTS "${BRIDGING_HEADER_ABSOLUTE}")
        message(FATAL_ERROR "File '${BRIDGING_HEADER_ABSOLUTE}' does not exist.")
    endif()
    if(IS_DIRECTORY "${BRIDGING_HEADER_ABSOLUTE}")
        message(FATAL_ERROR "File '${BRIDGING_HEADER_ABSOLUTE}' is a directory.")
    endif()

    get_target_property(CURRENT_SWIFT_OBJC_BRIDGING_HEADER ${NAME} XCODE_ATTRIBUTE_SWIFT_OBJC_BRIDGING_HEADER)
    if(CURRENT_SWIFT_OBJC_BRIDGING_HEADER)
        message(FATAL_ERROR "Objective-C to Swift bridging header was already set to '${CURRENT_SWIFT_OBJC_BRIDGING_HEADER}' file previously.")
    endif()

    set(DUMMY_SWIFT_FILE_ABSOLUTE "${CMAKE_CURRENT_BINARY_DIR}/Dummy.swift")
    if(EXISTS "${DUMMY_SWIFT_FILE_ABSOLUTE}")
        if(IS_DIRECTORY "${DUMMY_SWIFT_FILE_ABSOLUTE}")
            message(FATAL_ERROR "File '${DUMMY_SWIFT_FILE_ABSOLUTE}' is a directory.")
        endif()
    else()
        file(WRITE "${DUMMY_SWIFT_FILE_ABSOLUTE}" "// Xcode needs at least one Swift file to recognize Swift build settings such as 'SWIFT_INSTALL_OBJC_HEADER' and 'SWIFT_OBJC_BRIDGING_HEADER' which are set via set_apple_platform_bridging_header() CMake function.")
    endif()

    add_platform_sources(
        ${NAME}
        OBJC_HEADERS
            "${BRIDGING_HEADER_ABSOLUTE}"
        Swift_SOURCES
            "${DUMMY_SWIFT_FILE_ABSOLUTE}"
    )

    set_target_properties(
        ${NAME}
        PROPERTIES
            XCODE_ATTRIBUTE_SWIFT_INSTALL_OBJC_HEADER NO
            XCODE_ATTRIBUTE_SWIFT_OBJC_BRIDGING_HEADER "${BRIDGING_HEADER_ABSOLUTE}"
    )
endfunction()
