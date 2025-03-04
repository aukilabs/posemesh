#include <Posemesh/Vector2f.hpp>
#include <cmath>

namespace psm {

float Vector2f::getLength() const noexcept
{
    return std::sqrt(m_x * m_x + m_y * m_y);
}

}
