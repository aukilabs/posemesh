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

    set(PROTO_OUT_DIR "${CMAKE_SOURCE_DIR}/src/Protobuf")
    file(MAKE_DIRECTORY "${PROTO_OUT_DIR}")

    foreach(PROTO_FILE ${PROTO_FILES})
    execute_process(
        COMMAND ${Protobuf_PROTOC_EXECUTABLE}
        --proto_path=${PROTO_SRC_DIR} 
        --cpp_out=${PROTO_OUT_DIR} 
        ${PROTO_FILE}
        RESULT_VARIABLE PROTOC_RESULT
    )
    if(NOT PROTOC_RESULT EQUAL 0)
        message(FATAL_ERROR "Protobuf compilation failed for ${PROTO_FILE}")
    endif()
    endforeach()

    file(GLOB PROTO_HEADERS "${PROTO_OUT_DIR}/*.pb.h")
    file(GLOB PROTO_SOURCES "${PROTO_OUT_DIR}/*.pb.cc")
    set(POSEMESH_GENERATED_PROTOBUF_CXX_HEADERS ${PROTO_HEADERS} CACHE STRING INTERNAL)
    set(POSEMESH_GENERATED_PROTOBUF_CXX_SOURCES ${PROTO_SOURCES} CACHE STRING INTERNAL)

    message(STATUS ".proto files compiled with ${Protobuf_PROTOC_EXECUTABLE}")
endfunction()
