#include <Posemesh/Portals.hpp>
#include <Posemesh/QRDetection.hpp>
#include <iostream>
#include <opencv2/imgproc.hpp>
#include <opencv2/objdetect.hpp>
#include <stdexcept>
#include <utility>

namespace psm {

bool QRDetection::detectQRFromLuminance(
    const std::uint8_t* imageBytes,
    std::size_t imageBytesSize,
    int width,
    int height,
    std::vector<std::string>& outContents,
    std::vector<Vector2>& outCorners)
{
    outContents.clear();
    outCorners.clear();

    try {
        if (width <= 0) {
            throw std::invalid_argument("width");
        }
        if (height <= 0) {
            throw std::invalid_argument("height");
        }
        if (!imageBytes) {
            throw std::invalid_argument("imageBytes");
        }
        if (imageBytesSize != width * height * sizeof(std::uint8_t)) {
            throw std::invalid_argument("imageBytesSize");
        }

        // Hacky, but we aren't gonna modify cvImage so this way we skip unnecessary copying
        const cv::Mat cvImage(cv::Size(width, height), CV_8U, const_cast<std::uint8_t*>(imageBytes));

        cv::QRCodeDetector qrDetector;
        std::vector<cv::Point2f> cornersFound;
        std::vector<std::string> contentsFound;

        bool detected = qrDetector.detectAndDecodeMulti(cvImage, contentsFound, cornersFound);
        if (!detected) {
            std::vector<cv::Point> singleCorners;
            std::string singleContent = qrDetector.detectAndDecode(cvImage, singleCorners);
            if (!singleContent.empty() && singleCorners.size() == 4) {
                contentsFound.push_back(std::move(singleContent));
                cornersFound.reserve(singleCorners.size());
                for (const auto& corner : singleCorners) {
                    cornersFound.emplace_back(static_cast<float>(corner.x), static_cast<float>(corner.y));
                }
                detected = true;
            }
        }

        if (!detected) {
            return false;
        }

        if (cornersFound.size() != contentsFound.size() * 4) {
            std::cerr << "QRDetection::detectQRFromLuminance(): Invalid corner output count from decoder." << std::endl;
            return false;
        }

        outContents.reserve(contentsFound.size());
        for (const auto& s : contentsFound) {
            outContents.push_back(s);
        }

        outCorners.reserve(cornersFound.size());
        for (const auto& p : cornersFound) {
            Vector2 corner;
            corner.setX(p.x);
            corner.setY(p.y);
            outCorners.push_back(std::move(corner));
        }
    } catch (const cv::Exception& e) {
        std::cerr << "QRDetection::detectQRFromLuminance(): An OpenCV exception occurred: " << e.what() << std::endl;
        return false;
    } catch (const std::exception& e) {
        std::cerr << "QRDetection::detectQRFromLuminance(): An exception occurred: " << e.what() << std::endl;
        return false;
    }

    return true;
}

bool QRDetection::detectQRFromLuminance(
    const std::vector<std::uint8_t>& imageBytes,
    int width,
    int height,
    std::vector<std::string>& outContents,
    std::vector<Vector2>& outCorners)
{
    return detectQRFromLuminance(imageBytes.data(), imageBytes.size(), width, height, outContents, outCorners);
}

std::vector<LandmarkObservation> QRDetection::detectQRFromLuminance(
    const std::vector<std::uint8_t>& imageBytes,
    int width,
    int height)
{
    std::vector<std::string> outContents;
    std::vector<Vector2> outCorners;
    bool detected = QRDetection::detectQRFromLuminance(
        imageBytes.data(),
        imageBytes.size(),
        width,
        height,
        outContents,
        outCorners);

    std::vector<LandmarkObservation> observations;

    if (detected) {
        if (outCorners.size() < outContents.size() * 4) {
            std::cerr << "QRDetection::detectQRFromLuminance(): Observation corner count mismatch." << std::endl;
            return observations;
        }
        for (std::size_t i = 0; i < outContents.size(); i++) {
            const std::size_t cornerBaseIndex = i * 4;
            for (int j = 0; j < 4; j++) {
                LandmarkObservation l;
                l.setPosition(outCorners[cornerBaseIndex + j]);
                l.setType("qr");
                std::string content = outContents[i];
                if (Portals::isAukiQR(content)) {
                    l.setType("portal");
                    content = Portals::extractShortId(content);
                }
                l.setId(content + "_" + std::to_string(j));
                observations.push_back(l);
            }
        }
    }

    return observations;
}
}
