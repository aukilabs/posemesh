#include <emscripten/bind.h>
#include <memory>
#include <Posemesh/Posemesh.hpp>

using namespace emscripten;
using namespace psm;

EMSCRIPTEN_BINDINGS(Posemesh) {
    class_<Posemesh>("Posemesh")
        .smart_ptr<std::shared_ptr<Posemesh>>("Posemesh")
        .constructor(&std::make_shared<Posemesh>)
    ;
}
