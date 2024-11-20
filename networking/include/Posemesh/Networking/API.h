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
    const char* relays;
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
        const char* const bootstraps = config->bootstraps;
        assert(bootstraps);
        const char* const relays = config->relays;
        assert(relays);
        void* context = EM_ASM_PTR({
            let bootstraps = UTF8ToString($0);
            let relays = UTF8ToString($1);
            let config = new __internalPosemeshNetworking.Config(bootstraps, relays);
            try {
                return __internalPosemeshNetworking.posemeshNetworkingContextCreate(config);
            } finally {
                config.free();
            }
        }, bootstraps, relays);
        return (psm_posemesh_networking_context_t*)context;
    }
#endif

#if !defined(__EMSCRIPTEN__)
    void psm_posemesh_networking_context_destroy(psm_posemesh_networking_context_t* context);
#else
    static void psm_posemesh_networking_context_destroy(psm_posemesh_networking_context_t* context) {
        assert(context);
        EM_ASM({
            __internalPosemeshNetworking.posemeshNetworkingContextDestroy($0);
        }, context);
    }
#endif

#if !defined(__EMSCRIPTEN__)
    uint8_t psm_posemesh_networking_context_send_message(
        psm_posemesh_networking_context_t* context,
        const void* message,
        uint32_t message_size,
        const char* peer_id,
        const char* protocol,
        void* user_data,
        void (*callback)(uint8_t status, void* user_data)
    );
#else
    static uint8_t psm_posemesh_networking_context_send_message(
        psm_posemesh_networking_context_t* context,
        const void* message,
        uint32_t message_size,
        const char* peer_id,
        const char* protocol,
        void* user_data,
        void (*callback)(uint8_t status, void* user_data)
    ) {
        assert(context);
        assert(message);
        assert(message_size > 0);
        assert(peer_id);
        assert(protocol);
        EM_ASM({
            let context = $0;
            let message = $1;
            let message_size = $2;
            let peer_id = UTF8ToString($3);
            let protocol = UTF8ToString($4);
            let user_data = $5;
            let callback = $6;
            __internalPosemeshNetworking.posemeshNetworkingContextSendMessage(
                context, new Uint8Array(HEAPU8.buffer, message, message_size), peer_id, protocol
            ).then(function(status) {
                if (callback) {
                    dynCall('vip', callback, [status ? 1 : 0, user_data]);
                }
            }).catch(function(error) {
                console.error('psm_posemesh_networking_context_send_message():', error.message);
                if (callback) {
                    dynCall('vip', callback, [0, user_data]);
                }
            });
        }, context, message, message_size, peer_id, protocol, user_data, callback);
        return 1;
    }
#endif

#if defined(__cplusplus)
}
#endif

#endif // __POSEMESH_NETWORKING_API_H__
