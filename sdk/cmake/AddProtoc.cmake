function(ADD_PROTOC NAME PROTO_BUILD_PATH)
    if(NOT Protobuf_SRC_ROOT_FOLDER)
        message(FATAL_ERROR "Protobuf_SRC_ROOT_FOLDER must be set to the protobuf source directory")
    endif()

    set(PROTOC_BUILD_DIR "${PROTO_BUILD_PATH}/build-protoc-${PROTOBUF_OS}-${PROTOBUF_ARCH}")
    file(MAKE_DIRECTORY ${PROTOC_BUILD_DIR})

    # CMake configuration options
    set(PROTOBUF_CMAKE_ARGS
        -DCMAKE_BUILD_TYPE=${CMAKE_BUILD_TYPE}
        -Dprotobuf_BUILD_TESTS=OFF
        -Dprotobuf_BUILD_CONFORMANCE=OFF
        -Dprotobuf_BUILD_EXAMPLES=OFF
        -Dprotobuf_BUILD_SHARED_LIBS=OFF
        -Dprotobuf_DISABLE_RTTI=ON
        -Dprotobuf_STATIC_RUNTIME=ON 
        -Dprotobuf_BUILD_PROTOBUF_BINARIES=ON
        -Dprotobuf_FORCE_FETCH_DEPENDENCIES=ON
        -Dprotobuf_DISABLE_WERROR=ON
        -DCMAKE_CXX_STANDARD=17
        -Dprotobuf_WITH_ZLIB=OFF
    )

    execute_process(
        COMMAND ${CMAKE_COMMAND} 
        ${PROTOBUF_CMAKE_ARGS}
        -S ${Protobuf_SRC_ROOT_FOLDER}
        -B ${PROTOC_BUILD_DIR}
        RESULT_VARIABLE CMAKE_CONFIG_RESULT
        ERROR_VARIABLE CMAKE_CONFIG_ERROR
    )
    if(NOT CMAKE_CONFIG_RESULT EQUAL 0)
        message(FATAL_ERROR "CMake configuration failed: ${CMAKE_CONFIG_ERROR}")
    endif()

    execute_process(
        COMMAND ${CMAKE_COMMAND}
        --build ${PROTOC_BUILD_DIR} 
        --target protoc
        RESULT_VARIABLE CMAKE_BUILD_RESULT
        ERROR_VARIABLE CMAKE_BUILD_ERROR
    )
    if(NOT CMAKE_BUILD_RESULT EQUAL 0)
        message(FATAL_ERROR "Protoc compilation failed: ${CMAKE_BUILD_ERROR}")
    endif()

    set(Protobuf_PROTOC_EXECUTABLE "${PROTOC_BUILD_DIR}/protoc" CACHE STRING INTERNAL)
endfunction()