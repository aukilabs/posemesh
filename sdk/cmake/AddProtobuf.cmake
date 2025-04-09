include(AddProtobufLibraries)
include(AddProtoc)
include(CompileProtoFiles)
include(GetBuildDirectorySuffix)

function(ADD_PROTOBUF NAME)
    set(THIRD_PARTY_DIR_REL "${CMAKE_SOURCE_DIR}/../third-party")
    cmake_path(NORMAL_PATH THIRD_PARTY_DIR_REL OUTPUT_VARIABLE THIRD_PARTY_DIR)

    get_build_directory_suffix(BUILD_DIRECTORY_SUFFIX)    
    set(PROTOBUF_BUILD_DIR "${THIRD_PARTY_DIR}/build-protobuf-${BUILD_DIRECTORY_SUFFIX}")
    set(PROTOBUF_ROOT "${THIRD_PARTY_DIR}/out-protobuf-${BUILD_DIRECTORY_SUFFIX}")

    if(NOT EXISTS ${PROTOBUF_ROOT})
        message(FATAL_ERROR "Protobuf library is not built for targeted platform, architecture and configuration (build type): Install directory is missing (expected at ${PROTOBUF_ROOT}).")
    endif()

    if(NOT EXISTS ${PROTOBUF_BUILD_DIR})
        message(FATAL_ERROR "Protobuf library is not built for targeted platform, architecture and configuration (build type): Build directory is missing (expected at ${PROTOBUF_BUILD_DIR}).")
    endif()

    set(PROTOBUF_INCLUDE_DIR ${PROTOBUF_ROOT}/include CACHE STRING INTERNAL)
    set(Protobuf_SRC_ROOT_FOLDER "${THIRD_PARTY_DIR}/protobuf")

    add_protoc(${NAME} ${Protobuf_SRC_ROOT_FOLDER} ${PROTOBUF_ROOT})
    add_protobuf_libraries(${NAME} ${PROTOBUF_ROOT})
    compile_proto_files()
endfunction()