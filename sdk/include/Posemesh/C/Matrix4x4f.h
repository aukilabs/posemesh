/* This code is automatically generated from Matrix4x4f.json interface. Do not modify it manually as it will be overwritten! */

#ifndef __POSEMESH_C_MATRIX4X4F_H__
#define __POSEMESH_C_MATRIX4X4F_H__

#include <stddef.h>
#include <stdint.h>

#include "API.hpp"

#if defined(__cplusplus)
namespace psm {
class Matrix4x4f;
}
typedef psm::Matrix4x4f psm_matrix4x4f_t;
#else
typedef struct psm_matrix4x4f psm_matrix4x4f_t;
#endif

#if defined(__cplusplus)
extern "C" {
#endif

psm_matrix4x4f_t* PSM_API psm_matrix4x4f_create();
psm_matrix4x4f_t* PSM_API psm_matrix4x4f_duplicate(const psm_matrix4x4f_t* matrix4x4f);
void PSM_API psm_matrix4x4f_destroy(psm_matrix4x4f_t* matrix4x4f);

uint8_t PSM_API psm_matrix4x4f_equals(const psm_matrix4x4f_t* matrix4x4f, const psm_matrix4x4f_t* other_matrix4x4f);
size_t PSM_API psm_matrix4x4f_hash(const psm_matrix4x4f_t* matrix4x4f);

float PSM_API psm_matrix4x4f_get_m00(const psm_matrix4x4f_t* matrix4x4f);
void PSM_API psm_matrix4x4f_set_m00(psm_matrix4x4f_t* matrix4x4f, float m00);
float PSM_API psm_matrix4x4f_get_m01(const psm_matrix4x4f_t* matrix4x4f);
void PSM_API psm_matrix4x4f_set_m01(psm_matrix4x4f_t* matrix4x4f, float m01);
float PSM_API psm_matrix4x4f_get_m02(const psm_matrix4x4f_t* matrix4x4f);
void PSM_API psm_matrix4x4f_set_m02(psm_matrix4x4f_t* matrix4x4f, float m02);
float PSM_API psm_matrix4x4f_get_m03(const psm_matrix4x4f_t* matrix4x4f);
void PSM_API psm_matrix4x4f_set_m03(psm_matrix4x4f_t* matrix4x4f, float m03);
float PSM_API psm_matrix4x4f_get_m04(const psm_matrix4x4f_t* matrix4x4f);
void PSM_API psm_matrix4x4f_set_m04(psm_matrix4x4f_t* matrix4x4f, float m04);
float PSM_API psm_matrix4x4f_get_m10(const psm_matrix4x4f_t* matrix4x4f);
void PSM_API psm_matrix4x4f_set_m10(psm_matrix4x4f_t* matrix4x4f, float m10);
float PSM_API psm_matrix4x4f_get_m11(const psm_matrix4x4f_t* matrix4x4f);
void PSM_API psm_matrix4x4f_set_m11(psm_matrix4x4f_t* matrix4x4f, float m11);
float PSM_API psm_matrix4x4f_get_m12(const psm_matrix4x4f_t* matrix4x4f);
void PSM_API psm_matrix4x4f_set_m12(psm_matrix4x4f_t* matrix4x4f, float m12);
float PSM_API psm_matrix4x4f_get_m13(const psm_matrix4x4f_t* matrix4x4f);
void PSM_API psm_matrix4x4f_set_m13(psm_matrix4x4f_t* matrix4x4f, float m13);
float PSM_API psm_matrix4x4f_get_m14(const psm_matrix4x4f_t* matrix4x4f);
void PSM_API psm_matrix4x4f_set_m14(psm_matrix4x4f_t* matrix4x4f, float m14);
float PSM_API psm_matrix4x4f_get_m20(const psm_matrix4x4f_t* matrix4x4f);
void PSM_API psm_matrix4x4f_set_m20(psm_matrix4x4f_t* matrix4x4f, float m20);
float PSM_API psm_matrix4x4f_get_m21(const psm_matrix4x4f_t* matrix4x4f);
void PSM_API psm_matrix4x4f_set_m21(psm_matrix4x4f_t* matrix4x4f, float m21);
float PSM_API psm_matrix4x4f_get_m22(const psm_matrix4x4f_t* matrix4x4f);
void PSM_API psm_matrix4x4f_set_m22(psm_matrix4x4f_t* matrix4x4f, float m22);
float PSM_API psm_matrix4x4f_get_m23(const psm_matrix4x4f_t* matrix4x4f);
void PSM_API psm_matrix4x4f_set_m23(psm_matrix4x4f_t* matrix4x4f, float m23);
float PSM_API psm_matrix4x4f_get_m24(const psm_matrix4x4f_t* matrix4x4f);
void PSM_API psm_matrix4x4f_set_m24(psm_matrix4x4f_t* matrix4x4f, float m24);
float PSM_API psm_matrix4x4f_get_m30(const psm_matrix4x4f_t* matrix4x4f);
void PSM_API psm_matrix4x4f_set_m30(psm_matrix4x4f_t* matrix4x4f, float m30);
float PSM_API psm_matrix4x4f_get_m31(const psm_matrix4x4f_t* matrix4x4f);
void PSM_API psm_matrix4x4f_set_m31(psm_matrix4x4f_t* matrix4x4f, float m31);
float PSM_API psm_matrix4x4f_get_m32(const psm_matrix4x4f_t* matrix4x4f);
void PSM_API psm_matrix4x4f_set_m32(psm_matrix4x4f_t* matrix4x4f, float m32);
float PSM_API psm_matrix4x4f_get_m33(const psm_matrix4x4f_t* matrix4x4f);
void PSM_API psm_matrix4x4f_set_m33(psm_matrix4x4f_t* matrix4x4f, float m33);
float PSM_API psm_matrix4x4f_get_m34(const psm_matrix4x4f_t* matrix4x4f);
void PSM_API psm_matrix4x4f_set_m34(psm_matrix4x4f_t* matrix4x4f, float m34);
float PSM_API psm_matrix4x4f_get_m40(const psm_matrix4x4f_t* matrix4x4f);
void PSM_API psm_matrix4x4f_set_m40(psm_matrix4x4f_t* matrix4x4f, float m40);
float PSM_API psm_matrix4x4f_get_m41(const psm_matrix4x4f_t* matrix4x4f);
void PSM_API psm_matrix4x4f_set_m41(psm_matrix4x4f_t* matrix4x4f, float m41);
float PSM_API psm_matrix4x4f_get_m42(const psm_matrix4x4f_t* matrix4x4f);
void PSM_API psm_matrix4x4f_set_m42(psm_matrix4x4f_t* matrix4x4f, float m42);
float PSM_API psm_matrix4x4f_get_m43(const psm_matrix4x4f_t* matrix4x4f);
void PSM_API psm_matrix4x4f_set_m43(psm_matrix4x4f_t* matrix4x4f, float m43);
float PSM_API psm_matrix4x4f_get_m44(const psm_matrix4x4f_t* matrix4x4f);
void PSM_API psm_matrix4x4f_set_m44(psm_matrix4x4f_t* matrix4x4f, float m44);

#if defined(__cplusplus)
}
#endif

#endif // __POSEMESH_C_MATRIX4X4F_H__
