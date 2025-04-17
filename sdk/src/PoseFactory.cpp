#include <Posemesh/PoseFactory.hpp>
#include <glm/gtc/quaternion.hpp>
#include <glm/matrix.hpp>
#include <opencv2/opencv.hpp>

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
    glm::mat3 r(
        rotation.getM00(), rotation.getM01(), rotation.getM02(),
        rotation.getM10(), rotation.getM11(), rotation.getM12(),
        rotation.getM20(), rotation.getM21(), rotation.getM22());

    glm::quat rq = glm::quat_cast(r);

    Quaternion q;
    q.setX(rq.x);
    q.setY(rq.y);
    q.setZ(rq.z);
    q.setW(rq.w);

    return create(position, q);
}

Pose PoseFactory::createRodrigues(const Vector3& position, const Vector3& rodriguesRotation)
{
    cv::Mat rv = cv::Mat::zeros(3, 1, CV_32F);
    rv.at<float>(0) = rodriguesRotation.getX();
    rv.at<float>(1) = rodriguesRotation.getY();
    rv.at<float>(2) = rodriguesRotation.getZ();
    cv::Mat rm = cv::Mat::zeros(3, 3, CV_32F);
    cv::Rodrigues(rv, rm);

    Matrix3x3 r;
    r.setM00(rm.at<float>(0));
    r.setM01(rm.at<float>(1));
    r.setM02(rm.at<float>(2));
    r.setM10(rm.at<float>(3));
    r.setM11(rm.at<float>(4));
    r.setM12(rm.at<float>(5));
    r.setM20(rm.at<float>(6));
    r.setM21(rm.at<float>(7));
    r.setM22(rm.at<float>(8));

    return create(position, r);
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
