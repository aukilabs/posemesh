/* This code is automatically generated from Matrix4x4f.json interface. Do not modify it manually as it will be overwritten! */

#include <Posemesh/Matrix4x4f.hpp>
#include <emscripten/bind.h>
#include <memory>

using namespace emscripten;
using namespace psm;

namespace {
std::size_t hash(const Matrix4x4f& matrix4x4f) noexcept
{
    return std::hash<Matrix4x4f> {}(matrix4x4f);
}
}

EMSCRIPTEN_BINDINGS(Matrix4x4f)
{
    class_<Matrix4x4f>("Matrix4x4f")
        .smart_ptr<std::shared_ptr<Matrix4x4f>>("Matrix4x4f")
        .constructor(&std::make_shared<Matrix4x4f>)
        .constructor(&std::make_shared<Matrix4x4f, const Matrix4x4f&>)
        .function("duplicate()", &std::make_shared<Matrix4x4f, const Matrix4x4f&>, nonnull<ret_val>())
        .function("equals(matrix4x4f)", &Matrix4x4f::operator==)
        .function("hash()", &hash)
        .function("__getM00()", &Matrix4x4f::getM00)
        .function("__setM00(m00)", &Matrix4x4f::setM00)
        .function("__getM01()", &Matrix4x4f::getM01)
        .function("__setM01(m01)", &Matrix4x4f::setM01)
        .function("__getM02()", &Matrix4x4f::getM02)
        .function("__setM02(m02)", &Matrix4x4f::setM02)
        .function("__getM03()", &Matrix4x4f::getM03)
        .function("__setM03(m03)", &Matrix4x4f::setM03)
        .function("__getM04()", &Matrix4x4f::getM04)
        .function("__setM04(m04)", &Matrix4x4f::setM04)
        .function("__getM10()", &Matrix4x4f::getM10)
        .function("__setM10(m10)", &Matrix4x4f::setM10)
        .function("__getM11()", &Matrix4x4f::getM11)
        .function("__setM11(m11)", &Matrix4x4f::setM11)
        .function("__getM12()", &Matrix4x4f::getM12)
        .function("__setM12(m12)", &Matrix4x4f::setM12)
        .function("__getM13()", &Matrix4x4f::getM13)
        .function("__setM13(m13)", &Matrix4x4f::setM13)
        .function("__getM14()", &Matrix4x4f::getM14)
        .function("__setM14(m14)", &Matrix4x4f::setM14)
        .function("__getM20()", &Matrix4x4f::getM20)
        .function("__setM20(m20)", &Matrix4x4f::setM20)
        .function("__getM21()", &Matrix4x4f::getM21)
        .function("__setM21(m21)", &Matrix4x4f::setM21)
        .function("__getM22()", &Matrix4x4f::getM22)
        .function("__setM22(m22)", &Matrix4x4f::setM22)
        .function("__getM23()", &Matrix4x4f::getM23)
        .function("__setM23(m23)", &Matrix4x4f::setM23)
        .function("__getM24()", &Matrix4x4f::getM24)
        .function("__setM24(m24)", &Matrix4x4f::setM24)
        .function("__getM30()", &Matrix4x4f::getM30)
        .function("__setM30(m30)", &Matrix4x4f::setM30)
        .function("__getM31()", &Matrix4x4f::getM31)
        .function("__setM31(m31)", &Matrix4x4f::setM31)
        .function("__getM32()", &Matrix4x4f::getM32)
        .function("__setM32(m32)", &Matrix4x4f::setM32)
        .function("__getM33()", &Matrix4x4f::getM33)
        .function("__setM33(m33)", &Matrix4x4f::setM33)
        .function("__getM34()", &Matrix4x4f::getM34)
        .function("__setM34(m34)", &Matrix4x4f::setM34)
        .function("__getM40()", &Matrix4x4f::getM40)
        .function("__setM40(m40)", &Matrix4x4f::setM40)
        .function("__getM41()", &Matrix4x4f::getM41)
        .function("__setM41(m41)", &Matrix4x4f::setM41)
        .function("__getM42()", &Matrix4x4f::getM42)
        .function("__setM42(m42)", &Matrix4x4f::setM42)
        .function("__getM43()", &Matrix4x4f::getM43)
        .function("__setM43(m43)", &Matrix4x4f::setM43)
        .function("__getM44()", &Matrix4x4f::getM44)
        .function("__setM44(m44)", &Matrix4x4f::setM44);
}
