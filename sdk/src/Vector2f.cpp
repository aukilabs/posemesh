#include <Posemesh/Vector2f.hpp>

namespace psm {
float Vector2f::getLength() const noexcept
{
    return sqrt(m_x * m_x + m_y * m_y);
}
}