#include <Posemesh/Vector4.hpp>
#include <cmath>

namespace psm {

float Vector4::getLength() const noexcept
{
    return std::sqrt(m_x * m_x + m_y * m_y + m_z * m_z + m_w * m_w);
}

std::ostream& operator<<(std::ostream& outputStream, const Vector4& vector4)
{
    return outputStream << "(" << vector4.m_x << ", " << vector4.m_y << ", " << vector4.m_z << ", " << vector4.m_w << ")";
}

}
