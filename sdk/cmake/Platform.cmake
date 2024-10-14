# Platform languages
if(APPLE)
    set(PLATFORM_LANGUAGES C CXX OBJC OBJCXX Swift)
else()
    set(PLATFORM_LANGUAGES C CXX)
endif()

# Platform functions
include(AddPlatformLibrary)
include(AddPlatformSources)
