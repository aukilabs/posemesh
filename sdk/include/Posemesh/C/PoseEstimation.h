#ifndef __POSEMESH_C_POSE_ESTIMATION_H__
#define __POSEMESH_C_POSE_ESTIMATION_H__

#include <stdint.h>

#include "API.h"
#include "Matrix3x3.h"
#include "SolvePnPMethod.h"
#include "Vector2.h"
#include "Vector3.h"

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

uint8_t PSM_API psm_pose_estimation_solve_pnp(
    const psm_vector3_t* objectPoints[],
    const psm_vector2_t* imagePoints[],
    const psm_matrix3x3_t* cameraMatrix,
    psm_matrix3x3_t* outR,
    psm_vector3_t* outT,
    psm_solve_pnp_method_e method);

#if defined(__cplusplus)
}
#endif

#endif // __POSEMESH_C_POSE_ESTIMATION_H__
