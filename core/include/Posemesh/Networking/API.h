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
    const uint8_t* private_key;
    uint32_t private_key_size;
    #if !defined(__EMSCRIPTEN__)
        const char* private_key_path;
    #endif
} psm_posemesh_networking_config_t;

typedef struct psm_posemesh_networking_context psm_posemesh_networking_context_t;

#if defined(__cplusplus)
extern "C" {
#endif

#if !defined(__EMSCRIPTEN__)
    void psm_posemesh_networking_get_commit_id(char* buffer, unsigned int* size);
#else
    static void psm_posemesh_networking_get_commit_id(char* buffer, unsigned int* size) {
        assert(buffer);
        assert(size);
        EM_ASM({
            let buffer = $0;
            let size = $1;
            let maxSize = HEAPU32[size >> 2];
            if (maxSize == 0)
                return;
            let commitId = __internalPosemeshBase.posemeshNetworkingGetCommitId();
            let copySize = maxSize > 1 ? Math.min(lengthBytesUTF8(commitId), maxSize - 1) : 0;
            stringToUTF8(commitId, buffer, copySize + 1);
            HEAPU32[size >> 2] = copySize + 1;
        }, buffer, size);
    }
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
        const uint8_t* const private_key = config->private_key;
        const uint32_t private_key_size = config->private_key_size;
        assert(private_key || private_key_size == 0);
        void* context = EM_ASM_PTR({
            let bootstraps = UTF8ToString($0);
            let relays = UTF8ToString($1);
            let privateKey = $2;
            let privateKeySize = $3;
            let config = new __internalPosemeshBase.Config(
                bootstraps, relays, new Uint8Array(HEAPU8.buffer, privateKey, privateKeySize)
            );
            try {
                return __internalPosemeshBase.posemeshNetworkingContextCreate(config);
            } finally {
                config.free();
            }
        }, bootstraps, relays, private_key, private_key_size);
        return (psm_posemesh_networking_context_t*)context;
    }
#endif

#if !defined(__EMSCRIPTEN__)
    void psm_posemesh_networking_context_destroy(psm_posemesh_networking_context_t* context);
#else
    static void psm_posemesh_networking_context_destroy(psm_posemesh_networking_context_t* context) {
        assert(context);
        EM_ASM({
            __internalPosemeshBase.posemeshNetworkingContextDestroy($0);
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
        uint32_t timeout,
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
        uint32_t timeout,
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
            let messageSize = $2;
            let peerId = UTF8ToString($3);
            let protocol = UTF8ToString($4);
            let userData = $5;
            let timeout = $6;
            let callback = $7;
            __internalPosemeshBase.posemeshNetworkingContextSendMessage(
                context, new Uint8Array(HEAPU8.buffer, message, messageSize), peerId, protocol, timeout
            ).then(function(status) {
                if (callback) {
                    dynCall('vip', callback, [status ? 1 : 0, userData]);
                }
            }).catch(function(error) {
                console.error('psm_posemesh_networking_context_send_message():', error.message);
                if (callback) {
                    dynCall('vip', callback, [0, userData]);
                }
            });
        }, context, message, message_size, peer_id, protocol, user_data, timeout, callback);
        return 1;
    }
#endif

#if defined(__cplusplus)
}
#endif

#endif // __POSEMESH_NETWORKING_API_H__
