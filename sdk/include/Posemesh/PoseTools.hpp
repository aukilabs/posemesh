#ifndef __POSEMESH_POSE_TOOLS_HPP__
#define __POSEMESH_POSE_TOOLS_HPP__

#include <Posemesh/Pose.hpp>
#include <glm/mat4x4.hpp>

#include "API.hpp"

namespace psm {

class PoseTools final {
public:
    static Pose PSM_API fromOpenCVToOpenGL(const Pose& pose);
    static Pose PSM_API fromOpenGLToOpenCV(const Pose& pose);
    static Pose PSM_API invertPose(const Pose& pose);
    static glm::mat4 PSM_API fromPoseToMatrix(const Pose& pose);

private:
    PoseTools() = delete;
};

}

#endif // __POSEMESH_POSE_TOOLS_HPP__
