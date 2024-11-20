#include <emscripten/bind.h>
#include <memory>
#include <Posemesh/Config.hpp>

using namespace emscripten;
using namespace psm;

namespace {
    std::shared_ptr<Config> createDefault() {
        return std::make_shared<Config>(std::move(Config::createDefault()));
    }
}

EMSCRIPTEN_BINDINGS(Config) {
    class_<Config>("Config")
        .smart_ptr<std::shared_ptr<Config>>("Config")
        .constructor(&std::make_shared<Config>)
        .constructor(&std::make_shared<Config, const Config&>)
        .function("duplicate()", &std::make_shared<Config, const Config&>, nonnull<ret_val>())
        .function("equals(config)", &Config::operator==)
        .function("__getBootstraps()", &Config::getBootstraps)
        .function("__setBootstraps(bootstraps)", &Config::setBootstraps)
        .function("__getRelays()", &Config::getRelays)
        .function("__setRelays(relays)", &Config::setRelays)
        .class_function("createDefault", &createDefault, nonnull<ret_val>())
    ;
}
