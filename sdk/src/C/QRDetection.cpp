#include <Posemesh/C/QRDetection.h>
#include <Posemesh/QRDetection.hpp>
#include <cassert>
#include <cstring>
#include <limits>
#include <memory>
#include <string>

bool PSM_API psm_qr_detection_detect_qr(
    const uint8_t* image_bytes,
    size_t image_bytes_size,
    int width,
    int height,
    const char* const** out_contents,
    uint32_t* out_contents_count,
    const psm_vector2_t* const** out_corners,
    uint32_t* out_corners_count)
{
    if (!out_contents) {
        assert(!"psm_qr_detection_detect_qr(): out_contents is null");
        return false;
    }
    if (!out_corners) {
        assert(!"psm_qr_detection_detect_qr(): out_corners is null");
        return false;
    }
    std::vector<std::string> contents;
    std::vector<psm::Vector2> corners;
    const bool result = psm::QRDetection::detectQRFromLuminance(image_bytes, image_bytes_size, width, height, contents, corners);
    if (!result) {
        return false;
    }

    if (contents.size() > std::numeric_limits<uint32_t>::max()) {
        assert(!"psm_qr_detection_detect_qr(): contents count overflow");
        return false;
    }
    const auto contents_count = static_cast<uint32_t>(contents.size());
    auto contents_buffer_size = (contents_count + 1) * sizeof(char*);
    const auto contents_prefix_offset = contents_buffer_size;
    for (const auto& content : contents) {
        contents_buffer_size += content.size() + 1;
    }
    std::unique_ptr<char[]> contents_buffer(new (std::nothrow) char[contents_buffer_size]);
    char** contents_prefix_ptr = reinterpret_cast<char**>(contents_buffer.get());
    char* contents_content_ptr = contents_buffer.get() + contents_prefix_offset;
    for (const auto& content : contents) {
        *contents_prefix_ptr = contents_content_ptr;
        contents_prefix_ptr++;
        std::memcpy(contents_content_ptr, content.data(), content.size() + 1);
        contents_content_ptr += content.size() + 1;
    }
    *contents_prefix_ptr = nullptr;

    if (corners.size() > std::numeric_limits<uint32_t>::max()) {
        assert(!"psm_qr_detection_detect_qr(): corners count overflow");
        return false;
    }
    const auto corners_count = static_cast<uint32_t>(corners.size());
    auto corners_buffer_size = (corners_count + 1) * sizeof(psm_vector2_t*);
    corners_buffer_size = ((corners_buffer_size + alignof(psm_vector2_t) - 1) / alignof(psm_vector2_t)) * alignof(psm_vector2_t); // Ensure alignment
    const auto corners_prefix_offset = corners_buffer_size;
    corners_buffer_size += corners.size() * sizeof(psm_vector2_t);
    std::unique_ptr<char[]> corners_buffer(new (std::nothrow) char[corners_buffer_size]);
    psm_vector2_t** corners_prefix_ptr = reinterpret_cast<psm_vector2_t**>(corners_buffer.get());
    psm_vector2_t* corners_content_ptr = reinterpret_cast<psm_vector2_t*>(corners_buffer.get() + corners_prefix_offset);
    for (auto& corner : corners) {
        *corners_prefix_ptr = corners_content_ptr;
        corners_prefix_ptr++;
        new (corners_content_ptr) psm_vector2_t(std::move(corner));
        corners_content_ptr++;
    }
    *corners_prefix_ptr = nullptr;

    *out_contents = reinterpret_cast<const char* const*>(contents_buffer.release());
    if (out_contents_count) {
        *out_contents_count = contents_count;
    }
    *out_corners = reinterpret_cast<const psm_vector2_t* const*>(corners_buffer.release());
    if (out_corners_count) {
        *out_corners_count = corners_count;
    }
    return true;
}

void PSM_API psm_qr_detection_detect_qr_free(const char* const* contents, const psm_vector2_t* const* corners)
{
    delete[] const_cast<char*>(reinterpret_cast<const char*>(contents));
    if (corners) {
        for (const auto* const* corner = corners; *corner; ++corner) {
            (*corner)->~Vector2();
        }
        delete[] const_cast<char*>(reinterpret_cast<const char*>(corners));
    }
}

bool PSM_API psm_qr_detection_detect_qr_landmark_observations(
    const uint8_t* image_bytes,
    size_t image_bytes_size,
    int width,
    int height,
    const psm_landmark_observation_t* const** out_observations,
    uint32_t* out_observations_count)
{
    if (!out_observations) {
        assert(!"psm_qr_detection_detect_qr_landmark_observations(): out_observations is null");
        return false;
    }
    std::vector<std::string> contents;
    std::vector<psm::Vector2> corners;
    std::vector<uint8_t> image_bytes_vector(image_bytes, image_bytes + image_bytes_size);
    const std::vector<psm::LandmarkObservation> observations = psm::QRDetection::detectQRFromLuminance(image_bytes_vector, width, height);
    if (observations.size() == 0) {
        return false;
    }

    if (observations.size() > std::numeric_limits<uint32_t>::max()) {
        assert(!"psm_qr_detection_detect_qr_landmark_observations(): observations count overflow");
        return false;
    }

    const auto landmark_observation_count = static_cast<uint32_t>(observations.size());
    auto landmark_observation_buffer_size = (landmark_observation_count + 1) * sizeof(psm_landmark_observation_t*);
    landmark_observation_buffer_size = ((landmark_observation_buffer_size + alignof(psm_landmark_observation_t) - 1) / alignof(psm_landmark_observation_t)) * alignof(psm_landmark_observation_t); // Ensure alignment
    const auto landmark_observation_prefix_offset = landmark_observation_buffer_size;
    landmark_observation_buffer_size += observations.size() * sizeof(psm_landmark_observation_t);
    std::unique_ptr<psm_landmark_observation_t[]> landmark_observation_buffer(new (std::nothrow) psm_landmark_observation_t[landmark_observation_buffer_size]);
    psm_landmark_observation_t** landmark_observation_prefix_ptr = reinterpret_cast<psm_landmark_observation_t**>(landmark_observation_buffer.get());
    psm_landmark_observation_t* landmark_observation_content_ptr = reinterpret_cast<psm_landmark_observation_t*>(landmark_observation_buffer.get() + landmark_observation_prefix_offset);
    for (auto& observation : observations) {
        *landmark_observation_prefix_ptr = landmark_observation_content_ptr;
        landmark_observation_prefix_ptr++;
        new (landmark_observation_content_ptr) psm_landmark_observation_t(std::move(observation));
        landmark_observation_content_ptr++;
    }
    *landmark_observation_prefix_ptr = nullptr;

    *out_observations = reinterpret_cast<const psm_landmark_observation_t* const*>(landmark_observation_buffer.release());
    if (out_observations_count) {
        *out_observations_count = landmark_observation_count;
    }
    return true;
}

void PSM_API psm_qr_detection_detect_qr_landmark_observations_free(const psm_landmark_observation_t* const* observations)
{
    if (observations) {
        for (const auto* const* observation = observations; *observation; ++observation) {
            (*observation)->~LandmarkObservation();
        }
        delete[] const_cast<psm_landmark_observation_t*>(reinterpret_cast<const psm_landmark_observation_t*>(observations));
    }
}
