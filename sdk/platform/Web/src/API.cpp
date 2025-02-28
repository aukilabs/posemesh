#include <Posemesh/Vector2f.hpp>
#include <Posemesh/Vector3f.hpp>
#include <cstdint>
#include <emscripten/bind.h>
#include <string>

using namespace emscripten;

EMSCRIPTEN_BINDINGS(API)
{
    register_vector<std::int8_t>("VectorInt8");
    register_vector<std::int16_t>("VectorInt16");
    register_vector<std::int32_t>("VectorInt32");
    register_vector<std::int64_t>("VectorInt64");
    register_vector<std::uint8_t>("VectorUint8");
    register_vector<std::uint16_t>("VectorUint16");
    register_vector<std::uint32_t>("VectorUint32");
    register_vector<std::uint64_t>("VectorUint64");
    register_vector<float>("VectorFloat");
    register_vector<double>("VectorDouble");
    // VectorBoolean is an alias for VectorUint8
    register_vector<std::string>("VectorString");
    register_vector<psm::Vector2f>("Vector2fArray");
    register_vector<psm::Vector3f>("Vector3fArray");
}
