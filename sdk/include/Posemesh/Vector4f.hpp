/* This code is automatically generated from Vector4f.json interface. Do not modify it manually as it will be overwritten! */

#ifndef __POSEMESH_VECTOR4F_HPP__
#define __POSEMESH_VECTOR4F_HPP__

#include <functional>

#include "API.hpp"

namespace psm {

class Vector4f {
public:
    PSM_API Vector4f() noexcept;
    PSM_API Vector4f(const Vector4f& vector4f) noexcept;
    PSM_API Vector4f(Vector4f&& vector4f) noexcept;
    PSM_API ~Vector4f();

    Vector4f& PSM_API operator=(const Vector4f& vector4f) noexcept;
    Vector4f& PSM_API operator=(Vector4f&& vector4f) noexcept;
    bool PSM_API operator==(const Vector4f& vector4f) const noexcept;
    bool PSM_API operator!=(const Vector4f& vector4f) const noexcept;

    float PSM_API getX() const noexcept;
    void PSM_API setX(float x) noexcept;
    float PSM_API getY() const noexcept;
    void PSM_API setY(float y) noexcept;
    float PSM_API getZ() const noexcept;
    void PSM_API setZ(float z) noexcept;
    float PSM_API getW() const noexcept;
    void PSM_API setW(float w) noexcept;

private:
    float m_x;
    float m_y;
    float m_z;
    float m_w;

    friend struct std::hash<Vector4f>;
};
using Quaternion = Vector4f;

}

namespace std {

template <>
struct hash<psm::Vector4f> {
    std::size_t PSM_API operator()(const psm::Vector4f& vector4f) const noexcept;
};

}

#endif // __POSEMESH_VECTOR4F_HPP__
