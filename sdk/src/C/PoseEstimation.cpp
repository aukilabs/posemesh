#include <Posemesh/C/PoseEstimation.h>
#include <Posemesh/PoseEstimation.hpp>

uint8_t psm_pose_estimation_solve_pnp(
    const psm_vector3_t* object_points[],
    const int object_points_count,
    const psm_vector2_t* image_points[],
    const int image_points_count,
    const psm_matrix3x3_t* camera_matrix,
    psm_matrix3x3_t* out_r,
    psm_vector3_t* out_t,
    psm_solve_pnp_method_e method)
{
    psm::Vector3 o_points[object_points_count];
    for (int i = 0; i < object_points_count; i++) {
        o_points[i] = *(object_points[i]);
    }

    psm::Vector2 i_points[image_points_count];
    for (int i = 0; i < image_points_count; i++) {
        i_points[i] = *(image_points[i]);
    }

    return static_cast<uint8_t>(psm::PoseEstimation::solvePnP(
        o_points,
        i_points,
        *camera_matrix,
        *out_r,
        *out_t,
        static_cast<psm::SolvePnpMethod>(method)));
}

uint8_t psm_pose_estimation_solve_pnp_landmarks(
    const psm_landmark_t* landmarks[],
    const int landmarks_count,
    const psm_landmark_observation_t* landmark_observations[],
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

    return static_cast<uint8_t>(psm::PoseEstimation::solvePnP(
        l,
        lo,
        *camera_matrix,
        *out_pose,
        static_cast<psm::SolvePnpMethod>(method)));
}
