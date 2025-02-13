/* This code is automatically generated from Vector4f.json interface. Do not modify it manually as it will be overwritten! */

#ifndef __POSEMESH_C_VECTOR4F_H__
#define __POSEMESH_C_VECTOR4F_H__

#include <stddef.h>
#include <stdint.h>

#include "API.hpp"

#if defined(__cplusplus)
namespace psm {
class Vector4f;
}
typedef psm::Vector4f psm_vector4f_t;
#else
typedef struct psm_vector4f psm_vector4f_t;
#endif
typedef psm_vector4f_t psm_quaternion_t;

#if defined(__cplusplus)
extern "C" {
#endif

psm_vector4f_t* PSM_API psm_vector4f_create();
psm_vector4f_t* PSM_API psm_vector4f_duplicate(const psm_vector4f_t* vector4f);
void PSM_API psm_vector4f_destroy(psm_vector4f_t* vector4f);

uint8_t PSM_API psm_vector4f_equals(const psm_vector4f_t* vector4f, const psm_vector4f_t* other_vector4f);
size_t PSM_API psm_vector4f_hash(const psm_vector4f_t* vector4f);

float PSM_API psm_vector4f_get_x(const psm_vector4f_t* vector4f);
void PSM_API psm_vector4f_set_x(psm_vector4f_t* vector4f, float x);
float PSM_API psm_vector4f_get_y(const psm_vector4f_t* vector4f);
void PSM_API psm_vector4f_set_y(psm_vector4f_t* vector4f, float y);
float PSM_API psm_vector4f_get_z(const psm_vector4f_t* vector4f);
void PSM_API psm_vector4f_set_z(psm_vector4f_t* vector4f, float z);
float PSM_API psm_vector4f_get_w(const psm_vector4f_t* vector4f);
void PSM_API psm_vector4f_set_w(psm_vector4f_t* vector4f, float w);

#if defined(__cplusplus)
}
#endif

#define psm_quaternion_create() (psm_vector4f_create())
#define psm_quaternion_duplicate(_vector4f) (psm_vector4f_duplicate((_vector4f)))
#define psm_quaternion_destroy(_vector4f) (psm_vector4f_destroy((_vector4f)))
#define psm_quaternion_equals(_vector4f, _other_vector4f) (psm_vector4f_equals((_vector4f), (_other_vector4f)))
#define psm_quaternion_hash(_vector4f) (psm_vector4f_hash((_vector4f)))
#define psm_quaternion_get_x(_vector4f) (psm_vector4f_get_x((_vector4f)))
#define psm_quaternion_set_x(_vector4f, _x) (psm_vector4f_set_x((_vector4f), (_x)))
#define psm_quaternion_get_y(_vector4f) (psm_vector4f_get_y((_vector4f)))
#define psm_quaternion_set_y(_vector4f, _y) (psm_vector4f_set_y((_vector4f), (_y)))
#define psm_quaternion_get_z(_vector4f) (psm_vector4f_get_z((_vector4f)))
#define psm_quaternion_set_z(_vector4f, _z) (psm_vector4f_set_z((_vector4f), (_z)))
#define psm_quaternion_get_w(_vector4f) (psm_vector4f_get_w((_vector4f)))
#define psm_quaternion_set_w(_vector4f, _w) (psm_vector4f_set_w((_vector4f), (_w)))

#endif // __POSEMESH_C_VECTOR4F_H__
