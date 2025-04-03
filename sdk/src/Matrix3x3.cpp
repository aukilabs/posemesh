#include <Posemesh/Matrix3x3.hpp>

namespace psm {

std::ostream& operator<<(std::ostream& outputStream, const Matrix3x3& matrix3x3)
{
    return outputStream << "[[" << matrix3x3.m_m00 << ", " << matrix3x3.m_m01 << ", " << matrix3x3.m_m02
                        << "], [" << matrix3x3.m_m10 << ", " << matrix3x3.m_m11 << ", " << matrix3x3.m_m12
                        << "], [" << matrix3x3.m_m20 << ", " << matrix3x3.m_m21 << ", " << matrix3x3.m_m22 << "]]";
}

}
