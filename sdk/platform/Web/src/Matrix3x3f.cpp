/* This code is automatically generated from Matrix3x3f.json interface. Do not modify it manually as it will be overwritten! */

#include <Posemesh/Matrix3x3f.hpp>
#include <emscripten/bind.h>
#include <memory>

using namespace emscripten;
using namespace psm;

namespace {
std::size_t hash(const Matrix3x3f& matrix3x3f) noexcept
{
    return std::hash<Matrix3x3f> {}(matrix3x3f);
}
}

EMSCRIPTEN_BINDINGS(Matrix3x3f)
{
    class_<Matrix3x3f>("Matrix3x3f")
        .smart_ptr<std::shared_ptr<Matrix3x3f>>("Matrix3x3f")
        .constructor(&std::make_shared<Matrix3x3f>)
        .constructor(&std::make_shared<Matrix3x3f, const Matrix3x3f&>)
        .function("duplicate()", &std::make_shared<Matrix3x3f, const Matrix3x3f&>, nonnull<ret_val>())
        .function("equals(matrix3x3f)", &Matrix3x3f::operator==)
        .function("hash()", &hash)
        .function("__getM00()", &Matrix3x3f::getM00)
        .function("__setM00(m00)", &Matrix3x3f::setM00)
        .function("__getM01()", &Matrix3x3f::getM01)
        .function("__setM01(m01)", &Matrix3x3f::setM01)
        .function("__getM02()", &Matrix3x3f::getM02)
        .function("__setM02(m02)", &Matrix3x3f::setM02)
        .function("__getM03()", &Matrix3x3f::getM03)
        .function("__setM03(m03)", &Matrix3x3f::setM03)
        .function("__getM10()", &Matrix3x3f::getM10)
        .function("__setM10(m10)", &Matrix3x3f::setM10)
        .function("__getM11()", &Matrix3x3f::getM11)
        .function("__setM11(m11)", &Matrix3x3f::setM11)
        .function("__getM12()", &Matrix3x3f::getM12)
        .function("__setM12(m12)", &Matrix3x3f::setM12)
        .function("__getM13()", &Matrix3x3f::getM13)
        .function("__setM13(m13)", &Matrix3x3f::setM13)
        .function("__getM20()", &Matrix3x3f::getM20)
        .function("__setM20(m20)", &Matrix3x3f::setM20)
        .function("__getM21()", &Matrix3x3f::getM21)
        .function("__setM21(m21)", &Matrix3x3f::setM21)
        .function("__getM22()", &Matrix3x3f::getM22)
        .function("__setM22(m22)", &Matrix3x3f::setM22)
        .function("__getM23()", &Matrix3x3f::getM23)
        .function("__setM23(m23)", &Matrix3x3f::setM23)
        .function("__getM30()", &Matrix3x3f::getM30)
        .function("__setM30(m30)", &Matrix3x3f::setM30)
        .function("__getM31()", &Matrix3x3f::getM31)
        .function("__setM31(m31)", &Matrix3x3f::setM31)
        .function("__getM32()", &Matrix3x3f::getM32)
        .function("__setM32(m32)", &Matrix3x3f::setM32)
        .function("__getM33()", &Matrix3x3f::getM33)
        .function("__setM33(m33)", &Matrix3x3f::setM33);
}
