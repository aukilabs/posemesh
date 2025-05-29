#include <Posemesh/CalibrationHelpers.hpp>
#include <Posemesh/PoseTools.hpp>
#include <glm/gtc/quaternion.hpp>
#include <glm/mat3x3.hpp>
#include <glm/vec3.hpp>
#include <utility>

namespace psm {

Matrix4x4 CalibrationHelpers::getCalibrationMatrix(const Pose& inWorld, const Pose& inDomain, bool onlyRotateAroundY) noexcept
{
    auto rw = inWorld.getRotation();
    auto rd = inDomain.getRotation();
    glm::quat rotationWorld(
        rw.getW(),
        rw.getX(),
        rw.getY(),
        rw.getZ());
    glm::quat rotationDomain(
        rd.getW(),
        rd.getX(),
        rd.getY(),
        rd.getZ());

    glm::quat rotation = glm::inverse(rotationDomain) * rotationWorld;

    if (onlyRotateAroundY) {
        rotation = glm::angleAxis(glm::eulerAngles(rotation).y, glm::vec3(0, 1, 0));
    }

    glm::mat3x3 rotationMatrix = glm::mat3x3(rotation);

    auto pw = inWorld.getPosition();
    auto pd = inDomain.getPosition();
    glm::vec3 position;
    position.x = -pd.getX();
    position.y = -pd.getY();
    position.z = -pd.getZ();
    position = rotation * position;
    position.x += pw.getX();
    position.y += pw.getY();
    position.z += pw.getZ();

    Matrix4x4 calibrationMatrix;
    calibrationMatrix.setM00(rotationMatrix[0][0]);
    calibrationMatrix.setM01(rotationMatrix[0][1]);
    calibrationMatrix.setM02(rotationMatrix[0][2]);
    calibrationMatrix.setM10(rotationMatrix[1][0]);
    calibrationMatrix.setM11(rotationMatrix[1][1]);
    calibrationMatrix.setM12(rotationMatrix[1][2]);
    calibrationMatrix.setM20(rotationMatrix[2][0]);
    calibrationMatrix.setM21(rotationMatrix[2][1]);
    calibrationMatrix.setM22(rotationMatrix[2][2]);
    calibrationMatrix.setM30(position.x);
    calibrationMatrix.setM31(position.y);
    calibrationMatrix.setM32(position.z);
    calibrationMatrix.setM33(1.0f);

    return calibrationMatrix;
}

Matrix4x4 CalibrationHelpers::getCalibrationMatrix2(const Pose& inWorld, const Pose& inDomain, bool onlyRotateAroundY) noexcept
{
    glm::mat4 inWorldMatrix = PoseTools::fromPoseToMatrix(inWorld);
    glm::mat4 inDomainMatrix = PoseTools::fromPoseToMatrix(inDomain);
    glm::mat4 calibration = inDomainMatrix * glm::inverse(inWorldMatrix);

    if (onlyRotateAroundY) {
        glm::mat4 yawRotationMat = glm::mat4_cast(glm::angleAxis(glm::yaw(glm::quat_cast(calibration)), glm::vec3(0, 1, 0)));
        glm::vec3 translation = glm::vec3(calibration[3]);
        calibration = glm::translate(glm::mat4(1.0f), translation) * yawRotationMat;
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
