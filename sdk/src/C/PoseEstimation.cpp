#include <Posemesh/C/PoseEstimation.h>
#include <Posemesh/PoseEstimation.hpp>

enum psm_pose_estimation_solve_pnp_result psm_pose_estimation_solve_pnp(
    const psm_landmark_t** landmarks,
    const int landmarks_count,
    const psm_landmark_observation_t** landmark_observations,
    const int landmark_observations_count,
    const psm_matrix3x3_t* camera_matrix,
    psm_pose_t* out_pose,
    psm_solve_pnp_method_e method)
{
    std::vector<psm::Landmark> l(landmarks_count);
    for (int i = 0; i < landmarks_count; i++) {
        l[i] = *(landmarks[i]);
    }

    std::vector<psm::LandmarkObservation> lo(landmark_observations_count);
    for (int i = 0; i < landmark_observations_count; i++) {
        lo[i] = *(landmark_observations[i]);
    }

    try {
        psm::Pose pose = psm::PoseEstimation::solvePnP(l, lo, *camera_matrix, static_cast<psm::SolvePnpMethod>(method));

        psm::Vector3 p = pose.getPosition();
        psm_vector3_t* position = psm_vector3_create();
        psm_vector3_set_x(position, p.getX());
        psm_vector3_set_y(position, p.getY());
        psm_vector3_set_z(position, p.getZ());
        psm_pose_set_position(out_pose, position);

        psm::Quaternion r = pose.getRotation();
        psm_quaternion_t* rotation = psm_quaternion_create();
        psm_quaternion_set_x(rotation, r.getX());
        psm_quaternion_set_y(rotation, r.getY());
        psm_quaternion_set_z(rotation, r.getZ());
        psm_quaternion_set_w(rotation, r.getW());
        psm_pose_set_rotation(out_pose, rotation);
    } catch (const std::exception& e) {
        return PSM_POSE_ESTIMATION_SOLVE_PNP_RESULT_FAILED;
    }
    return PSM_POSE_ESTIMATION_SOLVE_PNP_RESULT_SUCCESS;
}
