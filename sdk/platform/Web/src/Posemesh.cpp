#include <emscripten/bind.h>
#include <memory>
#include <Posemesh/Config.hpp>
#include <Posemesh/Posemesh.hpp>

using namespace emscripten;
using namespace psm;

namespace {
    bool equals(const std::shared_ptr<Posemesh>& self, const std::shared_ptr<Posemesh>& posemesh) {
        return self.get() == posemesh.get();
    }
}

EMSCRIPTEN_BINDINGS(Posemesh) {
    class_<Posemesh>("Posemesh")
        .smart_ptr<std::shared_ptr<Posemesh>>("Posemesh")
        .constructor(&std::make_shared<Posemesh>)
        .constructor(&std::make_shared<Posemesh, const Config&>)
        .function("equals(posemesh)", &equals)
    ;
}
