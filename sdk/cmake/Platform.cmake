# Platform languages
if(CMAKE_TOOLCHAIN_FILE)
    if(NOT EXISTS "${CMAKE_TOOLCHAIN_FILE}")
        message(FATAL_ERROR "Specified toolchain file '${CMAKE_TOOLCHAIN_FILE}' does not exist.")
    endif()
    if(IS_DIRECTORY "${CMAKE_TOOLCHAIN_FILE}")
        message(FATAL_ERROR "Specified toolchain file '${CMAKE_TOOLCHAIN_FILE}' is a directory.")
    endif()
    get_filename_component(CMAKE_TOOLCHAIN_FILE_ABSOLUTE "${CMAKE_TOOLCHAIN_FILE}" ABSOLUTE)

    set(VSCODE_TOOLCHAIN_SELECTOR_FILE_UNRESOLVED "${CMAKE_CURRENT_LIST_DIR}/VSCodeToolchainSelector.cmake")
    if(NOT EXISTS "${VSCODE_TOOLCHAIN_SELECTOR_FILE_UNRESOLVED}")
        message(FATAL_ERROR "Toolchain file '${VSCODE_TOOLCHAIN_SELECTOR_FILE_UNRESOLVED}' does not exist.")
    endif()
    if(IS_DIRECTORY "${VSCODE_TOOLCHAIN_SELECTOR_FILE_UNRESOLVED}")
        message(FATAL_ERROR "Toolchain file '${VSCODE_TOOLCHAIN_SELECTOR_FILE_UNRESOLVED}' is a directory.")
    endif()
    get_filename_component(VSCODE_TOOLCHAIN_SELECTOR_FILE_ABSOLUTE "${VSCODE_TOOLCHAIN_SELECTOR_FILE_UNRESOLVED}" ABSOLUTE)

    set(IOS_TOOLCHAIN_FILE_UNRESOLVED "${CMAKE_CURRENT_LIST_DIR}/../../third-party/ios-cmake/ios.toolchain.cmake")
    if(NOT EXISTS "${IOS_TOOLCHAIN_FILE_UNRESOLVED}")
        message(FATAL_ERROR "Toolchain file '${IOS_TOOLCHAIN_FILE_UNRESOLVED}' does not exist. Are the Git repository submodules cloned?")
    endif()
    if(IS_DIRECTORY "${IOS_TOOLCHAIN_FILE_UNRESOLVED}")
        message(FATAL_ERROR "Toolchain file '${IOS_TOOLCHAIN_FILE_UNRESOLVED}' is a directory.")
    endif()
    get_filename_component(IOS_TOOLCHAIN_FILE_ABSOLUTE "${IOS_TOOLCHAIN_FILE_UNRESOLVED}" ABSOLUTE)

    if(CMAKE_TOOLCHAIN_FILE_ABSOLUTE STREQUAL VSCODE_TOOLCHAIN_SELECTOR_FILE_ABSOLUTE)
        if(APPLE)
            set(PLATFORM_LANGUAGES C CXX OBJC OBJCXX Swift)
        else()
            set(PLATFORM_LANGUAGES C CXX)
        endif()
    elseif(CMAKE_TOOLCHAIN_FILE_ABSOLUTE STREQUAL IOS_TOOLCHAIN_FILE_ABSOLUTE)
        set(PLATFORM_LANGUAGES C CXX OBJC OBJCXX Swift)
    else()
        set(PLATFORM_LANGUAGES C CXX)
    endif()
else()
    set(PLATFORM_LANGUAGES C CXX)
endif()

# Platform functions
include("${CMAKE_CURRENT_LIST_DIR}/AddPlatformLibrary.cmake")
include("${CMAKE_CURRENT_LIST_DIR}/AddPlatformSources.cmake")
include("${CMAKE_CURRENT_LIST_DIR}/LinkPlatformLibraries.cmake")
include("${CMAKE_CURRENT_LIST_DIR}/SetAppleInfo.cmake")
include("${CMAKE_CURRENT_LIST_DIR}/SetApplePlatformBridgingHeader.cmake")
include("${CMAKE_CURRENT_LIST_DIR}/SetApplePlatformUmbrellaHeader.cmake")
