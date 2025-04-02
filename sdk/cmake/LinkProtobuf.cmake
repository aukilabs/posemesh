function(LINK_PROTOBUF NAME)
    link_directories(${PROTOBUF_LIBS_DIR})

    if(EMSCRIPTEN)
        link_platform_libraries(
            ${NAME}
            HIDE_SYMBOLS
            PRIVATE
                "${Protobuf_LIBRARIES}"
        )
    else()
        target_link_libraries(
            ${NAME} 
            PRIVATE
            protobuf::libprotobuf
            absl::log
            absl::log_internal_check_op
            absl::status
            absl::statusor
            absl::raw_hash_map
            utf8_range::utf8_range
        )
    endif()
endfunction()