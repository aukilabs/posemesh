#include <emscripten/bind.h>
#include <memory>
#include <Posemesh/Config.hpp>

using namespace emscripten;
using namespace psm;

namespace {
    std::shared_ptr<Config> duplicate(const std::shared_ptr<Config>& self) {
        return std::make_shared<Config>(*self);
    }

    bool equals(const std::shared_ptr<Config>& self, const std::shared_ptr<Config>& config) {
        return self->operator==(*config);
    }
}

EMSCRIPTEN_BINDINGS(Config) {
    class_<Config>("Config")
        .smart_ptr<std::shared_ptr<Config>>("Config")
        .constructor(&std::make_shared<Config>)
        .constructor(&duplicate)
        .function("duplicate()", &duplicate)
        .function("equals(config)", &equals)
        .function("__getBootstraps()", &Config::getBootstraps)
        .function("__setBootstraps(bootstraps)", &Config::setBootstraps)
    ;
}
