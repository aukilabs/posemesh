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

Pose PoseTools::invertPose(const Pose& pose)
{
    Vector3 p = pose.getPosition();
    glm::vec3 pos = glm::vec3(p.getX(), p.getY(), p.getZ());

    Quaternion r = pose.getRotation();
    glm::quat rq = glm::quat(r.getW(), r.getX(), r.getY(), r.getZ());
    glm::mat3 rMat = glm::mat3_cast(rq);

    glm::mat3 rMatInv = glm::transpose(rMat);

    glm::vec3 posInv = -rMatInv * pos;

    Vector3 newP;
    newP.setX(posInv.x);
    newP.setY(posInv.y);
    newP.setZ(posInv.z);

    glm::quat rotInv = glm::quat_cast(rMatInv);
    Quaternion newRot;
    newRot.setX(rotInv.x);
    newRot.setY(rotInv.y);
    newRot.setZ(rotInv.z);
    newRot.setW(rotInv.w);

    return PoseFactory::create(newP, newRot);
}

}
