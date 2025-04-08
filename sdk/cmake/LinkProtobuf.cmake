function(LINK_PROTOBUF NAME)
    link_platform_libraries(
        ${NAME}
        HIDE_SYMBOLS
        PRIVATE
            "${Protobuf_LIBRARIES}"
    )
endfunction()