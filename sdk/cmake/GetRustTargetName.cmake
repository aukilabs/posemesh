function(GET_RUST_TARGET_NAME OUT_RUST_TARGET_NAME)
    if(APPLE)
        if(PLATFORM STREQUAL "MAC")
            set(${OUT_RUST_TARGET_NAME} "x86_64-apple-darwin" PARENT_SCOPE)
        elseif(PLATFORM STREQUAL "MAC_ARM64")
            set(${OUT_RUST_TARGET_NAME} "aarch64-apple-darwin" PARENT_SCOPE)
        elseif(PLATFORM STREQUAL "MAC_CATALYST")
            set(${OUT_RUST_TARGET_NAME} "x86_64-apple-ios-macabi" PARENT_SCOPE)
        elseif(PLATFORM STREQUAL "MAC_CATALYST_ARM64")
            set(${OUT_RUST_TARGET_NAME} "aarch64-apple-ios-macabi" PARENT_SCOPE)
        elseif(PLATFORM STREQUAL "OS64")
            set(${OUT_RUST_TARGET_NAME} "aarch64-apple-ios" PARENT_SCOPE)
        elseif(PLATFORM STREQUAL "SIMULATOR64")
            set(${OUT_RUST_TARGET_NAME} "x86_64-apple-ios" PARENT_SCOPE)
        elseif(PLATFORM STREQUAL "SIMULATORARM64")
            set(${OUT_RUST_TARGET_NAME} "aarch64-apple-ios-sim" PARENT_SCOPE)
        else()
            message(FATAL_ERROR "Unknown Rust target name.")
        endif()
    elseif(EMSCRIPTEN)
        if(CMAKE_SIZEOF_VOID_P EQUAL 4)
            set(${OUT_RUST_TARGET_NAME} "wasm32-unknown-unknown" PARENT_SCOPE)
        elseif(CMAKE_SIZEOF_VOID_P EQUAL 8)
            set(${OUT_RUST_TARGET_NAME} "wasm64-unknown-unknown" PARENT_SCOPE)
        else()
            message(FATAL_ERROR "Unknown Rust target name.")
        endif()
    else()
        message(FATAL_ERROR "Unknown Rust target name.")
    endif()
endfunction()
