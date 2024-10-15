if(APPLE)
    if(CMAKE_HOST_SYSTEM_PROCESSOR STREQUAL "arm64")
        set(PLATFORM "MAC_ARM64")
    else()
        set(PLATFORM "MAC")
    endif()
    set(THIRD_PARTY_IOS_TOOLCHAIN_FILE_PATH "${CMAKE_CURRENT_LIST_DIR}/../../third-party/ios-cmake/ios.toolchain.cmake")
    if(NOT EXISTS "${THIRD_PARTY_IOS_TOOLCHAIN_FILE_PATH}")
        message(FATAL_ERROR "File '${THIRD_PARTY_IOS_TOOLCHAIN_FILE_PATH}' does not exist. Are the Git repository submodules cloned?")
    endif()
    if(IS_DIRECTORY "${THIRD_PARTY_IOS_TOOLCHAIN_FILE_PATH}")
        message(FATAL_ERROR "File '${THIRD_PARTY_IOS_TOOLCHAIN_FILE_PATH}' is a directory.")
    endif()
    include("${THIRD_PARTY_IOS_TOOLCHAIN_FILE_PATH}")
endif()
