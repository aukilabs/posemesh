#ifndef __POSEMESH_POSEMESH_HPP__
#define __POSEMESH_POSEMESH_HPP__

#include "API.hpp"
#include "Config.hpp"

namespace psm {

class Posemesh final {
public:
    PSM_API Posemesh();
    PSM_API Posemesh(const Config& config);
    Posemesh(const Posemesh& posemesh) = delete;
    PSM_API Posemesh(Posemesh&& posemesh);
    PSM_API ~Posemesh();

    Posemesh& operator=(const Posemesh& posemesh) = delete;
    Posemesh& PSM_API operator=(Posemesh&& posemesh) noexcept;
private:
    void* m_context;
};

}

#endif // __POSEMESH_POSEMESH_HPP__
