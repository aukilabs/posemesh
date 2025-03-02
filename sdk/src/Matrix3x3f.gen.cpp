/* This code is automatically generated from Matrix3x3f.json interface. Do not modify it manually as it will be overwritten! */

#include <Posemesh/Matrix3x3f.hpp>

namespace psm {

Matrix3x3f::Matrix3x3f() noexcept
    : m_m00(0.0f)
    , m_m01(0.0f)
    , m_m02(0.0f)
    , m_m03(0.0f)
    , m_m10(0.0f)
    , m_m11(0.0f)
    , m_m12(0.0f)
    , m_m13(0.0f)
    , m_m20(0.0f)
    , m_m21(0.0f)
    , m_m22(0.0f)
    , m_m23(0.0f)
    , m_m30(0.0f)
    , m_m31(0.0f)
    , m_m32(0.0f)
    , m_m33(0.0f)
{
}

Matrix3x3f::Matrix3x3f(const Matrix3x3f& matrix3x3f) noexcept = default;

Matrix3x3f::Matrix3x3f(Matrix3x3f&& matrix3x3f) noexcept = default;

Matrix3x3f::~Matrix3x3f() = default;

Matrix3x3f& Matrix3x3f::operator=(const Matrix3x3f& matrix3x3f) noexcept = default;

Matrix3x3f& Matrix3x3f::operator=(Matrix3x3f&& matrix3x3f) noexcept = default;

bool Matrix3x3f::operator==(const Matrix3x3f& matrix3x3f) const noexcept
{
    if (!(m_m00 == matrix3x3f.m_m00)) {
        return false;
    }
    if (!(m_m01 == matrix3x3f.m_m01)) {
        return false;
    }
    if (!(m_m02 == matrix3x3f.m_m02)) {
        return false;
    }
    if (!(m_m03 == matrix3x3f.m_m03)) {
        return false;
    }
    if (!(m_m10 == matrix3x3f.m_m10)) {
        return false;
    }
    if (!(m_m11 == matrix3x3f.m_m11)) {
        return false;
    }
    if (!(m_m12 == matrix3x3f.m_m12)) {
        return false;
    }
    if (!(m_m13 == matrix3x3f.m_m13)) {
        return false;
    }
    if (!(m_m20 == matrix3x3f.m_m20)) {
        return false;
    }
    if (!(m_m21 == matrix3x3f.m_m21)) {
        return false;
    }
    if (!(m_m22 == matrix3x3f.m_m22)) {
        return false;
    }
    if (!(m_m23 == matrix3x3f.m_m23)) {
        return false;
    }
    if (!(m_m30 == matrix3x3f.m_m30)) {
        return false;
    }
    if (!(m_m31 == matrix3x3f.m_m31)) {
        return false;
    }
    if (!(m_m32 == matrix3x3f.m_m32)) {
        return false;
    }
    if (!(m_m33 == matrix3x3f.m_m33)) {
        return false;
    }
    return true;
}

bool Matrix3x3f::operator!=(const Matrix3x3f& matrix3x3f) const noexcept
{
    return !(*this == matrix3x3f);
}

float Matrix3x3f::getM00() const noexcept
{
    return m_m00;
}

void Matrix3x3f::setM00(float m00) noexcept
{
    m_m00 = m00;
}

float Matrix3x3f::getM01() const noexcept
{
    return m_m01;
}

void Matrix3x3f::setM01(float m01) noexcept
{
    m_m01 = m01;
}

float Matrix3x3f::getM02() const noexcept
{
    return m_m02;
}

void Matrix3x3f::setM02(float m02) noexcept
{
    m_m02 = m02;
}

float Matrix3x3f::getM03() const noexcept
{
    return m_m03;
}

void Matrix3x3f::setM03(float m03) noexcept
{
    m_m03 = m03;
}

float Matrix3x3f::getM10() const noexcept
{
    return m_m10;
}

void Matrix3x3f::setM10(float m10) noexcept
{
    m_m10 = m10;
}

float Matrix3x3f::getM11() const noexcept
{
    return m_m11;
}

void Matrix3x3f::setM11(float m11) noexcept
{
    m_m11 = m11;
}

float Matrix3x3f::getM12() const noexcept
{
    return m_m12;
}

void Matrix3x3f::setM12(float m12) noexcept
{
    m_m12 = m12;
}

float Matrix3x3f::getM13() const noexcept
{
    return m_m13;
}

void Matrix3x3f::setM13(float m13) noexcept
{
    m_m13 = m13;
}

float Matrix3x3f::getM20() const noexcept
{
    return m_m20;
}

void Matrix3x3f::setM20(float m20) noexcept
{
    m_m20 = m20;
}

float Matrix3x3f::getM21() const noexcept
{
    return m_m21;
}

void Matrix3x3f::setM21(float m21) noexcept
{
    m_m21 = m21;
}

float Matrix3x3f::getM22() const noexcept
{
    return m_m22;
}

void Matrix3x3f::setM22(float m22) noexcept
{
    m_m22 = m22;
}

float Matrix3x3f::getM23() const noexcept
{
    return m_m23;
}

void Matrix3x3f::setM23(float m23) noexcept
{
    m_m23 = m23;
}

float Matrix3x3f::getM30() const noexcept
{
    return m_m30;
}

void Matrix3x3f::setM30(float m30) noexcept
{
    m_m30 = m30;
}

float Matrix3x3f::getM31() const noexcept
{
    return m_m31;
}

void Matrix3x3f::setM31(float m31) noexcept
{
    m_m31 = m31;
}

float Matrix3x3f::getM32() const noexcept
{
    return m_m32;
}

void Matrix3x3f::setM32(float m32) noexcept
{
    m_m32 = m32;
}

float Matrix3x3f::getM33() const noexcept
{
    return m_m33;
}

void Matrix3x3f::setM33(float m33) noexcept
{
    m_m33 = m33;
}

}

namespace std {

std::size_t hash<psm::Matrix3x3f>::operator()(const psm::Matrix3x3f& matrix3x3f) const noexcept
{
    std::size_t result = 0;
    result ^= (hash<float> {}(matrix3x3f.m_m00)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix3x3f.m_m01)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix3x3f.m_m02)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix3x3f.m_m03)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix3x3f.m_m10)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix3x3f.m_m11)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix3x3f.m_m12)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix3x3f.m_m13)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix3x3f.m_m20)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix3x3f.m_m21)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix3x3f.m_m22)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix3x3f.m_m23)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix3x3f.m_m30)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix3x3f.m_m31)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix3x3f.m_m32)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(matrix3x3f.m_m33)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    return result;
}

}
