#ifndef __POSEMESH_C_QR_DETECTION_H__
#define __POSEMESH_C_QR_DETECTION_H__

#include <stdint.h>

#include "API.h"
#include "Vector2.h"
#include "Vector3.h"

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
    uint8_t* image,
    int width,
    int height,
    char** contents,
    psm_vector2_t*** corners);

#if defined(__cplusplus)
}
#endif

#endif // __POSEMESH_C_QR_DETECTION_H__ 