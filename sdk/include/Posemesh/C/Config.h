#ifndef __POSEMESH_C_CONFIG_H__
#define __POSEMESH_C_CONFIG_H__

#include <stdint.h>

#include "API.h"

#if defined(__cplusplus)
namespace psm { class Config; }
typedef psm::Config psm_config_t;
#else
typedef struct psm_config psm_config_t;
#endif

#if defined(__cplusplus)
extern "C" {
#endif

psm_config_t* PSM_API psm_config_create();
psm_config_t* PSM_API psm_config_create_default();
psm_config_t* PSM_API psm_config_duplicate(const psm_config_t* config);
uint8_t PSM_API psm_config_equals(const psm_config_t* config, const psm_config_t* other_config);
void PSM_API psm_config_destroy(psm_config_t* config);

#if !defined(__EMSCRIPTEN__)
    uint8_t PSM_API psm_config_get_serve_as_bootstrap(const psm_config_t* config);
    void PSM_API psm_config_set_serve_as_bootstrap(psm_config_t* config, uint8_t serve_as_bootstrap);

    uint8_t PSM_API psm_config_get_serve_as_relay(const psm_config_t* config);
    void PSM_API psm_config_set_serve_as_relay(psm_config_t* config, uint8_t serve_as_relay);
#endif

const char* const* PSM_API psm_config_get_bootstraps(const psm_config_t* config, uint32_t* out_bootstraps_count);
void PSM_API psm_config_get_bootstraps_free(const char* const* bootstraps);
uint8_t PSM_API psm_config_set_bootstraps(psm_config_t* config, const char* const* bootstraps, uint32_t bootstraps_count);

const char* const* PSM_API psm_config_get_relays(const psm_config_t* config, uint32_t* out_relays_count);
void PSM_API psm_config_get_relays_free(const char* const* relays);
uint8_t PSM_API psm_config_set_relays(psm_config_t* config, const char* const* relays, uint32_t relays_count);

#if defined(__cplusplus)
}
#endif

#endif // __POSEMESH_C_CONFIG_H__
