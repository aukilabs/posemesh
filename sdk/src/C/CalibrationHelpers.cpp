#include <Posemesh/C/CalibrationHelpers.h>
#include <Posemesh/CalibrationHelpers.hpp>
#include <cassert>
#include <memory>
#include <utility>

const psm_matrix4x4_t* psm_calibration_helpers_get_calibration_matrix(psm_pose_t* domain, psm_pose_t* observed, bool only_rotate_around_y)
{
    psm::Matrix4x4 calibrationMatrix = psm::CalibrationHelpers::getCalibrationMatrix(*static_cast<const psm::Pose*>(domain), *static_cast<const psm::Pose*>(observed), only_rotate_around_y);

    psm_matrix4x4_t* result = psm_matrix4x4_create();
    psm_matrix4x4_set_m00(result, calibrationMatrix.getM00());
    psm_matrix4x4_set_m10(result, calibrationMatrix.getM10());
    psm_matrix4x4_set_m20(result, calibrationMatrix.getM20());
    psm_matrix4x4_set_m30(result, calibrationMatrix.getM30());

    psm_matrix4x4_set_m01(result, calibrationMatrix.getM01());
    psm_matrix4x4_set_m11(result, calibrationMatrix.getM11());
    psm_matrix4x4_set_m21(result, calibrationMatrix.getM21());
    psm_matrix4x4_set_m31(result, calibrationMatrix.getM31());

    psm_matrix4x4_set_m02(result, calibrationMatrix.getM02());
    psm_matrix4x4_set_m12(result, calibrationMatrix.getM12());
    psm_matrix4x4_set_m22(result, calibrationMatrix.getM22());
    psm_matrix4x4_set_m32(result, calibrationMatrix.getM32());

    psm_matrix4x4_set_m03(result, calibrationMatrix.getM03());
    psm_matrix4x4_set_m13(result, calibrationMatrix.getM13());
    psm_matrix4x4_set_m23(result, calibrationMatrix.getM23());
    psm_matrix4x4_set_m33(result, calibrationMatrix.getM33());
    return result;
}
