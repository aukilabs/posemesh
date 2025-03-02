/* This code is automatically generated from Matrix4x4f.json interface. Do not modify it manually as it will be overwritten! */

#include <Posemesh/Matrix4x4f.hpp>

namespace psm {

Matrix4x4f::Matrix4x4f() noexcept
    : m_m00(0.0f)
    , m_m01(0.0f)
    , m_m02(0.0f)
    , m_m03(0.0f)
    , m_m04(0.0f)
    , m_m10(0.0f)
    , m_m11(0.0f)
    , m_m12(0.0f)
    , m_m13(0.0f)
    , m_m14(0.0f)
    , m_m20(0.0f)
    , m_m21(0.0f)
    , m_m22(0.0f)
    , m_m23(0.0f)
    , m_m24(0.0f)
    , m_m30(0.0f)
    , m_m31(0.0f)
    , m_m32(0.0f)
    , m_m33(0.0f)
    , m_m34(0.0f)
    , m_m40(0.0f)
    , m_m41(0.0f)
    , m_m42(0.0f)
    , m_m43(0.0f)
    , m_m44(0.0f)
{
}

Matrix4x4f::Matrix4x4f(const Matrix4x4f& matrix4x4f) noexcept = default;

Matrix4x4f::Matrix4x4f(Matrix4x4f&& matrix4x4f) noexcept = default;

Matrix4x4f::~Matrix4x4f() = default;

Matrix4x4f& Matrix4x4f::operator=(const Matrix4x4f& matrix4x4f) noexcept = default;

Matrix4x4f& Matrix4x4f::operator=(Matrix4x4f&& matrix4x4f) noexcept = default;

bool Matrix4x4f::operator==(const Matrix4x4f& matrix4x4f) const noexcept
{
    if (!(m_m00 == matrix4x4f.m_m00)) {
        return false;
    }
    if (!(m_m01 == matrix4x4f.m_m01)) {
        return false;
    }
    if (!(m_m02 == matrix4x4f.m_m02)) {
        return false;
    }
    if (!(m_m03 == matrix4x4f.m_m03)) {
        return false;
    }
    if (!(m_m04 == matrix4x4f.m_m04)) {
        return false;
    }
    if (!(m_m10 == matrix4x4f.m_m10)) {
        return false;
    }
    if (!(m_m11 == matrix4x4f.m_m11)) {
        return false;
    }
    if (!(m_m12 == matrix4x4f.m_m12)) {
        return false;
    }
    if (!(m_m13 == matrix4x4f.m_m13)) {
        return false;
    }
    if (!(m_m14 == matrix4x4f.m_m14)) {
        return false;
    }
    if (!(m_m20 == matrix4x4f.m_m20)) {
        return false;
    }
    if (!(m_m21 == matrix4x4f.m_m21)) {
        return false;
    }
    if (!(m_m22 == matrix4x4f.m_m22)) {
        return false;
    }
    if (!(m_m23 == matrix4x4f.m_m23)) {
        return false;
    }
    if (!(m_m24 == matrix4x4f.m_m24)) {
        return false;
    }
    if (!(m_m30 == matrix4x4f.m_m30)) {
        return false;
    }
    if (!(m_m31 == matrix4x4f.m_m31)) {
        return false;
    }
    if (!(m_m32 == matrix4x4f.m_m32)) {
        return false;
    }
    if (!(m_m33 == matrix4x4f.m_m33)) {
        return false;
    }
    if (!(m_m34 == matrix4x4f.m_m34)) {
        return false;
    }
    if (!(m_m40 == matrix4x4f.m_m40)) {
        return false;
    }
    if (!(m_m41 == matrix4x4f.m_m41)) {
        return false;
    }
    if (!(m_m42 == matrix4x4f.m_m42)) {
        return false;
    }
    if (!(m_m43 == matrix4x4f.m_m43)) {
        return false;
    }
    if (!(m_m44 == matrix4x4f.m_m44)) {
        return false;
    }
    return true;
}

bool Matrix4x4f::operator!=(const Matrix4x4f& matrix4x4f) const noexcept
{
    return !(*this == matrix4x4f);
}

float Matrix4x4f::getM00() const noexcept
{
    return m_m00;
}

void Matrix4x4f::setM00(float m00) noexcept
{
    m_m00 = m00;
}

float Matrix4x4f::getM01() const noexcept
{
    return m_m01;
}

void Matrix4x4f::setM01(float m01) noexcept
{
    m_m01 = m01;
}

float Matrix4x4f::getM02() const noexcept
{
    return m_m02;
}

void Matrix4x4f::setM02(float m02) noexcept
{
    m_m02 = m02;
}

float Matrix4x4f::getM03() const noexcept
{
    return m_m03;
}

void Matrix4x4f::setM03(float m03) noexcept
{
    m_m03 = m03;
}

float Matrix4x4f::getM04() const noexcept
{
    return m_m04;
}

void Matrix4x4f::setM04(float m04) noexcept
{
    m_m04 = m04;
}

float Matrix4x4f::getM10() const noexcept
{
    return m_m10;
}

void Matrix4x4f::setM10(float m10) noexcept
{
    m_m10 = m10;
}

float Matrix4x4f::getM11() const noexcept
{
    return m_m11;
}

void Matrix4x4f::setM11(float m11) noexcept
{
    m_m11 = m11;
}

float Matrix4x4f::getM12() const noexcept
{
    return m_m12;
}

void Matrix4x4f::setM12(float m12) noexcept
{
    m_m12 = m12;
}

float Matrix4x4f::getM13() const noexcept
{
    return m_m13;
}

void Matrix4x4f::setM13(float m13) noexcept
{
    m_m13 = m13;
}

float Matrix4x4f::getM14() const noexcept
{
    return m_m14;
}

void Matrix4x4f::setM14(float m14) noexcept
{
    m_m14 = m14;
}

float Matrix4x4f::getM20() const noexcept
{
    return m_m20;
}

void Matrix4x4f::setM20(float m20) noexcept
{
    m_m20 = m20;
}

float Matrix4x4f::getM21() const noexcept
{
    return m_m21;
}

void Matrix4x4f::setM21(float m21) noexcept
{
    m_m21 = m21;
}

float Matrix4x4f::getM22() const noexcept
{
    return m_m22;
}

void Matrix4x4f::setM22(float m22) noexcept
{
    m_m22 = m22;
}

float Matrix4x4f::getM23() const noexcept
{
    return m_m23;
}

void Matrix4x4f::setM23(float m23) noexcept
{
    m_m23 = m23;
}

float Matrix4x4f::getM24() const noexcept
{
    return m_m24;
}

void Matrix4x4f::setM24(float m24) noexcept
{
    m_m24 = m24;
}

float Matrix4x4f::getM30() const noexcept
{
    return m_m30;
}

void Matrix4x4f::setM30(float m30) noexcept
{
    m_m30 = m30;
}

float Matrix4x4f::getM31() const noexcept
{
    return m_m31;
}

void Matrix4x4f::setM31(float m31) noexcept
{
    m_m31 = m31;
}

float Matrix4x4f::getM32() const noexcept
{
    return m_m32;
}

void Matrix4x4f::setM32(float m32) noexcept
{
    m_m32 = m32;
}

float Matrix4x4f::getM33() const noexcept
{
    return m_m33;
}

void Matrix4x4f::setM33(float m33) noexcept
{
    m_m33 = m33;
}

float Matrix4x4f::getM34() const noexcept
{
    return m_m34;
}

void Matrix4x4f::setM34(float m34) noexcept
{
    m_m34 = m34;
}

float Matrix4x4f::getM40() const noexcept
{
    return m_m40;
}

void Matrix4x4f::setM40(float m40) noexcept
{
    m_m40 = m40;
}

float Matrix4x4f::getM41() const noexcept
{
    return m_m41;
}

void Matrix4x4f::setM41(float m41) noexcept
{
    m_m41 = m41;
}

float Matrix4x4f::getM42() const noexcept
{
    return m_m42;
}

void Matrix4x4f::setM42(float m42) noexcept
{
    m_m42 = m42;
}

float Matrix4x4f::getM43() const noexcept
{
    return m_m43;
}

void Matrix4x4f::setM43(float m43) noexcept
{
    m_m43 = m43;
}

float Matrix4x4f::getM44() const noexcept
{
    return m_m44;
}

void Matrix4x4f::setM44(float m44) noexcept
{
    m_m44 = m44;
}

}

namespace std {

std::size_t hash<psm::Matrix4x4f>::operator()(const psm::Matrix4x4f& matrix4x4f) const noexcept
{
    std::size_t result = 0;
    result ^= (hash<float> {}(matrix4x4f.m_m00)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix4x4f.m_m01)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix4x4f.m_m02)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix4x4f.m_m03)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix4x4f.m_m04)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix4x4f.m_m10)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix4x4f.m_m11)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix4x4f.m_m12)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix4x4f.m_m13)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix4x4f.m_m14)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix4x4f.m_m20)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix4x4f.m_m21)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix4x4f.m_m22)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix4x4f.m_m23)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix4x4f.m_m24)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix4x4f.m_m30)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix4x4f.m_m31)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix4x4f.m_m32)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix4x4f.m_m33)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix4x4f.m_m34)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix4x4f.m_m40)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix4x4f.m_m41)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix4x4f.m_m42)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix4x4f.m_m43)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix4x4f.m_m44)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    return result;
}

}
