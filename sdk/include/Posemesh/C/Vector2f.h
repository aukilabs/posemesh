/* This code is automatically generated from Vector2f.json interface. Do not modify it manually as it will be overwritten! */

#ifndef __POSEMESH_C_VECTOR2F_H__
#define __POSEMESH_C_VECTOR2F_H__

#include <stddef.h>
#include <stdint.h>

#include "API.h"

#if defined(__cplusplus)
namespace psm {
class Vector2f;
}
typedef psm::Vector2f psm_vector2f_t;
#else
typedef struct psm_vector2f psm_vector2f_t;
#endif

#if defined(__cplusplus)
extern "C" {
#endif

psm_vector2f_t* PSM_API psm_vector2f_create();
psm_vector2f_t* PSM_API psm_vector2f_duplicate(const psm_vector2f_t* vector2f);
void PSM_API psm_vector2f_destroy(psm_vector2f_t* vector2f);

uint8_t PSM_API psm_vector2f_equals(const psm_vector2f_t* vector2f, const psm_vector2f_t* other_vector2f);
size_t PSM_API psm_vector2f_hash(const psm_vector2f_t* vector2f);

float PSM_API psm_vector2f_get_x(const psm_vector2f_t* vector2f);
void PSM_API psm_vector2f_set_x(psm_vector2f_t* vector2f, float x);
float PSM_API psm_vector2f_get_y(const psm_vector2f_t* vector2f);
void PSM_API psm_vector2f_set_y(psm_vector2f_t* vector2f, float y);
float PSM_API psm_vector2f_get_length(const psm_vector2f_t* vector2f);

#if defined(__cplusplus)
}
#endif

#endif // __POSEMESH_C_VECTOR2F_H__
