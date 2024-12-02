function(LINK_OPENCV_LIBRARY NAME)
    if (APPLE)
        if (IOS)
            if (SDK_NAME STREQUAL "iphonesimulator")
                set(OPENCV_STATIC_LIB_DIR ${CMAKE_CURRENT_LIST_DIR}/third-party/opencv/opencv-static-lib-simulator)
            else()
                set(OPENCV_STATIC_LIB_DIR ${CMAKE_CURRENT_LIST_DIR}/third-party/opencv/opencv-static-lib)
            endif()

            set(OPENCV_LIB ${OPENCV_STATIC_LIB_DIR}/opencv2.a)
            include_directories(${OPENCV_STATIC_LIB_DIR})                
            target_link_libraries(${NAME} PRIVATE ${OPENCV_LIB})
        else()
            set(OpenCV_DIR "${CMAKE_CURRENT_LIST_DIR}/third-party/opencv/build/install/lib/cmake/opencv4")
            find_package(OpenCV)
            target_link_libraries(${NAME} PRIVATE ${OpenCV_LIBS})

            # TODO: Use link_platform_libraries instead of target_link_libraries
            # include_directories(${OpenCV_INCLUDE_DIRS})
            # link_platform_libraries(
            #     ${NAME}
            #     # HIDE_SYMBOLS
            #     PRIVATE
            #     "${OpenCV_LIBS}"
            # )
        endif()
    endif()
endfunction()