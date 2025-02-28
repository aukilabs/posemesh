find_program(
    NPM_EXECUTABLE_PATH
    NAMES npm
    REQUIRED
)

set(WEB_PLATFORM_ROOT "${CMAKE_CURRENT_LIST_DIR}/../platform/Web")

function(TRANSPILE_MINIFY_JAVASCRIPT NAME OUTPUT INPUT)
    if(NOT "${NAME}" MATCHES "^[a-zA-Z0-9_.+-]+$")
        message(FATAL_ERROR "Target name '${NAME}' is invalid.")
    endif()
    if(TARGET ${NAME})
        message(FATAL_ERROR "Target '${NAME}' already exists.")
    endif()

    file(REAL_PATH "${OUTPUT}" OUTPUT_ABSOLUTE BASE_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}")
    if(TARGET "${INPUT}")
        get_target_property(CUSTOM_TARGET_OUTPUT ${INPUT} CUSTOM_TARGET_OUTPUT)
        if(CUSTOM_TARGET_OUTPUT)
            set(INPUT_ABSOLUTE "${CUSTOM_TARGET_OUTPUT}")
        else()
            set(INPUT_ABSOLUTE "$<TARGET_FILE:${INPUT}>")
        endif()
        set(INPUT_DEPENDS "${INPUT}")
    else()
        file(REAL_PATH "${INPUT}" INPUT_ABSOLUTE BASE_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
        set(INPUT_DEPENDS "${INPUT_ABSOLUTE}")
    endif()

    add_custom_command(
        OUTPUT "${OUTPUT_ABSOLUTE}"
        COMMAND "${NPM_EXECUTABLE_PATH}" install
        COMMAND "${NPM_EXECUTABLE_PATH}" exec babel "${INPUT_ABSOLUTE}" > "${OUTPUT_ABSOLUTE}"
        DEPENDS
            "${INPUT_DEPENDS}"
        WORKING_DIRECTORY "${WEB_PLATFORM_ROOT}"
    )

    add_custom_target(
        ${NAME} ALL
        DEPENDS
            "${OUTPUT_ABSOLUTE}"
    )
    set_target_properties(
        ${NAME}
        PROPERTIES
            CUSTOM_TARGET_OUTPUT "${OUTPUT_ABSOLUTE}"
    )
endfunction()
