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
        set(OPENCV_CALIB3D_LIBRARY "${OPENCV_LIBRARY_DIRECTORY}/libopencv_calib3d.a")
        if(NOT EXISTS "${OPENCV_CALIB3D_LIBRARY}" OR IS_DIRECTORY "${OPENCV_CALIB3D_LIBRARY}")
            message(FATAL_ERROR "OpenCV library is not built for targeted platform, architecture and configuration (build type): Archive file 'libopencv_calib3d.a' is missing.")
        endif()

        set(OPENCV_CORE_LIBRARY "${OPENCV_LIBRARY_DIRECTORY}/libopencv_core.a")
        if(NOT EXISTS "${OPENCV_CORE_LIBRARY}" OR IS_DIRECTORY "${OPENCV_CORE_LIBRARY}")
            message(FATAL_ERROR "OpenCV library is not built for targeted platform, architecture and configuration (build type): Archive file 'libopencv_core.a' is missing.")
        endif()

        set(OPENCV_DNN_LIBRARY "${OPENCV_LIBRARY_DIRECTORY}/libopencv_dnn.a")
        if(NOT EXISTS "${OPENCV_DNN_LIBRARY}" OR IS_DIRECTORY "${OPENCV_DNN_LIBRARY}")
            message(FATAL_ERROR "OpenCV library is not built for targeted platform, architecture and configuration (build type): Archive file 'libopencv_dnn.a' is missing.")
        endif()

        set(OPENCV_FEATURES2D_LIBRARY "${OPENCV_LIBRARY_DIRECTORY}/libopencv_features2d.a")
        if(NOT EXISTS "${OPENCV_FEATURES2D_LIBRARY}" OR IS_DIRECTORY "${OPENCV_FEATURES2D_LIBRARY}")
            message(FATAL_ERROR "OpenCV library is not built for targeted platform, architecture and configuration (build type): Archive file 'libopencv_features2d.a' is missing.")
        endif()

        set(OPENCV_FLANN_LIBRARY "${OPENCV_LIBRARY_DIRECTORY}/libopencv_flann.a")
        if(NOT EXISTS "${OPENCV_FLANN_LIBRARY}" OR IS_DIRECTORY "${OPENCV_FLANN_LIBRARY}")
            message(FATAL_ERROR "OpenCV library is not built for targeted platform, architecture and configuration (build type): Archive file 'libopencv_flann.a' is missing.")
        endif()

        set(OPENCV_IMGPROC_LIBRARY "${OPENCV_LIBRARY_DIRECTORY}/libopencv_imgproc.a")
        if(NOT EXISTS "${OPENCV_IMGPROC_LIBRARY}" OR IS_DIRECTORY "${OPENCV_IMGPROC_LIBRARY}")
            message(FATAL_ERROR "OpenCV library is not built for targeted platform, architecture and configuration (build type): Archive file 'libopencv_imgproc.a' is missing.")
        endif()

        set(OPENCV_OBJDETECT_LIBRARY "${OPENCV_LIBRARY_DIRECTORY}/libopencv_objdetect.a")
        if(NOT EXISTS "${OPENCV_OBJDETECT_LIBRARY}" OR IS_DIRECTORY "${OPENCV_OBJDETECT_LIBRARY}")
            message(FATAL_ERROR "OpenCV library is not built for targeted platform, architecture and configuration (build type): Archive file 'libopencv_objdetect.a' is missing.")
        endif()

        set(OPENCV_PHOTO_LIBRARY "${OPENCV_LIBRARY_DIRECTORY}/libopencv_photo.a")
        if(NOT EXISTS "${OPENCV_PHOTO_LIBRARY}" OR IS_DIRECTORY "${OPENCV_PHOTO_LIBRARY}")
            message(FATAL_ERROR "OpenCV library is not built for targeted platform, architecture and configuration (build type): Archive file 'libopencv_photo.a' is missing.")
        endif()

        set(OPENCV_VIDEO_LIBRARY "${OPENCV_LIBRARY_DIRECTORY}/libopencv_video.a")
        if(NOT EXISTS "${OPENCV_VIDEO_LIBRARY}" OR IS_DIRECTORY "${OPENCV_VIDEO_LIBRARY}")
            message(FATAL_ERROR "OpenCV library is not built for targeted platform, architecture and configuration (build type): Archive file 'libopencv_video.a' is missing.")
        endif()

        link_platform_libraries(
            ${NAME}
            HIDE_SYMBOLS
            PRIVATE
                "${OPENCV_CALIB3D_LIBRARY}"
                "${OPENCV_CORE_LIBRARY}"
                "${OPENCV_DNN_LIBRARY}"
                "${OPENCV_FEATURES2D_LIBRARY}"
                "${OPENCV_FLANN_LIBRARY}"
                "${OPENCV_IMGPROC_LIBRARY}"
                "${OPENCV_OBJDETECT_LIBRARY}"
                "${OPENCV_PHOTO_LIBRARY}"
                "${OPENCV_VIDEO_LIBRARY}"
        )
    else()
        if(APPLE)
            target_link_libraries(${NAME} PRIVATE "-framework Accelerate")
            if(PLATFORM MATCHES "MAC")
                target_link_libraries(${NAME} PRIVATE "-framework OpenCL")
            endif()
        endif()

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
