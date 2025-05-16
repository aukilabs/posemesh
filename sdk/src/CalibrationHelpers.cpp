#include <Posemesh/CalibrationHelpers.hpp>
#include <utility>
#include <glm/mat3x3.hpp>
#include <glm/vec3.hpp>
#include <glm/gtc/quaternion.hpp>

namespace psm {

const Matrix4x4& CalibrationHelpers::getCalibrationMatrix(const Pose& poseInDomain, const Pose& observedPose, bool onlyRotateAroundY) noexcept
{
    auto rd = poseInDomain.getRotation();
    auto ro = observedPose.getRotation();
    glm::quat rotationInDomain(
        rd.getW(),
        rd.getX(),
        rd.getY(),
        rd.getZ());
    glm::quat rotationObserved(
        ro.getW(),
        ro.getX(),
        ro.getY(),
        ro.getZ());

    glm::quat rotation = glm::inverse(rotationObserved) * rotationInDomain;

    if (onlyRotateAroundY) {
        glm::vec3 rotationEuler = glm::eulerAngles(rotation);
        float rotationAroundY = rotationEuler.y;
        rotation = glm::angleAxis(rotationAroundY, glm::vec3(0, 1, 0));
    }

    glm::mat3x3 rotationMatrix = glm::mat3x3(rotation);
    
    auto pd = poseInDomain.getPosition();
    auto po = observedPose.getPosition();
    glm::vec3 position;
    position.x = -po.getX();
    position.y = -po.getY();
    position.z = -po.getZ();
    position = rotation * position;
    position.x += pd.getX();
    position.y += pd.getY();
    position.z += pd.getZ();
    
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

}
