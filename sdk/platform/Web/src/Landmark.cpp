#include <Posemesh/Landmark.hpp>
#include <emscripten/bind.h>
#include <memory>

using namespace emscripten;
using namespace psm;

namespace {
std::size_t hash(const Landmark& landmark) noexcept
{
    return std::hash<Landmark> {}(landmark);
}
}

EMSCRIPTEN_BINDINGS(Landmark)
{
    class_<Landmark>("Landmark")
        .smart_ptr<std::shared_ptr<Landmark>>("Landmark")
        .constructor(&std::make_shared<Landmark>)
        .constructor(&std::make_shared<Landmark, const Landmark&>)
        .function("duplicate()", &std::make_shared<Landmark, const Landmark&>, nonnull<ret_val>())
        .function("equals(landmark)", &Landmark::operator==)
        .function("hash()", &hash)
        .function("__getType()", &Landmark::getType)
        .function("__setType(type)", &Landmark::setType)
        .function("__getId()", &Landmark::getId)
        .function("__setId(id)", &Landmark::setId)
        .function("__getPosition()", &Landmark::getPosition)
        .function("__setPosition(position)", &Landmark::setPosition);
}
