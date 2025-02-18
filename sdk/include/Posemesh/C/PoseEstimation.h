#ifndef __POSEMESH_C_POSE_ESTIMATION_H__
#define __POSEMESH_C_POSE_ESTIMATION_H__

#include <stdint.h>

#include "API.h"
#include "Vector2f.h"
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
    psm_vector3f_t *objectPoints[],
    psm_vector2f_t *imagePoints[],
    psm_matrix3x3f_t *cameraMatrix,
    psm_matrix3x3f_t* outR,
    psm_vector3f_t* outT);

#if defined(__cplusplus)
}
#endif

#endif // __POSEMESH_C_POSE_ESTIMATION_H__
