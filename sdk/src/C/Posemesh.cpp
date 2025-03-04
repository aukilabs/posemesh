#include <Posemesh/C/Posemesh.h>
#include <Posemesh/Networking/API.h>
#include <Posemesh/Posemesh.hpp>
#include <cassert>
#include <cstring>
#include <new>

extern "C" {

psm_posemesh_t* psm_posemesh_create()
{
    return new (std::nothrow) psm::Posemesh;
}

psm_posemesh_t* psm_posemesh_create_with_config(const psm_config_t* config)
{
    if (!config) {
        assert(!"psm_posemesh_create_with_config(): config is null");
        return nullptr;
    }
    return new (std::nothrow) psm::Posemesh(*config);
}

uint8_t psm_posemesh_send_message(
    const psm_posemesh_t* posemesh,
    const void* message,
    uint32_t message_size,
    const char* peer_id,
    const char* protocol,
    void* user_data,
    void (*callback)(uint8_t status, void* user_data))
{
    if (!posemesh) {
        assert(!"psm_posemesh_send_message(): posemesh is null");
        return 0;
    }
    if (!message) {
        assert(!"psm_posemesh_send_message(): message is null");
        return 0;
    }
    if (message_size == 0) {
        assert(!"psm_posemesh_send_message(): message_size is zero");
        return 0;
    }
    if (!peer_id) {
        assert(!"psm_posemesh_send_message(): peer_id is null");
        return 0;
    }
    if (!protocol) {
        assert(!"psm_posemesh_send_message(): protocol is null");
        return 0;
    }
    auto* context = reinterpret_cast<psm_posemesh_networking_context_t*>(posemesh->__getContext());
    if (!context) {
        assert(!"psm_posemesh_send_message(): context is null");
        return 0;
    }
    return psm_posemesh_networking_context_send_message(
        context,
        message,
        message_size,
        peer_id,
        protocol,
        user_data,
        0,
        callback);
}

uint8_t psm_posemesh_send_string(
    const psm_posemesh_t* posemesh,
    const char* string,
    uint8_t append_terminating_null_character,
    const char* peer_id,
    const char* protocol,
    void* user_data,
    void (*callback)(uint8_t status, void* user_data))
{
    if (!string) {
        assert(!"psm_posemesh_send_string(): string is null");
        return 0;
    }
    const auto length = std::strlen(string);
    return psm_posemesh_send_message(
        posemesh,
        string,
        static_cast<std::uint32_t>(length + (append_terminating_null_character ? 1 : 0)),
        peer_id,
        protocol,
        user_data,
        callback);
}

void psm_posemesh_destroy(psm_posemesh_t* posemesh)
{
    delete posemesh;
}

const char* psm_posemesh_get_version()
{
    return POSEMESH_VERSION;
}

const char* psm_posemesh_get_commit_id()
{
    return POSEMESH_COMMIT_ID;
}
}
