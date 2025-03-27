function(ADD_LIBRARY_IMPORTS_EMSCRIPTEN NAME PROTOBUF_LIBRARY_ROOT)
    file(GLOB LIB_FILES "${PROTOBUF_LIBRARY_ROOT}/lib/*.a")

    foreach(LIB ${LIB_FILES})
        if(NOT TARGET ${LIB_NAME})
            list(APPEND PROTOBUF_DEPENDENCY_LIBS ${LIB})
        endif()
    endforeach()
        
    set(Protobuf_LIBRARIES ${PROTOBUF_DEPENDENCY_LIBS} CACHE STRING INTERNAL)
endfunction()