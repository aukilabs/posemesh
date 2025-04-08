include(AddProtobufLibraries)
include(AddProtoc)
include(CompileProtoFiles)

function(ADD_PROTOBUF NAME)
    string(REGEX MATCH "out-([a-zA-Z0-9_]+-[a-zA-Z0-9_]+|[a-zA-Z0-9_]+)-([a-zA-Z0-9_]+)-([a-zA-Z0-9_]+)$" match ${CMAKE_INSTALL_PREFIX})
    set(INSTALL_PLATFORM "${CMAKE_MATCH_1}")
    set(INSTALL_ARCHITECTURE "${CMAKE_MATCH_2}")
    set(THIRD_PARTY_DIR_REL "${CMAKE_SOURCE_DIR}/../third-party")
    cmake_path(NORMAL_PATH THIRD_PARTY_DIR_REL OUTPUT_VARIABLE THIRD_PARTY_DIR)
    
    set(PROTOBUF_BUILD_DIR "${THIRD_PARTY_DIR}/build-protobuf-${INSTALL_PLATFORM}-${INSTALL_ARCHITECTURE}-${CMAKE_BUILD_TYPE}")
    set(PROTOBUF_ROOT "${THIRD_PARTY_DIR}/out-protobuf-${INSTALL_PLATFORM}-${INSTALL_ARCHITECTURE}-${CMAKE_BUILD_TYPE}")

    if(NOT EXISTS ${PROTOBUF_ROOT})
        message(FATAL_ERROR "Protobuf library is not built for targeted platform, architecture and configuration (build type): Install directory is missing (expected at ${PROTOBUF_ROOT}).")
    endif()

    if(NOT EXISTS ${PROTOBUF_BUILD_DIR})
        message(FATAL_ERROR "Protobuf library is not built for targeted platform, architecture and configuration (build type): Build directory is missing (expected at ${PROTOBUF_BUILD_DIR}).")
    endif()

    set(PROTOBUF_INCLUDE_DIR ${PROTOBUF_ROOT}/include)
    set(Protobuf_SRC_ROOT_FOLDER "${THIRD_PARTY_DIR}/protobuf" CACHE STRING INTERNAL)

    add_protoc(${NAME} ${PROTOBUF_ROOT})
    add_protobuf_libraries(${NAME} ${PROTOBUF_ROOT})

    set(Protobuf_USE_STATIC_LIBS ON)
    find_package(Protobuf REQUIRED)
    include_directories(${PROTOBUF_INCLUDE_DIR})

    if(NOT Protobuf_FOUND)
        message(FATAL_ERROR "Failed to find protobuf library build.")
    endif()

    compile_proto_files()
endfunction()