function(LINK_PROTOBUF NAME)
    target_include_directories(
        ${NAME}
        PRIVATE
            ${PROTOBUF_INCLUDE_DIR}
    )
    link_platform_libraries(
        ${NAME}
        HIDE_SYMBOLS
        PRIVATE
            "${Protobuf_LIBRARIES}"
    )
endfunction()