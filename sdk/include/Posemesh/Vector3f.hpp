/* This code is automatically generated from Vector3f.json interface. Do not modify it manually as it will be overwritten! */

#ifndef __POSEMESH_VECTOR3F_HPP__
#define __POSEMESH_VECTOR3F_HPP__

#include <functional>

#include "API.hpp"

namespace psm {

class Vector3f {
public:
    PSM_API Vector3f() noexcept;
    PSM_API Vector3f(const Vector3f& vector3f) noexcept;
    PSM_API Vector3f(Vector3f&& vector3f) noexcept;
    PSM_API ~Vector3f();

    Vector3f& PSM_API operator=(const Vector3f& vector3f) noexcept;
    Vector3f& PSM_API operator=(Vector3f&& vector3f) noexcept;
    bool PSM_API operator==(const Vector3f& vector3f) const noexcept;
    bool PSM_API operator!=(const Vector3f& vector3f) const noexcept;

    float PSM_API getX() const noexcept;
    void PSM_API setX(float x) noexcept;
    float PSM_API getY() const noexcept;
    void PSM_API setY(float y) noexcept;
    float PSM_API getZ() const noexcept;
    void PSM_API setZ(float z) noexcept;

private:
    float m_x;
    float m_y;
    float m_z;

    friend struct std::hash<Vector3f>;
};

}

namespace std {

template <>
struct hash<psm::Vector3f> {
    std::size_t PSM_API operator()(const psm::Vector3f& vector3f) const noexcept;
};

}

#endif // __POSEMESH_VECTOR3F_HPP__
