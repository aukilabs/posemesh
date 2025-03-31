include(AddProtoc)
include(AddLibraryImportsEmscripten)

function(ADD_PROTOBUF NAME)
    set(Protobuf_SRC_ROOT_FOLDER "${CMAKE_SOURCE_DIR}/../third-party/protobuf" CACHE STRING INTERNAL)
    
    # extract platform & arcitecture from installation dir
    string(REGEX MATCH "out-([a-zA-Z0-9_]+-[a-zA-Z0-9_]+|[a-zA-Z0-9_]+)-([a-zA-Z0-9_]+)-([a-zA-Z0-9_]+)$" match ${CMAKE_INSTALL_PREFIX})
    set(INSTALL_PLATFORM "${CMAKE_MATCH_1}")
    set(INSTALL_ARCHITECTURE "${CMAKE_MATCH_2}")

    set(PROTOBUF_INSTALL_ROOT "${CMAKE_SOURCE_DIR}/../third-party/out-protobuf-${INSTALL_PLATFORM}-${INSTALL_ARCHITECTURE}-${CMAKE_BUILD_TYPE}")
    set(PROTOBUF_INCLUDE_DIR ${PROTOBUF_INSTALL_ROOT}/include)
    include_directories(${PROTOBUF_INCLUDE_DIR})

    add_protoc(${NAME} ${PROTOBUF_INSTALL_ROOT})

    set(Protobuf_USE_STATIC_LIBS ON)
    set(Protobuf_DEBUG ON)
    find_package(Protobuf REQUIRED CONFIG)

    if(NOT Protobuf_FOUND)
        message(FATAL_ERROR "Failed to find protobuf library build.")
    endif()

    if(EMSCRIPTEN)
        add_library_imports_emscripten(${NAME} "${PROTOBUF_INSTALL_ROOT}")
    endif()

    find_package(absl REQUIRED CONFIG PATHS ${PROTOBUF_INSTALL_ROOT})
    if(NOT absl_FOUND)
        message(FATAL_ERROR "Failed to find absl (a protobuf dependency) library build.")
    endif()

    find_package(utf8_range REQUIRED CONFIG PATHS ${PROTOBUF_INSTALL_ROOT})
    if(NOT utf8_range_FOUND)
        message(FATAL_ERROR "Failed to find utf8_range (a protobuf dependency) library build.")
    endif()

    # .proto file compilation
    if(NOT DEFINED Protobuf_PROTOC_EXECUTABLE)
        message(FATAL_ERROR "Failed to access protoc")
        return()
    endif()
 
    set(PROTO_SRC_DIR "${CMAKE_SOURCE_DIR}/protobuf")
    file(GLOB PROTO_FILES "${PROTO_SRC_DIR}/*.proto")
    if(NOT PROTO_FILES)
        message(WARNING "No .proto files found in ${PROTO_SRC_DIR}")
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
            message(STATUS "Failure params: COMMAND ${Protobuf_PROTOC_EXECUTABLE}")
            message(STATUS "Failure params: --proto_path=${PROTO_SRC_DIR}")
            message(STATUS "Failure params: --cpp_out=${TEMP_DIR} ")
            message(STATUS "Failure params: ${PROTO_FILE}")
            message(STATUS "Failure params: ${RESULT_VARIABLE} ${PROTOC_RESULT}")
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