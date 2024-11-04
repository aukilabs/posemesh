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
void PSM_API psm_posemesh_destroy(psm_posemesh_t* posemesh);

#if defined(__cplusplus)
}
#endif

#endif // __POSEMESH_C_POSEMESH_H__
