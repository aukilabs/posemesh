#ifndef __POSEMESH_LANDMARK_HPP__
#define __POSEMESH_LANDMARK_HPP__

#include <functional>
#include <string>

#include "API.hpp"
#include "Posemesh/Vector3.hpp"

namespace psm {

class Landmark {
public:
    PSM_API Landmark();
    PSM_API Landmark(const Landmark& landmark);
    PSM_API Landmark(Landmark&& landmark);
    PSM_API ~Landmark();

    Landmark& PSM_API operator=(const Landmark& landmark);
    Landmark& PSM_API operator=(Landmark&& landmark);
    bool PSM_API operator==(const Landmark& landmark) const noexcept;
    bool PSM_API operator!=(const Landmark& landmark) const noexcept;

    std::string PSM_API getType() const;
    void PSM_API setType(std::string type) noexcept;
    std::string PSM_API getId() const;
    void PSM_API setId(std::string id) noexcept;
    Vector3 PSM_API getPosition() const;
    void PSM_API setPosition(Vector3 position) noexcept;

private:
    std::string m_type;
    std::string m_id;
    Vector3 m_position;

    friend struct std::hash<Landmark>;
};

}

namespace std {

template <>
struct hash<psm::Landmark> {
    std::size_t PSM_API operator()(const psm::Landmark& landmark) const noexcept;
};

}

#endif // __POSEMESH_LANDMARK_HPP__
