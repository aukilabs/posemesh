#ifndef __POSEMESH_POSE_FACTORY_HPP__
#define __POSEMESH_POSE_FACTORY_HPP__

#include <Posemesh/Matrix3x3.hpp>
#include <Posemesh/Pose.hpp>
#include <Posemesh/Quaternion.hpp>
#include <Posemesh/Vector3.hpp>

#include "API.hpp"

namespace psm {

class PoseFactory final {
public:
    static Pose PSM_API create(const Vector3& position, const Quaternion& rotation);
    static Pose PSM_API create(const Vector3& position, const Matrix3x3& rotation);
    static Pose PSM_API create(const Vector3& position, const Vector3& rodriguesRotation);
    static Pose PSM_API create(const Vector3& position, const Vector3& eulerRotation);

private:
    PoseFactory() = delete;
};

}

#endif // __POSEMESH_POSE_FACTORY_HPP__
