/* This code is automatically generated from Matrix3x3f.json interface. Do not modify it manually as it will be overwritten! */

#include <Posemesh/C/Matrix3x3f.h>
#include <Posemesh/Matrix3x3f.hpp>
#include <cassert>
#include <new>

psm_matrix3x3f_t* psm_matrix3x3f_create()
{
    return new (std::nothrow) psm::Matrix3x3f;
}

psm_matrix3x3f_t* psm_matrix3x3f_duplicate(const psm_matrix3x3f_t* matrix3x3f)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_duplicate(): matrix3x3f is null");
        return nullptr;
    }
    return new (std::nothrow) psm::Matrix3x3f(*matrix3x3f);
}

void psm_matrix3x3f_destroy(psm_matrix3x3f_t* matrix3x3f)
{
    delete matrix3x3f;
}

uint8_t psm_matrix3x3f_equals(const psm_matrix3x3f_t* matrix3x3f, const psm_matrix3x3f_t* other_matrix3x3f)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_equals(): matrix3x3f is null");
        return 0;
    }
    if (!other_matrix3x3f) {
        assert(!"psm_matrix3x3f_equals(): other_matrix3x3f is null");
        return 0;
    }
    return static_cast<uint8_t>(matrix3x3f->operator==(*other_matrix3x3f));
}

size_t psm_matrix3x3f_hash(const psm_matrix3x3f_t* matrix3x3f)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_hash(): matrix3x3f is null");
        return 0;
    }
    return std::hash<psm::Matrix3x3f> {}(*matrix3x3f);
}

float psm_matrix3x3f_get_m00(const psm_matrix3x3f_t* matrix3x3f)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_get_m00(): matrix3x3f is null");
        return 0.0f;
    }
    return matrix3x3f->getM00();
}

void psm_matrix3x3f_set_m00(psm_matrix3x3f_t* matrix3x3f, float m00)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_set_m00(): matrix3x3f is null");
        return;
    }
    matrix3x3f->setM00(m00);
}

float psm_matrix3x3f_get_m01(const psm_matrix3x3f_t* matrix3x3f)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_get_m01(): matrix3x3f is null");
        return 0.0f;
    }
    return matrix3x3f->getM01();
}

void psm_matrix3x3f_set_m01(psm_matrix3x3f_t* matrix3x3f, float m01)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_set_m01(): matrix3x3f is null");
        return;
    }
    matrix3x3f->setM01(m01);
}

float psm_matrix3x3f_get_m02(const psm_matrix3x3f_t* matrix3x3f)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_get_m02(): matrix3x3f is null");
        return 0.0f;
    }
    return matrix3x3f->getM02();
}

void psm_matrix3x3f_set_m02(psm_matrix3x3f_t* matrix3x3f, float m02)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_set_m02(): matrix3x3f is null");
        return;
    }
    matrix3x3f->setM02(m02);
}

float psm_matrix3x3f_get_m03(const psm_matrix3x3f_t* matrix3x3f)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_get_m03(): matrix3x3f is null");
        return 0.0f;
    }
    return matrix3x3f->getM03();
}

void psm_matrix3x3f_set_m03(psm_matrix3x3f_t* matrix3x3f, float m03)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_set_m03(): matrix3x3f is null");
        return;
    }
    matrix3x3f->setM03(m03);
}

float psm_matrix3x3f_get_m10(const psm_matrix3x3f_t* matrix3x3f)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_get_m10(): matrix3x3f is null");
        return 0.0f;
    }
    return matrix3x3f->getM10();
}

void psm_matrix3x3f_set_m10(psm_matrix3x3f_t* matrix3x3f, float m10)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_set_m10(): matrix3x3f is null");
        return;
    }
    matrix3x3f->setM10(m10);
}

float psm_matrix3x3f_get_m11(const psm_matrix3x3f_t* matrix3x3f)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_get_m11(): matrix3x3f is null");
        return 0.0f;
    }
    return matrix3x3f->getM11();
}

void psm_matrix3x3f_set_m11(psm_matrix3x3f_t* matrix3x3f, float m11)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_set_m11(): matrix3x3f is null");
        return;
    }
    matrix3x3f->setM11(m11);
}

float psm_matrix3x3f_get_m12(const psm_matrix3x3f_t* matrix3x3f)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_get_m12(): matrix3x3f is null");
        return 0.0f;
    }
    return matrix3x3f->getM12();
}

void psm_matrix3x3f_set_m12(psm_matrix3x3f_t* matrix3x3f, float m12)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_set_m12(): matrix3x3f is null");
        return;
    }
    matrix3x3f->setM12(m12);
}

float psm_matrix3x3f_get_m13(const psm_matrix3x3f_t* matrix3x3f)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_get_m13(): matrix3x3f is null");
        return 0.0f;
    }
    return matrix3x3f->getM13();
}

void psm_matrix3x3f_set_m13(psm_matrix3x3f_t* matrix3x3f, float m13)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_set_m13(): matrix3x3f is null");
        return;
    }
    matrix3x3f->setM13(m13);
}

float psm_matrix3x3f_get_m20(const psm_matrix3x3f_t* matrix3x3f)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_get_m20(): matrix3x3f is null");
        return 0.0f;
    }
    return matrix3x3f->getM20();
}

void psm_matrix3x3f_set_m20(psm_matrix3x3f_t* matrix3x3f, float m20)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_set_m20(): matrix3x3f is null");
        return;
    }
    matrix3x3f->setM20(m20);
}

float psm_matrix3x3f_get_m21(const psm_matrix3x3f_t* matrix3x3f)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_get_m21(): matrix3x3f is null");
        return 0.0f;
    }
    return matrix3x3f->getM21();
}

void psm_matrix3x3f_set_m21(psm_matrix3x3f_t* matrix3x3f, float m21)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_set_m21(): matrix3x3f is null");
        return;
    }
    matrix3x3f->setM21(m21);
}

float psm_matrix3x3f_get_m22(const psm_matrix3x3f_t* matrix3x3f)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_get_m22(): matrix3x3f is null");
        return 0.0f;
    }
    return matrix3x3f->getM22();
}

void psm_matrix3x3f_set_m22(psm_matrix3x3f_t* matrix3x3f, float m22)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_set_m22(): matrix3x3f is null");
        return;
    }
    matrix3x3f->setM22(m22);
}

float psm_matrix3x3f_get_m23(const psm_matrix3x3f_t* matrix3x3f)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_get_m23(): matrix3x3f is null");
        return 0.0f;
    }
    return matrix3x3f->getM23();
}

void psm_matrix3x3f_set_m23(psm_matrix3x3f_t* matrix3x3f, float m23)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_set_m23(): matrix3x3f is null");
        return;
    }
    matrix3x3f->setM23(m23);
}

float psm_matrix3x3f_get_m30(const psm_matrix3x3f_t* matrix3x3f)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_get_m30(): matrix3x3f is null");
        return 0.0f;
    }
    return matrix3x3f->getM30();
}

void psm_matrix3x3f_set_m30(psm_matrix3x3f_t* matrix3x3f, float m30)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_set_m30(): matrix3x3f is null");
        return;
    }
    matrix3x3f->setM30(m30);
}

float psm_matrix3x3f_get_m31(const psm_matrix3x3f_t* matrix3x3f)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_get_m31(): matrix3x3f is null");
        return 0.0f;
    }
    return matrix3x3f->getM31();
}

void psm_matrix3x3f_set_m31(psm_matrix3x3f_t* matrix3x3f, float m31)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_set_m31(): matrix3x3f is null");
        return;
    }
    matrix3x3f->setM31(m31);
}

float psm_matrix3x3f_get_m32(const psm_matrix3x3f_t* matrix3x3f)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_get_m32(): matrix3x3f is null");
        return 0.0f;
    }
    return matrix3x3f->getM32();
}

void psm_matrix3x3f_set_m32(psm_matrix3x3f_t* matrix3x3f, float m32)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_set_m32(): matrix3x3f is null");
        return;
    }
    matrix3x3f->setM32(m32);
}

float psm_matrix3x3f_get_m33(const psm_matrix3x3f_t* matrix3x3f)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_get_m33(): matrix3x3f is null");
        return 0.0f;
    }
    return matrix3x3f->getM33();
}

void psm_matrix3x3f_set_m33(psm_matrix3x3f_t* matrix3x3f, float m33)
{
    if (!matrix3x3f) {
        assert(!"psm_matrix3x3f_set_m33(): matrix3x3f is null");
        return;
    }
    matrix3x3f->setM33(m33);
}
