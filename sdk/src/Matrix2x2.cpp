#include <Posemesh/Matrix2x2.hpp>

namespace psm {

std::ostream& operator<<(std::ostream& outputStream, const Matrix2x2& matrix2x2)
{
    return outputStream << "[[" << matrix2x2.m_m00 << ", " << matrix2x2.m_m01
                        << "], [" << matrix2x2.m_m10 << ", " << matrix2x2.m_m11 << "]]";
}

}
