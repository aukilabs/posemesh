function(MERGE_FILES NAME)
    list(APPEND OPTION_KEYWORDS)
    list(
        APPEND SINGLE_VALUE_KEYWORDS
            OUTPUT
    )
    list(
        APPEND MULTI_VALUE_KEYWORDS
            INPUTS
    )
    cmake_parse_arguments(ARG "${OPTION_KEYWORDS}" "${SINGLE_VALUE_KEYWORDS}" "${MULTI_VALUE_KEYWORDS}" ${ARGN})
    if(DEFINED ARG_UNPARSED_ARGUMENTS)
        list(GET ARG_UNPARSED_ARGUMENTS 0 FIRST_UNKNOWN_KEYWORD)
        message(FATAL_ERROR "Unknown '${FIRST_UNKNOWN_KEYWORD}' keyword.")
    endif()

    if(NOT "${NAME}" MATCHES "^[a-zA-Z0-9_.+-]+$")
        message(FATAL_ERROR "Target name '${NAME}' is invalid.")
    endif()
    if(TARGET ${NAME})
        message(FATAL_ERROR "Target '${NAME}' already exists.")
    endif()

    if(NOT ARG_OUTPUT)
        message(FATAL_ERROR "Keyword 'OUTPUT' is not specified or it is empty.")
    endif()
    file(REAL_PATH "${ARG_OUTPUT}" OUTPUT_ABSOLUTE BASE_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}")

    if(NOT ARG_INPUTS)
        message(FATAL_ERROR "Keyword 'INPUTS' is not specified or no arguments are specified for 'INPUTS' keyword.")
    endif()
    list(APPEND INPUT_TUPLES)
    foreach(INPUT_FILE_OR_TARGET ${ARG_INPUTS})
        if(TARGET "${INPUT_FILE_OR_TARGET}")
            list(
                APPEND INPUT_TUPLES
                "${INPUT_FILE_OR_TARGET}"
                ON
            )
        else()
            file(REAL_PATH "${INPUT_FILE_OR_TARGET}" INPUT_PATH_ABSOLUTE BASE_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")
            if(NOT EXISTS "${INPUT_PATH_ABSOLUTE}")
                message(FATAL_ERROR "File '${INPUT_PATH_ABSOLUTE}' does not exist.")
            endif()
            if(IS_DIRECTORY "${INPUT_PATH_ABSOLUTE}")
                message(FATAL_ERROR "File '${INPUT_PATH_ABSOLUTE}' is a directory.")
            endif()
            list(
                APPEND INPUT_TUPLES
                "${INPUT_PATH_ABSOLUTE}"
                OFF
            )
        endif()
    endforeach()

    list(
        APPEND COMMAND_ARGS
            COMMAND "${CMAKE_COMMAND}" -E cat
    )
    list(
        APPEND DEPENDS_ARGS
            DEPENDS
    )

    list(LENGTH INPUT_TUPLES INPUT_TUPLES_LENGTH)
    math(EXPR FOREACH_STOP "${INPUT_TUPLES_LENGTH} - 1")
    foreach(FOREACH_INDEX RANGE 0 ${FOREACH_STOP} 2)
        list(GET INPUT_TUPLES ${FOREACH_INDEX} TARGET_OR_FILE)
        math(EXPR FOREACH_INDEX_NEXT "${FOREACH_INDEX} + 1")
        list(GET INPUT_TUPLES ${FOREACH_INDEX_NEXT} IS_TARGET)
        if(IS_TARGET)
            list(APPEND COMMAND_ARGS "$<TARGET_FILE:${TARGET_OR_FILE}>")
            list(APPEND DEPENDS_ARGS "${TARGET_OR_FILE}")
        else()
            list(APPEND COMMAND_ARGS "${TARGET_OR_FILE}")
            list(APPEND DEPENDS_ARGS "${TARGET_OR_FILE}")
        endif()
    endforeach()
    list(APPEND COMMAND_ARGS > "${OUTPUT_ABSOLUTE}")

    add_custom_command(
        OUTPUT "${OUTPUT_ABSOLUTE}"
        ${COMMAND_ARGS}
        ${DEPENDS_ARGS}
    )

    add_custom_target(
        ${NAME} ALL
        DEPENDS
            "${OUTPUT_ABSOLUTE}"
    )
endfunction()
