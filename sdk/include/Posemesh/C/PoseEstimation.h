#ifndef __POSEMESH_C_POSE_ESTIMATION_H__
#define __POSEMESH_C_POSE_ESTIMATION_H__

#include <stdint.h>

#include "API.hpp"
#include "Vector3f.h"
#include "Matrix3x3f.h"

#if defined(__cplusplus)
namespace psm {
class PoseEstimation;
}
typedef psm::PoseEstimation psm_pose_estimation_t;
#else
typedef struct psm_pose_estimation psm_pose_estimation_t;
#endif

#if defined(__cplusplus)
extern "C" {
#endif

bool PSM_API psm_pose_estimation_get_solve_pnp(
    const float* objectPoints,
    const float* imagePoints,
    const float* cameraMatrix,
    Matrix3x3f* outR,
    Vector3f* outT);

#if defined(__cplusplus)
}
#endif

#endif // __POSEMESH_C_POSE_ESTIMATION_H__
