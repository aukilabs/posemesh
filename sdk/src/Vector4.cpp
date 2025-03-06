#include <Posemesh/Vector4.hpp>
#include <cmath>

namespace psm {

float Vector4::getLength() const noexcept
{
    return std::sqrt(m_x * m_x + m_y * m_y + m_z * m_z + m_w * m_w);
}

}
