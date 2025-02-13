/* This code is automatically generated from Vector3f.json interface. Do not modify it manually as it will be overwritten! */

#ifndef __POSEMESH_C_VECTOR3F_H__
#define __POSEMESH_C_VECTOR3F_H__

#include <stddef.h>
#include <stdint.h>

#include "API.hpp"

#if defined(__cplusplus)
namespace psm {
class Vector3f;
}
typedef psm::Vector3f psm_vector3f_t;
#else
typedef struct psm_vector3f psm_vector3f_t;
#endif

#if defined(__cplusplus)
extern "C" {
#endif

psm_vector3f_t* PSM_API psm_vector3f_create();
psm_vector3f_t* PSM_API psm_vector3f_duplicate(const psm_vector3f_t* vector3f);
void PSM_API psm_vector3f_destroy(psm_vector3f_t* vector3f);

uint8_t PSM_API psm_vector3f_equals(const psm_vector3f_t* vector3f, const psm_vector3f_t* other_vector3f);
size_t PSM_API psm_vector3f_hash(const psm_vector3f_t* vector3f);

float PSM_API psm_vector3f_get_x(const psm_vector3f_t* vector3f);
void PSM_API psm_vector3f_set_x(psm_vector3f_t* vector3f, float x);
float PSM_API psm_vector3f_get_y(const psm_vector3f_t* vector3f);
void PSM_API psm_vector3f_set_y(psm_vector3f_t* vector3f, float y);
float PSM_API psm_vector3f_get_z(const psm_vector3f_t* vector3f);
void PSM_API psm_vector3f_set_z(psm_vector3f_t* vector3f, float z);

#if defined(__cplusplus)
}
#endif

#endif // __POSEMESH_C_VECTOR3F_H__
