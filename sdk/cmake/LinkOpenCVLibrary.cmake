include("${CMAKE_CURRENT_LIST_DIR}/GetBuildDirectorySuffix.cmake")
include("${CMAKE_CURRENT_LIST_DIR}/LinkPlatformLibraries.cmake")

set(THIRD_PARTY_PREFIX "${CMAKE_CURRENT_LIST_DIR}/../../third-party")

function(LINK_OPENCV_LIBRARY NAME)
    if(NOT TARGET ${NAME})
        message(FATAL_ERROR "Target '${NAME}' does not exist.")
    endif()

    get_build_directory_suffix(BUILD_DIRECTORY_SUFFIX)
    set(OPENCV_OUTPUT_DIRECTORY "${THIRD_PARTY_PREFIX}/out-OpenCV-${BUILD_DIRECTORY_SUFFIX}")
    set(OPENCV_INCLUDE_DIRECTORY "${OPENCV_OUTPUT_DIRECTORY}/include")
    set(OPENCV_LIBRARY_DIRECTORY "${OPENCV_OUTPUT_DIRECTORY}/lib")

    if(NOT EXISTS "${OPENCV_INCLUDE_DIRECTORY}" OR NOT IS_DIRECTORY "${OPENCV_INCLUDE_DIRECTORY}")
        message(FATAL_ERROR "OpenCV library is not built for targeted platform, architecture and configuration (build type): Includes directory is missing.")
    endif()
    target_include_directories(
        ${NAME}
        PRIVATE
            ${OPENCV_INCLUDE_DIRECTORY}
    )

    if(EMSCRIPTEN)
        message(FATAL_ERROR "TODO: implement linking OpenCV in web")
    else()
        set(OPENCV_LIBRARY "${OPENCV_LIBRARY_DIRECTORY}/libopencv2.a")
        if(NOT EXISTS "${OPENCV_LIBRARY}" OR IS_DIRECTORY "${OPENCV_LIBRARY}")
            message(FATAL_ERROR "OpenCV library is not built for targeted platform, architecture and configuration (build type): Archive file is missing.")
        endif()

        link_platform_libraries(
            ${NAME}
            HIDE_SYMBOLS
            PRIVATE
                "${OPENCV_LIBRARY}"
        )
    endif()
endfunction()
