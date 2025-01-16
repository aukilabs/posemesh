#include <cstdint>
#include <emscripten/bind.h>
#include <string>

using namespace emscripten;

EMSCRIPTEN_BINDINGS(API)
{
    register_vector<std::string>("VectorString");
    register_vector<std::uint8_t>("VectorUint8");
}
