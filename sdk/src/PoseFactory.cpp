#include <Posemesh/PoseFactory.hpp>
#include <glm/gtc/quaternion.hpp>
#include <glm/matrix.hpp>

namespace psm {

Pose PoseFactory::create(const Vector3& position, const Quaternion& rotation)
{
    Pose p;
    p.setPosition(position);
    p.setRotation(rotation);
    return p;
}

Pose PoseFactory::create(const Vector3& position, const Matrix3x3& rotation)
{
    glm::mat3 r;
    r[0][0] = rotation.getM00();
    r[1][0] = rotation.getM10();
    r[2][0] = rotation.getM20();
    r[0][1] = rotation.getM01();
    r[1][1] = rotation.getM11();
    r[2][1] = rotation.getM21();
    r[0][2] = rotation.getM02();
    r[1][2] = rotation.getM12();
    r[2][2] = rotation.getM22();

    glm::quat rq = glm::quat_cast(r);

    Quaternion q;
    q.setX(rq.x);
    q.setY(rq.y);
    q.setZ(rq.z);
    q.setW(rq.w);

    return create(position, q);
}

Pose PoseFactory::createEuler(const Vector3& position, const Vector3& eulerRotation)
{
    glm::quat rq(glm::radians(glm::vec3(
        eulerRotation.getX(),
        eulerRotation.getY(),
        eulerRotation.getZ())));

    Quaternion q;
    q.setX(rq.x);
    q.setY(rq.y);
    q.setZ(rq.z);
    q.setW(rq.w);

    return create(position, q);
}

}