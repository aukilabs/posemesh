#ifndef __POSEMESH_C_POSE_ESTIMATION_H__
#define __POSEMESH_C_POSE_ESTIMATION_H__

#include <stdint.h>

#include "API.h"
#include "Landmark.h"
#include "LandmarkObservation.h"
#include "Matrix3x3.h"
#include "Pose.h"
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

enum psm_pose_estimation_solve_pnp_result {
    PSM_POSE_ESTIMATION_SOLVE_PNP_RESULT_SUCCESS = 0,
    PSM_POSE_ESTIMATION_SOLVE_PNP_RESULT_FAILED = 1,
};

enum psm_pose_estimation_solve_pnp_result PSM_API psm_pose_estimation_solve_pnp(
    const psm_landmark_t** landmarks,
    const int landmarks_count,
    const psm_landmark_observation_t** landmark_observations,
    const int landmark_observations_count,
    const psm_matrix3x3_t* camera_matrix,
    psm_pose_t* out_pose,
    psm_solve_pnp_method_e method);

#if defined(__cplusplus)
}
#endif

#endif // __POSEMESH_C_POSE_ESTIMATION_H__
