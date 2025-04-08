#ifndef __POSEMESH_C_QR_DETECTION_H__
#define __POSEMESH_C_QR_DETECTION_H__

#include <stdint.h>

#include "API.h"
#include "Vector2.h"

#if defined(__cplusplus)
namespace psm {
class QRDetection;
}
typedef psm::QRDetection psm_qr_detection_t;
#else
typedef struct psm_qr_detection psm_qr_detection_t;
#endif

#if defined(__cplusplus)
extern "C" {
#endif

bool PSM_API psm_qr_detection_detect_qr(
    const uint8_t* image_bytes,
    size_t image_bytes_size,
    int width,
    int height,
    const char* const** out_contents,
    uint32_t* out_contents_count,
    const psm_vector2_t* const** out_corners,
    uint32_t* out_corners_count);

void PSM_API psm_qr_detection_detect_qr_free(const char* const* contents, const psm_vector2_t* const* corners);

#if defined(__cplusplus)
}
#endif

#endif // __POSEMESH_C_QR_DETECTION_H__
