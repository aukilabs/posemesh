#include <Posemesh/Matrix4x4.hpp>

namespace psm {

std::ostream& operator<<(std::ostream& outputStream, const Matrix4x4& matrix4x4)
{
    return outputStream << "[[" << matrix4x4.m_m00 << ", " << matrix4x4.m_m01 << ", " << matrix4x4.m_m02 << ", " << matrix4x4.m_m03
                        << "], [" << matrix4x4.m_m10 << ", " << matrix4x4.m_m11 << ", " << matrix4x4.m_m12 << ", " << matrix4x4.m_m13
                        << "], [" << matrix4x4.m_m20 << ", " << matrix4x4.m_m21 << ", " << matrix4x4.m_m22 << ", " << matrix4x4.m_m23
                        << "], [" << matrix4x4.m_m30 << ", " << matrix4x4.m_m31 << ", " << matrix4x4.m_m32 << ", " << matrix4x4.m_m33 << "]]";
}

}
