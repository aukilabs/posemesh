#include <Posemesh/Vector2.hpp>
#include <cmath>

namespace psm {

float Vector2::getLength() const noexcept
{
    return std::sqrt(m_x * m_x + m_y * m_y);
}

std::ostream& operator<<(std::ostream& outputStream, const Vector2& vector2)
{
    return outputStream << "(" << vector2.m_x << ", " << vector2.m_y << ")";
}

}
