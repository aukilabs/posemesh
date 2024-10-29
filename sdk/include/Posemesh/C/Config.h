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
psm_config_t* PSM_API psm_config_duplicate(const psm_config_t* config);
void PSM_API psm_config_destroy(psm_config_t* config);

#if !defined(__EMSCRIPTEN__)
    uint8_t PSM_API psm_config_get_serve_as_bootstrap(const psm_config_t* config);
    void PSM_API psm_config_set_serve_as_bootstrap(psm_config_t* config, uint8_t serve_as_bootstrap);

    uint8_t PSM_API psm_config_get_serve_as_relay(const psm_config_t* config);
    void PSM_API psm_config_set_serve_as_relay(psm_config_t* config, uint8_t serve_as_relay);
#endif

#if defined(__cplusplus)
}
#endif

#endif // __POSEMESH_C_CONFIG_H__
