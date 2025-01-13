function(ADD_GLM_HEADERS NAME)
    target_include_directories(${NAME} PRIVATE "${CMAKE_CURRENT_LIST_DIR}/../third-party/glm")
endfunction()