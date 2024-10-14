# Platform languages
if(APPLE)
    set(PLATFORM_LANGUAGES C CXX OBJC OBJCXX Swift)
else()
    set(PLATFORM_LANGUAGES C CXX)
endif()

# Platform functions
include("${CMAKE_CURRENT_LIST_DIR}/AddPlatformLibrary.cmake")
include("${CMAKE_CURRENT_LIST_DIR}/AddPlatformSources.cmake")
include("${CMAKE_CURRENT_LIST_DIR}/SetApplePlatformBridgingHeader.cmake")
