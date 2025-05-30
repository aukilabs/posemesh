#ifndef __POSEMESH_CALIBRATION_HELPERS_HPP__
#define __POSEMESH_CALIBRATION_HELPERS_HPP__

#include "API.hpp"
#include "Matrix4x4.hpp"
#include "Pose.hpp"

namespace psm {

class CalibrationHelpers final {
public:
    static Matrix4x4 PSM_API getCalibrationMatrix(const Pose& domain, const Pose& observed, bool onlyRotateAroundY = true) noexcept;

private:
    CalibrationHelpers() = delete;
};

}

#endif // __POSEMESH_CALIBRATION_HELPERS_HPP__
