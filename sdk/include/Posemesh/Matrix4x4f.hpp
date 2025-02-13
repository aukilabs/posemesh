/* This code is automatically generated from Matrix4x4f.json interface. Do not modify it manually as it will be overwritten! */

#ifndef __POSEMESH_MATRIX4X4F_HPP__
#define __POSEMESH_MATRIX4X4F_HPP__

#include <functional>

#include "API.hpp"

namespace psm {

class Matrix4x4f {
public:
    PSM_API Matrix4x4f() noexcept;
    PSM_API Matrix4x4f(const Matrix4x4f& matrix4x4f) noexcept;
    PSM_API Matrix4x4f(Matrix4x4f&& matrix4x4f) noexcept;
    PSM_API ~Matrix4x4f();

    Matrix4x4f& PSM_API operator=(const Matrix4x4f& matrix4x4f) noexcept;
    Matrix4x4f& PSM_API operator=(Matrix4x4f&& matrix4x4f) noexcept;
    bool PSM_API operator==(const Matrix4x4f& matrix4x4f) const noexcept;
    bool PSM_API operator!=(const Matrix4x4f& matrix4x4f) const noexcept;

    float PSM_API getM00() const noexcept;
    void PSM_API setM00(float m00) noexcept;
    float PSM_API getM01() const noexcept;
    void PSM_API setM01(float m01) noexcept;
    float PSM_API getM02() const noexcept;
    void PSM_API setM02(float m02) noexcept;
    float PSM_API getM03() const noexcept;
    void PSM_API setM03(float m03) noexcept;
    float PSM_API getM04() const noexcept;
    void PSM_API setM04(float m04) noexcept;
    float PSM_API getM10() const noexcept;
    void PSM_API setM10(float m10) noexcept;
    float PSM_API getM11() const noexcept;
    void PSM_API setM11(float m11) noexcept;
    float PSM_API getM12() const noexcept;
    void PSM_API setM12(float m12) noexcept;
    float PSM_API getM13() const noexcept;
    void PSM_API setM13(float m13) noexcept;
    float PSM_API getM14() const noexcept;
    void PSM_API setM14(float m14) noexcept;
    float PSM_API getM20() const noexcept;
    void PSM_API setM20(float m20) noexcept;
    float PSM_API getM21() const noexcept;
    void PSM_API setM21(float m21) noexcept;
    float PSM_API getM22() const noexcept;
    void PSM_API setM22(float m22) noexcept;
    float PSM_API getM23() const noexcept;
    void PSM_API setM23(float m23) noexcept;
    float PSM_API getM24() const noexcept;
    void PSM_API setM24(float m24) noexcept;
    float PSM_API getM30() const noexcept;
    void PSM_API setM30(float m30) noexcept;
    float PSM_API getM31() const noexcept;
    void PSM_API setM31(float m31) noexcept;
    float PSM_API getM32() const noexcept;
    void PSM_API setM32(float m32) noexcept;
    float PSM_API getM33() const noexcept;
    void PSM_API setM33(float m33) noexcept;
    float PSM_API getM34() const noexcept;
    void PSM_API setM34(float m34) noexcept;
    float PSM_API getM40() const noexcept;
    void PSM_API setM40(float m40) noexcept;
    float PSM_API getM41() const noexcept;
    void PSM_API setM41(float m41) noexcept;
    float PSM_API getM42() const noexcept;
    void PSM_API setM42(float m42) noexcept;
    float PSM_API getM43() const noexcept;
    void PSM_API setM43(float m43) noexcept;
    float PSM_API getM44() const noexcept;
    void PSM_API setM44(float m44) noexcept;

private:
    float m_m00;
    float m_m01;
    float m_m02;
    float m_m03;
    float m_m04;
    float m_m10;
    float m_m11;
    float m_m12;
    float m_m13;
    float m_m14;
    float m_m20;
    float m_m21;
    float m_m22;
    float m_m23;
    float m_m24;
    float m_m30;
    float m_m31;
    float m_m32;
    float m_m33;
    float m_m34;
    float m_m40;
    float m_m41;
    float m_m42;
    float m_m43;
    float m_m44;

    friend struct std::hash<Matrix4x4f>;
};

}

namespace std {

template <>
struct hash<psm::Matrix4x4f> {
    std::size_t PSM_API operator()(const psm::Matrix4x4f& matrix4x4f) const noexcept;
};

}

#endif // __POSEMESH_MATRIX4X4F_HPP__
