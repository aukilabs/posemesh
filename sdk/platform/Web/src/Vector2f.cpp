/* This code is automatically generated from Vector2f.json interface. Do not modify it manually as it will be overwritten! */

#include <Posemesh/Vector2f.hpp>
#include <emscripten/bind.h>
#include <memory>

using namespace emscripten;
using namespace psm;

namespace {
std::size_t hash(const Vector2f& vector2f) noexcept
{
    return std::hash<Vector2f> {}(vector2f);
}
}

EMSCRIPTEN_BINDINGS(Vector2f)
{
    class_<Vector2f>("Vector2f")
        .smart_ptr<std::shared_ptr<Vector2f>>("Vector2f")
        .constructor(&std::make_shared<Vector2f>)
        .constructor(&std::make_shared<Vector2f, const Vector2f&>)
        .function("duplicate()", &std::make_shared<Vector2f, const Vector2f&>, nonnull<ret_val>())
        .function("equals(vector2f)", &Vector2f::operator==)
        .function("hash()", &hash)
        .function("__getX()", &Vector2f::getX)
        .function("__setX(x)", &Vector2f::setX)
        .function("__getY()", &Vector2f::getY)
        .function("__setY(y)", &Vector2f::setY);
}
