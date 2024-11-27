#ifndef __POSEMESH_C_POSEMESH_H__
#define __POSEMESH_C_POSEMESH_H__

#include "API.h"
#include "Config.h"

#if defined(__cplusplus)
namespace psm { class Posemesh; }
typedef psm::Posemesh psm_posemesh_t;
#else
typedef struct psm_posemesh psm_posemesh_t;
#endif

#if defined(__cplusplus)
extern "C" {
#endif

psm_posemesh_t* PSM_API psm_posemesh_create();
psm_posemesh_t* PSM_API psm_posemesh_create_with_config(const psm_config_t* config);
uint8_t PSM_API psm_posemesh_send_message(
    const psm_posemesh_t* posemesh,
    const void* message,
    uint32_t message_size,
    const char* peer_id,
    const char* protocol,
    void* user_data,
    void (*callback)(uint8_t status, void* user_data)
);
uint8_t PSM_API psm_posemesh_send_string(
    const psm_posemesh_t* posemesh,
    const char* string,
    uint8_t append_terminating_null_character,
    const char* peer_id,
    const char* protocol,
    void* user_data,
    void (*callback)(uint8_t status, void* user_data)
);
void PSM_API psm_posemesh_destroy(psm_posemesh_t* posemesh);

const char* PSM_API psm_posemesh_get_version();
const char* PSM_API psm_posemesh_get_commit_id();

#if defined(__cplusplus)
}
#endif

#endif // __POSEMESH_C_POSEMESH_H__
