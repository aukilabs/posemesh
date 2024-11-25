find_program(
    GIT_EXECUTABLE_PATH
    NAMES git
    REQUIRED
)

function(GET_COMMIT_ID OUT_COMMIT_ID_VAR)
    execute_process(
        COMMAND
            "${GIT_EXECUTABLE_PATH}" log -1 --format=%H
        WORKING_DIRECTORY "${CMAKE_CURRENT_LIST_DIR}"
        TIMEOUT 5
        OUTPUT_VARIABLE COMMIT_ID
        OUTPUT_STRIP_TRAILING_WHITESPACE
        COMMAND_ERROR_IS_FATAL ANY
    )
    set(${OUT_COMMIT_ID_VAR} "${COMMIT_ID}" PARENT_SCOPE)
endfunction()
