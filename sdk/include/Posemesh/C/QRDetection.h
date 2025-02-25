#ifndef __POSEMESH_C_QR_DETECTION_H__
#define __POSEMESH_C_QR_DETECTION_H__

#include <stdint.h>

#include "API.h"
#include "Vector2f.h"
#include "Vector3f.h"

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
    psm_vector3f_t image[],
    int width,
    int height);

#if defined(__cplusplus)
}
#endif

#endif // __POSEMESH_C_QR_DETECTION_H__ 