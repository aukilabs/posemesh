#include <Posemesh/Quaternion.hpp>

namespace psm {

std::ostream& operator<<(std::ostream& outputStream, const Quaternion& quaternion)
{
    const bool negX = quaternion.m_x < 0.0f;
    const bool negY = quaternion.m_y < 0.0f;
    const bool negZ = quaternion.m_z < 0.0f;
    return outputStream << quaternion.m_w
                        << (negX ? " - " : " + ") << (negX ? -quaternion.m_x : quaternion.m_x)
                        << (negY ? "i - " : "i + ") << (negY ? -quaternion.m_y : quaternion.m_y)
                        << (negZ ? "j - " : "j + ") << (negZ ? -quaternion.m_z : quaternion.m_z) << 'k';
}

}
