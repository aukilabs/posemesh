function(LINK_PROTOBUF NAME)
    get_property(Protobuf_LIBRARIES GLOBAL PROPERTY Protobuf_LIBRARIES)

    link_platform_libraries(
        ${NAME}
        HIDE_SYMBOLS
        PRIVATE
            "${Protobuf_LIBRARIES}"
    )
endfunction()