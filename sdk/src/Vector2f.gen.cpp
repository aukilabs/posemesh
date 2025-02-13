/* This code is automatically generated from Vector2f.json interface. Do not modify it manually as it will be overwritten! */

#include <Posemesh/Vector2f.hpp>

namespace psm {

Vector2f::Vector2f() noexcept
    : m_x(0.0f)
    , m_y(0.0f)
    , m_length(0.0f)
{
}

Vector2f::Vector2f(const Vector2f& vector2f) noexcept = default;

Vector2f::Vector2f(Vector2f&& vector2f) noexcept = default;

Vector2f::~Vector2f() = default;

Vector2f& Vector2f::operator=(const Vector2f& vector2f) noexcept = default;

Vector2f& Vector2f::operator=(Vector2f&& vector2f) noexcept = default;

bool Vector2f::operator==(const Vector2f& vector2f) const noexcept
{
    if (!(m_x == vector2f.m_x)) {
        return false;
    }
    if (!(m_y == vector2f.m_y)) {
        return false;
    }
    if (!(m_length == vector2f.m_length)) {
        return false;
    }
    return true;
}

bool Vector2f::operator!=(const Vector2f& vector2f) const noexcept
{
    return !(*this == vector2f);
}

float Vector2f::getX() const noexcept
{
    return m_x;
}

void Vector2f::setX(float x) noexcept
{
    m_x = x;
}

float Vector2f::getY() const noexcept
{
    return m_y;
}

void Vector2f::setY(float y) noexcept
{
    m_y = y;
}

}

namespace std {

std::size_t hash<psm::Vector2f>::operator()(const psm::Vector2f& vector2f) const noexcept
{
    std::size_t result = 0;
    result ^= (hash<float> {}(vector2f.m_x)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(vector2f.m_y)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    result ^= (hash<float> {}(vector2f.m_length)) + 0x9e3779b9 + (result << 6) + (result >> 2);
    return result;
}

}
