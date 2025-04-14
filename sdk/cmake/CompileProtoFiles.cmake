function(COMPILE_PROTO_FILES)
    if(NOT DEFINED Protobuf_PROTOC_EXECUTABLE)
        message(FATAL_ERROR "Failed to access protoc")
        return()
    endif()

    set(PROTO_SRC_DIR "${CMAKE_SOURCE_DIR}/protobuf")
    file(GLOB PROTO_FILES "${PROTO_SRC_DIR}/*.proto")
    if(NOT PROTO_FILES)
        message(STATUS "No .proto files found in ${PROTO_SRC_DIR}")
        return()
    endif()

    set(TEMP_DIR "${CMAKE_BINARY_DIR}/protobuf_temp")
    set(OUT_HEADER_DIR "${CMAKE_SOURCE_DIR}/include/Posemesh/Protobuf")
    file(MAKE_DIRECTORY "${TEMP_DIR}")
    file(MAKE_DIRECTORY "${OUT_HEADER_DIR}")

    foreach(PROTO_FILE ${PROTO_FILES})
    execute_process(
        COMMAND ${Protobuf_PROTOC_EXECUTABLE}
        --proto_path=${PROTO_SRC_DIR} 
        --cpp_out=${TEMP_DIR} 
        ${PROTO_FILE}
        RESULT_VARIABLE PROTOC_RESULT
    )
    if(NOT PROTOC_RESULT EQUAL 0)
        message(FATAL_ERROR "Protobuf compilation failed for ${PROTO_FILE}")
    endif()
    endforeach()

    file(GLOB GENERATED_HEADERS "${TEMP_DIR}/*.pb.h")
    file(GLOB GENERATED_SOURCES "${TEMP_DIR}/*.pb.cc")

    file(COPY ${GENERATED_HEADERS} DESTINATION "${OUT_HEADER_DIR}")

    foreach(SOURCE ${GENERATED_SOURCES})
        get_filename_component(SOURCE_NAME ${SOURCE} NAME_WE)
        file(RENAME "${SOURCE}" "${OUT_HEADER_DIR}/${SOURCE_NAME}.pb.cpp")
    endforeach()

    file(REMOVE_RECURSE "${TEMP_DIR}")

    file(GLOB PROTO_HEADERS "${OUT_HEADER_DIR}/*.pb.h")
    file(GLOB PROTO_SOURCES "${OUT_HEADER_DIR}/*.pb.cpp")
    set(POSEMESH_GENERATED_PROTOBUF_CXX_HEADERS ${PROTO_HEADERS} CACHE STRING INTERNAL)
    set(POSEMESH_GENERATED_PROTOBUF_CXX_SOURCES ${PROTO_SOURCES} CACHE STRING INTERNAL)

    message(STATUS ".proto files compiled with ${Protobuf_PROTOC_EXECUTABLE}")
endfunction()
