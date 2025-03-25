function(LINK_PROTOBUF NAME)
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
endfunction()