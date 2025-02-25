/* This code is automatically generated from Vector4f.json interface. Do not modify it manually as it will be overwritten! */

#include <Posemesh/Vector4f.hpp>
#include <emscripten/bind.h>
#include <memory>

using namespace emscripten;
using namespace psm;

namespace {
std::size_t hash(const Vector4f& vector4f) noexcept
{
    return std::hash<Vector4f> {}(vector4f);
}
}

EMSCRIPTEN_BINDINGS(Vector4f)
{
    class_<Vector4f>("Vector4f")
        .smart_ptr<std::shared_ptr<Vector4f>>("Vector4f")
        .constructor(&std::make_shared<Vector4f>)
        .constructor(&std::make_shared<Vector4f, const Vector4f&>)
        .function("duplicate()", &std::make_shared<Vector4f, const Vector4f&>, nonnull<ret_val>())
        .function("equals(vector4f)", &Vector4f::operator==)
        .function("hash()", &hash)
        .function("__getX()", &Vector4f::getX)
        .function("__setX(x)", &Vector4f::setX)
        .function("__getY()", &Vector4f::getY)
        .function("__setY(y)", &Vector4f::setY)
        .function("__getZ()", &Vector4f::getZ)
        .function("__setZ(z)", &Vector4f::setZ)
        .function("__getW()", &Vector4f::getW)
        .function("__setW(w)", &Vector4f::setW);
}
