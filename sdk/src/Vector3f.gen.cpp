/* This code is automatically generated from Vector3f.json interface. Do not modify it manually as it will be overwritten! */

#include <Posemesh/Vector3f.hpp>

namespace psm {

Vector3f::Vector3f() noexcept
    : m_x(0.0f)
    , m_y(0.0f)
    , m_z(0.0f)
{
}

Vector3f::Vector3f(const Vector3f& vector3f) noexcept = default;

Vector3f::Vector3f(Vector3f&& vector3f) noexcept = default;

Vector3f::~Vector3f() = default;

Vector3f& Vector3f::operator=(const Vector3f& vector3f) noexcept = default;

Vector3f& Vector3f::operator=(Vector3f&& vector3f) noexcept = default;

bool Vector3f::operator==(const Vector3f& vector3f) const noexcept
{
    if (!(m_x == vector3f.m_x)) {
        return false;
    }
    if (!(m_y == vector3f.m_y)) {
        return false;
    }
    if (!(m_z == vector3f.m_z)) {
        return false;
    }
    return true;
}

bool Vector3f::operator!=(const Vector3f& vector3f) const noexcept
{
    return !(*this == vector3f);
}

float Vector3f::getX() const noexcept
{
    return m_x;
}

void Vector3f::setX(float x) noexcept
{
    m_x = x;
}

float Vector3f::getY() const noexcept
{
    return m_y;
}

void Vector3f::setY(float y) noexcept
{
    m_y = y;
}

float Vector3f::getZ() const noexcept
{
    return m_z;
}

void Vector3f::setZ(float z) noexcept
{
    m_z = z;
}

}

namespace std {

std::size_t hash<psm::Vector3f>::operator()(const psm::Vector3f& vector3f) const noexcept
{
    std::size_t result = 0;
    result ^= (hash<float> {}(vector3f.m_x)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(vector3f.m_y)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(vector3f.m_z)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    return result;
}

}
