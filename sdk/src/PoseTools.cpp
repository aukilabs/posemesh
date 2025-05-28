#include <Posemesh/PoseFactory.hpp>
#include <Posemesh/PoseTools.hpp>
#include <glm/gtc/quaternion.hpp>
#include <glm/matrix.hpp>

namespace psm {

Pose PoseTools::fromOpenCVToOpenGL(const Pose& pose)
{
    Vector3 cvPos = pose.getPosition();
    Vector3 p;
    p.setX(cvPos.getX());
    p.setY(-cvPos.getY());
    p.setZ(-cvPos.getZ());

    Quaternion cvRot = pose.getRotation();
    glm::mat3 cvRotationMatrix = glm::mat3_cast(glm::quat(cvRot.getW(), cvRot.getX(), cvRot.getY(), cvRot.getZ()));
    glm::quat flipYZ = glm::angleAxis(glm::radians(180.0f), glm::vec3(1, 0, 0));
    glm::quat glRotationQuaternion = flipYZ * glm::quat_cast(cvRotationMatrix);

    Quaternion q;
    q.setX(glRotationQuaternion.x);
    q.setY(glRotationQuaternion.y);
    q.setZ(glRotationQuaternion.z);
    q.setW(glRotationQuaternion.w);

    return PoseFactory::create(p, q);
}

Pose PoseTools::fromOpenGLToOpenCV(const Pose& pose)
{
    return fromOpenCVToOpenGL(pose);
}

}
