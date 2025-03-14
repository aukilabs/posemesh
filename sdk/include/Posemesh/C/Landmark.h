#ifndef __POSEMESH_C_LANDMARK_H__
#define __POSEMESH_C_LANDMARK_H__

#include <stddef.h>
#include <stdint.h>

#include "API.h"
#include "Vector3.h"
#include "Posemesh/Vector3.hpp"

#if defined(__cplusplus)
#include <memory>
namespace psm {
class Landmark;
}
typedef psm::Landmark psm_landmark_t;
typedef std::shared_ptr<psm_landmark_t> psm_landmark_ref_t;
#else
typedef struct psm_landmark psm_landmark_t;
typedef struct psm_landmark_ref psm_landmark_ref_t;
#endif

#if defined(__cplusplus)
extern "C" {
#endif

psm_landmark_t* PSM_API psm_landmark_create();
psm_landmark_t* PSM_API psm_landmark_duplicate(const psm_landmark_t* landmark);
void PSM_API psm_landmark_destroy(psm_landmark_t* landmark);

uint8_t PSM_API psm_landmark_equals(const psm_landmark_t* landmark, const psm_landmark_t* other_landmark);
size_t PSM_API psm_landmark_hash(const psm_landmark_t* landmark);

const char* PSM_API psm_landmark_get_type(const psm_landmark_t* landmark);
void PSM_API psm_landmark_get_type_free(const char* type);
void PSM_API psm_landmark_set_type(psm_landmark_t* landmark, const char* type);
const char* PSM_API psm_landmark_get_id(const psm_landmark_t* landmark);
void PSM_API psm_landmark_get_id_free(const char* id);
void PSM_API psm_landmark_set_id(psm_landmark_t* landmark, const char* id);
psm_vector3_t* PSM_API psm_landmark_get_position(const psm_landmark_t* landmark);
void PSM_API psm_landmark_get_position_free(psm_vector3_t* position);
void PSM_API psm_landmark_set_position(psm_landmark_t* landmark, psm_vector3_t* position);

psm_landmark_ref_t* PSM_API psm_landmark_ref_make(psm_landmark_t* landmark);
psm_landmark_ref_t* PSM_API psm_landmark_ref_clone(const psm_landmark_ref_t* landmark_ref);
psm_landmark_t* PSM_API psm_landmark_ref_get(const psm_landmark_ref_t* landmark_ref);
void PSM_API psm_landmark_ref_delete(psm_landmark_ref_t* landmark_ref);

#if defined(__cplusplus)
}
#endif

#endif // __POSEMESH_C_LANDMARK_H__
