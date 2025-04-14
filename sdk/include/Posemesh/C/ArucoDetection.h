#ifndef __POSEMESH_C_ARUCO_DETECTION_H__
#define __POSEMESH_C_ARUCO_DETECTION_H__

#include <stdint.h>

#include "API.h"
#include "Vector2.h"
#include "ArucoMarkerFormat.h"

#if defined(__cplusplus)
namespace psm {
class ArucoDetection;
}
typedef psm::ArucoDetection psm_aruco_detection_t;
#else
typedef struct psm_aruco_detection psm_aruco_detection_t;
#endif

#if defined(__cplusplus)
extern "C" {
#endif

bool PSM_API psm_aruco_detection_detect_aruco(
    const uint8_t* image_bytes,
    size_t image_bytes_size,
    int width,
    int height,
    psm_aruco_marker_format markerFormat,
    const char* const** out_contents,
    uint32_t* out_contents_count,
    const psm_vector2_t* const** out_corners,
    uint32_t* out_corners_count);

void PSM_API psm_aruco_detection_detect_aruco_free(const char* const* contents, const psm_vector2_t* const* corners);

#if defined(__cplusplus)
}
#endif

#endif // __POSEMESH_C_ARUCO_DETECTION_H__
