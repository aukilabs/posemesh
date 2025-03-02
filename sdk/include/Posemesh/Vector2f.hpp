/* This code is automatically generated from Vector2f.json interface. Do not modify it manually as it will be overwritten! */

#ifndef __POSEMESH_VECTOR2F_HPP__
#define __POSEMESH_VECTOR2F_HPP__

#include <functional>

#include "API.hpp"

namespace psm {

class Vector2f {
public:
    PSM_API Vector2f() noexcept;
    PSM_API Vector2f(const Vector2f& vector2f) noexcept;
    PSM_API Vector2f(Vector2f&& vector2f) noexcept;
    PSM_API ~Vector2f();

    Vector2f& PSM_API operator=(const Vector2f& vector2f) noexcept;
    Vector2f& PSM_API operator=(Vector2f&& vector2f) noexcept;
    bool PSM_API operator==(const Vector2f& vector2f) const noexcept;
    bool PSM_API operator!=(const Vector2f& vector2f) const noexcept;

    float PSM_API getX() const noexcept;
    void PSM_API setX(float x) noexcept;
    float PSM_API getY() const noexcept;
    void PSM_API setY(float y) noexcept;

private:
    float m_x;
    float m_y;

    friend struct std::hash<Vector2f>;
};

}

namespace std {

template <>
struct hash<psm::Vector2f> {
    std::size_t PSM_API operator()(const psm::Vector2f& vector2f) const noexcept;
};

}

#endif // __POSEMESH_VECTOR2F_HPP__
