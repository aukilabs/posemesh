#ifndef __POSEMESH_C_CALIBRATION_HELPERS_H__
#define __POSEMESH_C_CALIBRATION_HELPERS_H__

#include "API.h"
#include "Matrix4x4.h"
#include "Pose.h"

#if defined(__cplusplus)
namespace psm {
class CalibrationHelpers;
}
typedef psm::CalibrationHelpers psm_calibration_helpers_t;
#else
typedef struct psm_calibration_helpers psm_calibration_helpers_t;
#endif

#if defined(__cplusplus)
extern "C" {
#endif

const psm_matrix4x4_t* PSM_API psm_calibration_helpers_get_calibration_matrix(psm_pose_t* pose_in_domain, psm_pose_t* observed_pose, bool only_rotate_around_y);

#if defined(__cplusplus)
}
#endif

#endif /* __POSEMESH_C_CALIBRATION_HELPERS_H__ */
