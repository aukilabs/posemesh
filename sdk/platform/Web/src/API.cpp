#include <cstdint>
#include <emscripten/bind.h>
#include <string>
#include <Posemesh/Vector2f.hpp>
#include <Posemesh/Vector3f.hpp>

using namespace emscripten;

EMSCRIPTEN_BINDINGS(API)
{
    register_vector<std::string>("VectorString");
    register_vector<std::uint8_t>("VectorUint8");
    register_vector<psm::Vector2f>("Vector2fArray");
    register_vector<psm::Vector3f>("Vector3fArray");
}
