/* This code is automatically generated from Vector3f.json interface. Do not modify it manually as it will be overwritten! */

#include <Posemesh/Vector3f.hpp>
#include <emscripten/bind.h>
#include <memory>

using namespace emscripten;
using namespace psm;

namespace {
std::size_t hash(const Vector3f& vector3f) noexcept
{
    return std::hash<Vector3f> {}(vector3f);
}
}

EMSCRIPTEN_BINDINGS(Vector3f)
{
    class_<Vector3f>("Vector3f")
        .smart_ptr<std::shared_ptr<Vector3f>>("Vector3f")
        .constructor(&std::make_shared<Vector3f>)
        .constructor(&std::make_shared<Vector3f, const Vector3f&>)
        .function("duplicate()", &std::make_shared<Vector3f, const Vector3f&>, nonnull<ret_val>())
        .function("equals(vector3f)", &Vector3f::operator==)
        .function("hash()", &hash)
        .function("__getX()", &Vector3f::getX)
        .function("__setX(x)", &Vector3f::setX)
        .function("__getY()", &Vector3f::getY)
        .function("__setY(y)", &Vector3f::setY)
        .function("__getZ()", &Vector3f::getZ)
        .function("__setZ(z)", &Vector3f::setZ);
}
