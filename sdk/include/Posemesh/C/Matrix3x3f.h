/* This code is automatically generated from Matrix3x3f.json interface. Do not modify it manually as it will be overwritten! */

#ifndef __POSEMESH_C_MATRIX3X3F_H__
#define __POSEMESH_C_MATRIX3X3F_H__

#include <stddef.h>
#include <stdint.h>

#include "API.h"

#if defined(__cplusplus)
#include <memory>
namespace psm {
class Matrix3x3f;
}
typedef psm::Matrix3x3f psm_matrix3x3f_t;
typedef std::shared_ptr<psm_matrix3x3f_t> psm_matrix3x3f_ref_t;
#else
typedef struct psm_matrix3x3f psm_matrix3x3f_t;
typedef struct psm_matrix3x3f_ref psm_matrix3x3f_ref_t;
#endif

#if defined(__cplusplus)
extern "C" {
#endif

psm_matrix3x3f_t* PSM_API psm_matrix3x3f_create();
psm_matrix3x3f_t* PSM_API psm_matrix3x3f_duplicate(const psm_matrix3x3f_t* matrix3x3f);
void PSM_API psm_matrix3x3f_destroy(psm_matrix3x3f_t* matrix3x3f);

uint8_t PSM_API psm_matrix3x3f_equals(const psm_matrix3x3f_t* matrix3x3f, const psm_matrix3x3f_t* other_matrix3x3f);
size_t PSM_API psm_matrix3x3f_hash(const psm_matrix3x3f_t* matrix3x3f);

float PSM_API psm_matrix3x3f_get_m00(const psm_matrix3x3f_t* matrix3x3f);
void PSM_API psm_matrix3x3f_set_m00(psm_matrix3x3f_t* matrix3x3f, float m00);
float PSM_API psm_matrix3x3f_get_m01(const psm_matrix3x3f_t* matrix3x3f);
void PSM_API psm_matrix3x3f_set_m01(psm_matrix3x3f_t* matrix3x3f, float m01);
float PSM_API psm_matrix3x3f_get_m02(const psm_matrix3x3f_t* matrix3x3f);
void PSM_API psm_matrix3x3f_set_m02(psm_matrix3x3f_t* matrix3x3f, float m02);
float PSM_API psm_matrix3x3f_get_m03(const psm_matrix3x3f_t* matrix3x3f);
void PSM_API psm_matrix3x3f_set_m03(psm_matrix3x3f_t* matrix3x3f, float m03);
float PSM_API psm_matrix3x3f_get_m10(const psm_matrix3x3f_t* matrix3x3f);
void PSM_API psm_matrix3x3f_set_m10(psm_matrix3x3f_t* matrix3x3f, float m10);
float PSM_API psm_matrix3x3f_get_m11(const psm_matrix3x3f_t* matrix3x3f);
void PSM_API psm_matrix3x3f_set_m11(psm_matrix3x3f_t* matrix3x3f, float m11);
float PSM_API psm_matrix3x3f_get_m12(const psm_matrix3x3f_t* matrix3x3f);
void PSM_API psm_matrix3x3f_set_m12(psm_matrix3x3f_t* matrix3x3f, float m12);
float PSM_API psm_matrix3x3f_get_m13(const psm_matrix3x3f_t* matrix3x3f);
void PSM_API psm_matrix3x3f_set_m13(psm_matrix3x3f_t* matrix3x3f, float m13);
float PSM_API psm_matrix3x3f_get_m20(const psm_matrix3x3f_t* matrix3x3f);
void PSM_API psm_matrix3x3f_set_m20(psm_matrix3x3f_t* matrix3x3f, float m20);
float PSM_API psm_matrix3x3f_get_m21(const psm_matrix3x3f_t* matrix3x3f);
void PSM_API psm_matrix3x3f_set_m21(psm_matrix3x3f_t* matrix3x3f, float m21);
float PSM_API psm_matrix3x3f_get_m22(const psm_matrix3x3f_t* matrix3x3f);
void PSM_API psm_matrix3x3f_set_m22(psm_matrix3x3f_t* matrix3x3f, float m22);
float PSM_API psm_matrix3x3f_get_m23(const psm_matrix3x3f_t* matrix3x3f);
void PSM_API psm_matrix3x3f_set_m23(psm_matrix3x3f_t* matrix3x3f, float m23);
float PSM_API psm_matrix3x3f_get_m30(const psm_matrix3x3f_t* matrix3x3f);
void PSM_API psm_matrix3x3f_set_m30(psm_matrix3x3f_t* matrix3x3f, float m30);
float PSM_API psm_matrix3x3f_get_m31(const psm_matrix3x3f_t* matrix3x3f);
void PSM_API psm_matrix3x3f_set_m31(psm_matrix3x3f_t* matrix3x3f, float m31);
float PSM_API psm_matrix3x3f_get_m32(const psm_matrix3x3f_t* matrix3x3f);
void PSM_API psm_matrix3x3f_set_m32(psm_matrix3x3f_t* matrix3x3f, float m32);
float PSM_API psm_matrix3x3f_get_m33(const psm_matrix3x3f_t* matrix3x3f);
void PSM_API psm_matrix3x3f_set_m33(psm_matrix3x3f_t* matrix3x3f, float m33);

psm_matrix3x3f_ref_t* PSM_API psm_matrix3x3f_ref_make(psm_matrix3x3f_t* matrix3x3f);
psm_matrix3x3f_ref_t* PSM_API psm_matrix3x3f_ref_clone(const psm_matrix3x3f_ref_t* matrix3x3f_ref);
psm_matrix3x3f_t* PSM_API psm_matrix3x3f_ref_get(const psm_matrix3x3f_ref_t* matrix3x3f_ref);
void PSM_API psm_matrix3x3f_ref_delete(psm_matrix3x3f_ref_t* matrix3x3f_ref);

#if defined(__cplusplus)
}
#endif

#endif // __POSEMESH_C_MATRIX3X3F_H__
