function(ADD_PROTOBUF_LIBRARIES NAME PROTOBUF_LIBRARY_ROOT)
    if("${CMAKE_BUILD_TYPE}" STREQUAL "Debug") 
        set(PROTOBUF_LIB "libprotobufd.a")    
    else()
        set(PROTOBUF_LIB "libprotobuf.a")
    endif()

    set(PROTOBUF_LIB_AND_DEPENDENCIES
        ${PROTOBUF_LIB}
        libabsl_base.a
        libabsl_city.a
        libabsl_cord.a
        libabsl_cord_internal.a
        libabsl_cordz_handle.a
        libabsl_cordz_info.a
        libabsl_crc32c.a
        libabsl_crc_cord_state.a
        libabsl_crc_cpu_detect.a
        libabsl_crc_internal.a
        libabsl_decode_rust_punycode.a
        libabsl_demangle_internal.a
        libabsl_demangle_rust.a
        libabsl_examine_stack.a
        libabsl_flags_config.a
        libabsl_flags_internal.a
        libabsl_flags_reflection.a
        libabsl_hash.a
        libabsl_int128.a
        libabsl_kernel_timeout_internal.a
        libabsl_leak_check.a
        libabsl_log_entry.a
        libabsl_log_flags.a
        libabsl_log_globals.a
        libabsl_log_initialize.a
        libabsl_log_internal_check_op.a
        libabsl_log_internal_conditions.a
        libabsl_log_internal_format.a
        libabsl_log_internal_globals.a
        libabsl_log_internal_log_sink_set.a
        libabsl_log_internal_message.a
        libabsl_log_internal_nullguard.a
        libabsl_log_internal_proto.a
        libabsl_log_internal_structured_proto.a
        libabsl_log_sink.a
        libabsl_low_level_hash.a
        libabsl_malloc_internal.a
        libabsl_raw_hash_set.a
        libabsl_raw_logging_internal.a
        libabsl_spinlock_wait.a
        libabsl_stacktrace.a
        libabsl_status.a
        libabsl_statusor.a
        libabsl_str_format_internal.a
        libabsl_strerror.a
        libabsl_strings.a
        libabsl_strings_internal.a
        libabsl_symbolize.a
        libabsl_synchronization.a
        libabsl_time.a
        libabsl_time_zone.a
        libabsl_utf8_for_code_point.a
        libabsl_vlog_config_internal.a
        libutf8_range.a
    )
    
    foreach(lib ${PROTOBUF_LIB_AND_DEPENDENCIES})
        list(APPEND PROTOBUF_DEPENDENCY_LIBS "${PROTOBUF_LIBRARY_ROOT}/lib/${lib}")
    endforeach()

    set(Protobuf_LIBRARIES ${PROTOBUF_DEPENDENCY_LIBS} CACHE STRING INTERNAL)
endfunction()