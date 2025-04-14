function(ADD_PROTOBUF_LIBRARIES NAME PROTOBUF_LIBRARY_ROOT)
    if("${CMAKE_BUILD_TYPE}" STREQUAL "Debug") 
        set(PROTOBUF_LIB "protobufd")    
    else()
        set(PROTOBUF_LIB "protobuf")
    endif()

    set(PROTOBUF_LIB_AND_DEPENDENCIES
        ${PROTOBUF_LIB}
        absl_base
        absl_city
        absl_cord
        absl_cord_internal
        absl_cordz_handle
        absl_cordz_info
        absl_crc32c
        absl_crc_cord_state
        absl_crc_cpu_detect
        absl_crc_internal
        absl_decode_rust_punycode
        absl_demangle_internal
        absl_demangle_rust
        absl_examine_stack
        absl_flags_config
        absl_flags_internal
        absl_flags_reflection
        absl_graphcycles_internal
        absl_hash
        absl_int128
        absl_kernel_timeout_internal
        absl_leak_check
        absl_log_entry
        absl_log_flags
        absl_log_globals
        absl_log_initialize
        absl_log_internal_check_op
        absl_log_internal_conditions
        absl_log_internal_format
        absl_log_internal_globals
        absl_log_internal_log_sink_set
        absl_log_internal_message
        absl_log_internal_nullguard
        absl_log_internal_proto
        absl_log_internal_structured_proto
        absl_log_sink
        absl_low_level_hash
        absl_malloc_internal
        absl_raw_hash_set
        absl_raw_logging_internal
        absl_spinlock_wait
        absl_stacktrace
        absl_status
        absl_statusor
        absl_str_format_internal
        absl_strerror
        absl_strings
        absl_strings_internal
        absl_symbolize
        absl_synchronization
        absl_throw_delegate
        absl_time
        absl_time_zone
        absl_utf8_for_code_point
        absl_vlog_config_internal
        utf8_range
    )
    
    foreach(lib ${PROTOBUF_LIB_AND_DEPENDENCIES})
        list(APPEND PROTOBUF_DEPENDENCY_LIBS "${PROTOBUF_LIBRARY_ROOT}/lib/lib${lib}.a")
    endforeach()

    set(Protobuf_LIBRARIES ${PROTOBUF_DEPENDENCY_LIBS} CACHE STRING INTERNAL)
endfunction()
