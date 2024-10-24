#ifndef __POSEMESH_NETWORKING_API_H__
#define __POSEMESH_NETWORKING_API_H__

#include <stdint.h>

#if defined(__EMSCRIPTEN__)
#include <assert.h>
#include <emscripten.h>
#endif

typedef struct {
    #if !defined(__EMSCRIPTEN__)
        uint8_t serve_as_bootstrap;
        uint8_t serve_as_relay;
    #endif
    const char* bootstraps;
} psm_posemesh_networking_config_t;

typedef struct psm_posemesh_networking_context psm_posemesh_networking_context_t;

#if defined(__cplusplus)
extern "C" {
#endif

#if !defined(__EMSCRIPTEN__)
    psm_posemesh_networking_context_t* psm_posemesh_networking_context_create(const psm_posemesh_networking_config_t* config);
#else
    static psm_posemesh_networking_context_t* psm_posemesh_networking_context_create(const psm_posemesh_networking_config_t* config) {
        assert(config);
        void* context = EM_ASM_PTR({
            let config = new wasm_bindgen.Config(UTF8ToString($0));
            try {
                return wasm_bindgen.posemeshNetworkingContextCreate(config);
            } finally {
                config.free();
            }
        }, config->bootstraps);
        return (psm_posemesh_networking_context_t*)context;
    }
#endif

#if !defined(__EMSCRIPTEN__)
    void psm_posemesh_networking_context_destroy(psm_posemesh_networking_context_t* context);
#else
    static void psm_posemesh_networking_context_destroy(psm_posemesh_networking_context_t* context) {
        assert(context);
        EM_ASM({
            wasm_bindgen.posemeshNetworkingContextDestroy($0);
        }, context);
    }
#endif

#if defined(__cplusplus)
}
#endif

#endif // __POSEMESH_NETWORKING_API_H__
