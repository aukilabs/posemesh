find_program(
    NODE_EXECUTABLE_PATH
    NAMES node
    REQUIRED
)

set(WEB_PLATFORM_ROOT "${CMAKE_CURRENT_LIST_DIR}/../platform/Web")

function(TRANSFORM_TYPESCRIPT_DEFINITIONS NAME OUTPUT INPUT)
    if(NOT "${NAME}" MATCHES "^[a-zA-Z0-9_.+-]+$")
        message(FATAL_ERROR "Target name '${NAME}' is invalid.")
    endif()
    if(TARGET ${NAME})
        message(FATAL_ERROR "Target '${NAME}' already exists.")
    endif()

    file(REAL_PATH "${OUTPUT}" OUTPUT_ABSOLUTE BASE_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}")
    file(REAL_PATH "${INPUT}" INPUT_ABSOLUTE BASE_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")

    add_custom_command(
        OUTPUT "${OUTPUT_ABSOLUTE}"
        COMMAND "${NODE_EXECUTABLE_PATH}" transform-typescript-definitions.js "${INPUT_ABSOLUTE}" "${OUTPUT_ABSOLUTE}"
        DEPENDS
            "${INPUT_ABSOLUTE}"
        WORKING_DIRECTORY "${WEB_PLATFORM_ROOT}"
    )

    add_custom_target(
        ${NAME} ALL
        DEPENDS
            "${OUTPUT_ABSOLUTE}"
    )
endfunction()
