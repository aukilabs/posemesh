/* This code is automatically generated from Matrix4x4f.json interface. Do not modify it manually as it will be overwritten! */

#include <Posemesh/C/Matrix4x4f.h>
#include <Posemesh/Matrix4x4f.hpp>
#include <cassert>
#include <new>

psm_matrix4x4f_t* psm_matrix4x4f_create()
{
    return new (std::nothrow) psm::Matrix4x4f;
}

psm_matrix4x4f_t* psm_matrix4x4f_duplicate(const psm_matrix4x4f_t* matrix4x4f)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_duplicate(): matrix4x4f is null");
        return nullptr;
    }
    return new (std::nothrow) psm::Matrix4x4f(*matrix4x4f);
}

void psm_matrix4x4f_destroy(psm_matrix4x4f_t* matrix4x4f)
{
    delete matrix4x4f;
}

uint8_t psm_matrix4x4f_equals(const psm_matrix4x4f_t* matrix4x4f, const psm_matrix4x4f_t* other_matrix4x4f)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_equals(): matrix4x4f is null");
        return 0;
    }
    if (!other_matrix4x4f) {
        assert(!"psm_matrix4x4f_equals(): other_matrix4x4f is null");
        return 0;
    }
    return static_cast<uint8_t>(matrix4x4f->operator==(*other_matrix4x4f));
}

size_t psm_matrix4x4f_hash(const psm_matrix4x4f_t* matrix4x4f)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_hash(): matrix4x4f is null");
        return 0;
    }
    return std::hash<psm::Matrix4x4f> {}(*matrix4x4f);
}

float psm_matrix4x4f_get_m00(const psm_matrix4x4f_t* matrix4x4f)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_get_m00(): matrix4x4f is null");
        return 0.0f;
    }
    return matrix4x4f->getM00();
}

void psm_matrix4x4f_set_m00(psm_matrix4x4f_t* matrix4x4f, float m00)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_set_m00(): matrix4x4f is null");
        return;
    }
    matrix4x4f->setM00(m00);
}

float psm_matrix4x4f_get_m01(const psm_matrix4x4f_t* matrix4x4f)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_get_m01(): matrix4x4f is null");
        return 0.0f;
    }
    return matrix4x4f->getM01();
}

void psm_matrix4x4f_set_m01(psm_matrix4x4f_t* matrix4x4f, float m01)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_set_m01(): matrix4x4f is null");
        return;
    }
    matrix4x4f->setM01(m01);
}

float psm_matrix4x4f_get_m02(const psm_matrix4x4f_t* matrix4x4f)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_get_m02(): matrix4x4f is null");
        return 0.0f;
    }
    return matrix4x4f->getM02();
}

void psm_matrix4x4f_set_m02(psm_matrix4x4f_t* matrix4x4f, float m02)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_set_m02(): matrix4x4f is null");
        return;
    }
    matrix4x4f->setM02(m02);
}

float psm_matrix4x4f_get_m03(const psm_matrix4x4f_t* matrix4x4f)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_get_m03(): matrix4x4f is null");
        return 0.0f;
    }
    return matrix4x4f->getM03();
}

void psm_matrix4x4f_set_m03(psm_matrix4x4f_t* matrix4x4f, float m03)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_set_m03(): matrix4x4f is null");
        return;
    }
    matrix4x4f->setM03(m03);
}

float psm_matrix4x4f_get_m04(const psm_matrix4x4f_t* matrix4x4f)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_get_m04(): matrix4x4f is null");
        return 0.0f;
    }
    return matrix4x4f->getM04();
}

void psm_matrix4x4f_set_m04(psm_matrix4x4f_t* matrix4x4f, float m04)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_set_m04(): matrix4x4f is null");
        return;
    }
    matrix4x4f->setM04(m04);
}

float psm_matrix4x4f_get_m10(const psm_matrix4x4f_t* matrix4x4f)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_get_m10(): matrix4x4f is null");
        return 0.0f;
    }
    return matrix4x4f->getM10();
}

void psm_matrix4x4f_set_m10(psm_matrix4x4f_t* matrix4x4f, float m10)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_set_m10(): matrix4x4f is null");
        return;
    }
    matrix4x4f->setM10(m10);
}

float psm_matrix4x4f_get_m11(const psm_matrix4x4f_t* matrix4x4f)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_get_m11(): matrix4x4f is null");
        return 0.0f;
    }
    return matrix4x4f->getM11();
}

void psm_matrix4x4f_set_m11(psm_matrix4x4f_t* matrix4x4f, float m11)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_set_m11(): matrix4x4f is null");
        return;
    }
    matrix4x4f->setM11(m11);
}

float psm_matrix4x4f_get_m12(const psm_matrix4x4f_t* matrix4x4f)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_get_m12(): matrix4x4f is null");
        return 0.0f;
    }
    return matrix4x4f->getM12();
}

void psm_matrix4x4f_set_m12(psm_matrix4x4f_t* matrix4x4f, float m12)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_set_m12(): matrix4x4f is null");
        return;
    }
    matrix4x4f->setM12(m12);
}

float psm_matrix4x4f_get_m13(const psm_matrix4x4f_t* matrix4x4f)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_get_m13(): matrix4x4f is null");
        return 0.0f;
    }
    return matrix4x4f->getM13();
}

void psm_matrix4x4f_set_m13(psm_matrix4x4f_t* matrix4x4f, float m13)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_set_m13(): matrix4x4f is null");
        return;
    }
    matrix4x4f->setM13(m13);
}

float psm_matrix4x4f_get_m14(const psm_matrix4x4f_t* matrix4x4f)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_get_m14(): matrix4x4f is null");
        return 0.0f;
    }
    return matrix4x4f->getM14();
}

void psm_matrix4x4f_set_m14(psm_matrix4x4f_t* matrix4x4f, float m14)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_set_m14(): matrix4x4f is null");
        return;
    }
    matrix4x4f->setM14(m14);
}

float psm_matrix4x4f_get_m20(const psm_matrix4x4f_t* matrix4x4f)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_get_m20(): matrix4x4f is null");
        return 0.0f;
    }
    return matrix4x4f->getM20();
}

void psm_matrix4x4f_set_m20(psm_matrix4x4f_t* matrix4x4f, float m20)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_set_m20(): matrix4x4f is null");
        return;
    }
    matrix4x4f->setM20(m20);
}

float psm_matrix4x4f_get_m21(const psm_matrix4x4f_t* matrix4x4f)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_get_m21(): matrix4x4f is null");
        return 0.0f;
    }
    return matrix4x4f->getM21();
}

void psm_matrix4x4f_set_m21(psm_matrix4x4f_t* matrix4x4f, float m21)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_set_m21(): matrix4x4f is null");
        return;
    }
    matrix4x4f->setM21(m21);
}

float psm_matrix4x4f_get_m22(const psm_matrix4x4f_t* matrix4x4f)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_get_m22(): matrix4x4f is null");
        return 0.0f;
    }
    return matrix4x4f->getM22();
}

void psm_matrix4x4f_set_m22(psm_matrix4x4f_t* matrix4x4f, float m22)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_set_m22(): matrix4x4f is null");
        return;
    }
    matrix4x4f->setM22(m22);
}

float psm_matrix4x4f_get_m23(const psm_matrix4x4f_t* matrix4x4f)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_get_m23(): matrix4x4f is null");
        return 0.0f;
    }
    return matrix4x4f->getM23();
}

void psm_matrix4x4f_set_m23(psm_matrix4x4f_t* matrix4x4f, float m23)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_set_m23(): matrix4x4f is null");
        return;
    }
    matrix4x4f->setM23(m23);
}

float psm_matrix4x4f_get_m24(const psm_matrix4x4f_t* matrix4x4f)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_get_m24(): matrix4x4f is null");
        return 0.0f;
    }
    return matrix4x4f->getM24();
}

void psm_matrix4x4f_set_m24(psm_matrix4x4f_t* matrix4x4f, float m24)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_set_m24(): matrix4x4f is null");
        return;
    }
    matrix4x4f->setM24(m24);
}

float psm_matrix4x4f_get_m30(const psm_matrix4x4f_t* matrix4x4f)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_get_m30(): matrix4x4f is null");
        return 0.0f;
    }
    return matrix4x4f->getM30();
}

void psm_matrix4x4f_set_m30(psm_matrix4x4f_t* matrix4x4f, float m30)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_set_m30(): matrix4x4f is null");
        return;
    }
    matrix4x4f->setM30(m30);
}

float psm_matrix4x4f_get_m31(const psm_matrix4x4f_t* matrix4x4f)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_get_m31(): matrix4x4f is null");
        return 0.0f;
    }
    return matrix4x4f->getM31();
}

void psm_matrix4x4f_set_m31(psm_matrix4x4f_t* matrix4x4f, float m31)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_set_m31(): matrix4x4f is null");
        return;
    }
    matrix4x4f->setM31(m31);
}

float psm_matrix4x4f_get_m32(const psm_matrix4x4f_t* matrix4x4f)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_get_m32(): matrix4x4f is null");
        return 0.0f;
    }
    return matrix4x4f->getM32();
}

void psm_matrix4x4f_set_m32(psm_matrix4x4f_t* matrix4x4f, float m32)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_set_m32(): matrix4x4f is null");
        return;
    }
    matrix4x4f->setM32(m32);
}

float psm_matrix4x4f_get_m33(const psm_matrix4x4f_t* matrix4x4f)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_get_m33(): matrix4x4f is null");
        return 0.0f;
    }
    return matrix4x4f->getM33();
}

void psm_matrix4x4f_set_m33(psm_matrix4x4f_t* matrix4x4f, float m33)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_set_m33(): matrix4x4f is null");
        return;
    }
    matrix4x4f->setM33(m33);
}

float psm_matrix4x4f_get_m34(const psm_matrix4x4f_t* matrix4x4f)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_get_m34(): matrix4x4f is null");
        return 0.0f;
    }
    return matrix4x4f->getM34();
}

void psm_matrix4x4f_set_m34(psm_matrix4x4f_t* matrix4x4f, float m34)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_set_m34(): matrix4x4f is null");
        return;
    }
    matrix4x4f->setM34(m34);
}

float psm_matrix4x4f_get_m40(const psm_matrix4x4f_t* matrix4x4f)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_get_m40(): matrix4x4f is null");
        return 0.0f;
    }
    return matrix4x4f->getM40();
}

void psm_matrix4x4f_set_m40(psm_matrix4x4f_t* matrix4x4f, float m40)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_set_m40(): matrix4x4f is null");
        return;
    }
    matrix4x4f->setM40(m40);
}

float psm_matrix4x4f_get_m41(const psm_matrix4x4f_t* matrix4x4f)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_get_m41(): matrix4x4f is null");
        return 0.0f;
    }
    return matrix4x4f->getM41();
}

void psm_matrix4x4f_set_m41(psm_matrix4x4f_t* matrix4x4f, float m41)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_set_m41(): matrix4x4f is null");
        return;
    }
    matrix4x4f->setM41(m41);
}

float psm_matrix4x4f_get_m42(const psm_matrix4x4f_t* matrix4x4f)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_get_m42(): matrix4x4f is null");
        return 0.0f;
    }
    return matrix4x4f->getM42();
}

void psm_matrix4x4f_set_m42(psm_matrix4x4f_t* matrix4x4f, float m42)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_set_m42(): matrix4x4f is null");
        return;
    }
    matrix4x4f->setM42(m42);
}

float psm_matrix4x4f_get_m43(const psm_matrix4x4f_t* matrix4x4f)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_get_m43(): matrix4x4f is null");
        return 0.0f;
    }
    return matrix4x4f->getM43();
}

void psm_matrix4x4f_set_m43(psm_matrix4x4f_t* matrix4x4f, float m43)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_set_m43(): matrix4x4f is null");
        return;
    }
    matrix4x4f->setM43(m43);
}

float psm_matrix4x4f_get_m44(const psm_matrix4x4f_t* matrix4x4f)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_get_m44(): matrix4x4f is null");
        return 0.0f;
    }
    return matrix4x4f->getM44();
}

void psm_matrix4x4f_set_m44(psm_matrix4x4f_t* matrix4x4f, float m44)
{
    if (!matrix4x4f) {
        assert(!"psm_matrix4x4f_set_m44(): matrix4x4f is null");
        return;
    }
    matrix4x4f->setM44(m44);
}
