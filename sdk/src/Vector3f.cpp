#include <Posemesh/Vector3f.hpp>
#include <cmath>

namespace psm {

float Vector3f::getLength() const noexcept
{
    return std::sqrt(m_x * m_x + m_y * m_y + m_z * m_z);
}

}
