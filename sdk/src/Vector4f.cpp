#include <Posemesh/Vector4f.hpp>
#include <cmath>

namespace psm {

float Vector4f::getLength() const noexcept
{
    return std::sqrt(m_x * m_x + m_y * m_y + m_z * m_z + m_w * m_w);
}

}
