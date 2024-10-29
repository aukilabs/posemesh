#include <emscripten/bind.h>
#include <memory>
#include <Posemesh/Config.hpp>

using namespace emscripten;
using namespace psm;

namespace {
    std::shared_ptr<Config> duplicate(const std::shared_ptr<Config>& config) {
        return std::make_shared<Config>(*config);
    }
}

EMSCRIPTEN_BINDINGS(Config) {
    class_<Config>("Config")
        .smart_ptr<std::shared_ptr<Config>>("Config")
        .constructor(&std::make_shared<Config>)
        .constructor(&duplicate)
    ;
}
