#include <Posemesh/PoseFactory.hpp>
#include <Posemesh/PoseTools.hpp>
#include <glm/gtc/quaternion.hpp>
#include <glm/matrix.hpp>

namespace psm {

Pose PoseTools::fromOpenCVToOpenGL(const Pose& pose)
{
    // Y and Z are flipped for the position.
    Vector3 p = pose.getPosition();
    p.setY(-p.getY());
    p.setZ(-p.getZ());

    // Rotate 180 degrees around X to convert the rotation.
    auto pr = pose.getRotation();
    glm::quat coordinateSystemChange = glm::angleAxis(glm::radians(180.0f), glm::vec3(1.0f, 0.0f, 0.0f));
    glm::quat rq = coordinateSystemChange * glm::quat(pr.getX(), pr.getY(), pr.getZ(), pr.getW());

    Quaternion q;
    q.setX(rq.x);
    q.setY(rq.y);
    q.setZ(rq.z);
    q.setW(rq.w);

    return PoseFactory::create(p, q);
}

Pose PoseTools::fromOpenGLToOpenCV(const Pose& pose)
{
    return fromOpenCVToOpenGL(pose);
}

}
