#include <Posemesh/Vector3.hpp>
#include <cmath>

namespace psm {

float Vector3::getLength() const noexcept
{
    return std::sqrt(m_x * m_x + m_y * m_y + m_z * m_z);
}

}
