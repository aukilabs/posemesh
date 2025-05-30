#include <Posemesh/CalibrationHelpers.hpp>
#include <Posemesh/PoseTools.hpp>
#include <glm/gtc/quaternion.hpp>
#include <glm/mat3x3.hpp>
#include <glm/vec3.hpp>
#include <utility>

namespace psm {

Matrix4x4 CalibrationHelpers::getCalibrationMatrix(const Pose& domain, const Pose& observed, bool onlyRotateAroundY) noexcept
{
    glm::mat4 domainMatrix = PoseTools::fromPoseToMatrix(domain);
    glm::mat4 observedMatrix = PoseTools::fromPoseToMatrix(observed);
    glm::mat4 calibration = domainMatrix * glm::inverse(observedMatrix);

    if (onlyRotateAroundY) {
        glm::quat calibrationRotation(calibration);
        calibrationRotation.x = 0;
        calibrationRotation.z = 0;
        glm::vec3 translation = glm::vec3(calibration[3]);
        calibration = glm::translate(glm::mat4(1.0f), translation) * glm::mat4_cast(calibrationRotation);
    }

    Matrix4x4 calibrationMatrix;
    calibrationMatrix.setM00(calibration[0][0]);
    calibrationMatrix.setM01(calibration[0][1]);
    calibrationMatrix.setM02(calibration[0][2]);
    calibrationMatrix.setM10(calibration[1][0]);
    calibrationMatrix.setM11(calibration[1][1]);
    calibrationMatrix.setM12(calibration[1][2]);
    calibrationMatrix.setM20(calibration[2][0]);
    calibrationMatrix.setM21(calibration[2][1]);
    calibrationMatrix.setM22(calibration[2][2]);
    calibrationMatrix.setM30(calibration[3][0]);
    calibrationMatrix.setM31(calibration[3][1]);
    calibrationMatrix.setM32(calibration[3][2]);
    calibrationMatrix.setM33(calibration[3][3]);

    return calibrationMatrix;
}

}
