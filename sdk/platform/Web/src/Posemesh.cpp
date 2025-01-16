#include <Posemesh/Config.hpp>
#include <Posemesh/Posemesh.hpp>
#include <emscripten/bind.h>
#include <memory>

using namespace emscripten;
using namespace psm;

namespace {
bool equals(const Posemesh& self, const Posemesh& posemesh)
{
    return &self == &posemesh;
}
}

EMSCRIPTEN_BINDINGS(Posemesh)
{
    class_<Posemesh>("Posemesh")
        .smart_ptr<std::shared_ptr<Posemesh>>("Posemesh")
        .constructor(&std::make_shared<Posemesh>)
        .constructor(&std::make_shared<Posemesh, const Config&>)
        .function("equals(posemesh)", &equals)
        .property("__context", &Posemesh::__getContext)
        .class_function("getVersion()", &Posemesh::getVersion)
        .class_function("getCommitId()", &Posemesh::getCommitId);
}
