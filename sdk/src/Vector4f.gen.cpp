/* This code is automatically generated from Vector4f.json interface. Do not modify it manually as it will be overwritten! */

#include <Posemesh/Vector4f.hpp>

namespace psm {

Vector4f::Vector4f() noexcept
    : m_x(0.0f)
    , m_y(0.0f)
    , m_z(0.0f)
    , m_w(0.0f)
{
}

Vector4f::Vector4f(const Vector4f& vector4f) noexcept = default;

Vector4f::Vector4f(Vector4f&& vector4f) noexcept = default;

Vector4f::~Vector4f() = default;

Vector4f& Vector4f::operator=(const Vector4f& vector4f) noexcept = default;

Vector4f& Vector4f::operator=(Vector4f&& vector4f) noexcept = default;

bool Vector4f::operator==(const Vector4f& vector4f) const noexcept
{
    if (!(m_x == vector4f.m_x)) {
        return false;
    }
    if (!(m_y == vector4f.m_y)) {
        return false;
    }
    if (!(m_z == vector4f.m_z)) {
        return false;
    }
    if (!(m_w == vector4f.m_w)) {
        return false;
    }
    return true;
}

bool Vector4f::operator!=(const Vector4f& vector4f) const noexcept
{
    return !(*this == vector4f);
}

float Vector4f::getX() const noexcept
{
    return m_x;
}

void Vector4f::setX(float x) noexcept
{
    m_x = x;
}

float Vector4f::getY() const noexcept
{
    return m_y;
}

void Vector4f::setY(float y) noexcept
{
    m_y = y;
}

float Vector4f::getZ() const noexcept
{
    return m_z;
}

void Vector4f::setZ(float z) noexcept
{
    m_z = z;
}

float Vector4f::getW() const noexcept
{
    return m_w;
}

void Vector4f::setW(float w) noexcept
{
    m_w = w;
}

}

namespace std {

std::size_t hash<psm::Vector4f>::operator()(const psm::Vector4f& vector4f) const noexcept
{
    std::size_t result = 0;
    result ^= (hash<float> {}(vector4f.m_x)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(vector4f.m_y)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(vector4f.m_z)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(vector4f.m_w)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    return result;
}

}
