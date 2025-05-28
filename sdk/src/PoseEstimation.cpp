#include <Posemesh/PoseEstimation.hpp>
#include <Posemesh/PoseFactory.hpp>
#include <Posemesh/PoseTools.hpp>
#include <iostream>
#include <opencv2/calib3d.hpp>
#include <opencv2/opencv.hpp>

namespace psm {

cv::SolvePnPMethod toCVSolvePnPMethod(psm::SolvePnpMethod format)
{
    switch (format) {
    case SolvePnpMethod::SolvePnpIterative:
        return cv::SOLVEPNP_ITERATIVE;
    case SolvePnpMethod::SolvePnpEpnp:
        return cv::SOLVEPNP_EPNP;
    case SolvePnpMethod::SolvePnpP3p:
        return cv::SOLVEPNP_P3P;
    case SolvePnpMethod::SolvePnpDls:
        return cv::SOLVEPNP_DLS;
    case SolvePnpMethod::SolvePnpUpnp:
        return cv::SOLVEPNP_UPNP;
    case SolvePnpMethod::SolvePnpAp3p:
        return cv::SOLVEPNP_AP3P;
    case SolvePnpMethod::SolvePnpIppe:
        return cv::SOLVEPNP_IPPE;
    case SolvePnpMethod::SolvePnpIppeSquare:
        return cv::SOLVEPNP_IPPE_SQUARE;
    case SolvePnpMethod::SolvePnpSqpnp:
        return cv::SOLVEPNP_SQPNP;

    default:
        throw std::invalid_argument("Invalid SolvePnpMethod");
    }
}

Pose PoseEstimation::solvePnP(
    const std::vector<Landmark>& landmarks,
    const std::vector<LandmarkObservation>& landmarkObservations,
    const Matrix3x3& cameraMatrix,
    SolvePnpMethod method)
{
    std::vector<cv::Point3f> cvObjectPoints(landmarks.size());
    for (int i = 0; i < landmarks.size(); ++i) {
        auto landmarkPosition = landmarks[i].getPosition();
        cvObjectPoints[i] = cv::Point3f(landmarkPosition.getX(), landmarkPosition.getY(), landmarkPosition.getZ());
    }

    std::vector<cv::Point2f> cvImagePoints(landmarkObservations.size());
    for (int i = 0; i < landmarkObservations.size(); ++i) {
        auto landmarkObservationPosition = landmarkObservations[i].getPosition();
        cvImagePoints[i] = cv::Point2f(landmarkObservationPosition.getX(), landmarkObservationPosition.getY());
    }

    // OpenCV uses row major order, OpenGL uses column major order.
    // To address this we need to transpose the matrix (swap rows and columns).
    cv::Mat cvCameraMatrix = cv::Mat::zeros(3, 3, CV_32F);
    cvCameraMatrix.at<float>(0, 0) = cameraMatrix.getM00();
    cvCameraMatrix.at<float>(0, 1) = cameraMatrix.getM10();
    cvCameraMatrix.at<float>(0, 2) = cameraMatrix.getM20();
    cvCameraMatrix.at<float>(1, 0) = cameraMatrix.getM01();
    cvCameraMatrix.at<float>(1, 1) = cameraMatrix.getM11();
    cvCameraMatrix.at<float>(1, 2) = cameraMatrix.getM21();
    cvCameraMatrix.at<float>(2, 0) = cameraMatrix.getM02();
    cvCameraMatrix.at<float>(2, 1) = cameraMatrix.getM12();
    cvCameraMatrix.at<float>(2, 2) = cameraMatrix.getM22();

    cv::Mat distCoeffs = cv::Mat::zeros(4, 1, CV_32F);
    cv::Mat rvec = cv::Mat::zeros(3, 1, CV_32F);
    cv::Mat tvec = cv::Mat::zeros(3, 1, CV_32F);

    try {
        cv::solvePnP(cvObjectPoints,
            cvImagePoints,
            cvCameraMatrix,
            distCoeffs,
            rvec,
            tvec,
            false,
            toCVSolvePnPMethod(method));
    } catch (const cv::Exception& e) {
        std::cerr << "PoseEstimation::solvePnP(): An OpenCV exception occurred: " << e.what() << std::endl;
        return Pose();
    }

    Vector3 p;
    p.setX(tvec.at<float>(0));
    p.setY(tvec.at<float>(1));
    p.setZ(tvec.at<float>(2));

    cv::Mat rm = cv::Mat::zeros(3, 3, CV_32F);
    cv::Rodrigues(rvec, rm);
    // OpenCV uses row major order, OpenGL uses column major order.
    // To address this we need to transpose the matrix (swap rows and columns).
    Matrix3x3 r;
    r.setM00(rm.at<float>(0, 0));
    r.setM10(rm.at<float>(0, 1));
    r.setM20(rm.at<float>(0, 2));
    r.setM01(rm.at<float>(1, 0));
    r.setM11(rm.at<float>(1, 1));
    r.setM21(rm.at<float>(1, 2));
    r.setM02(rm.at<float>(2, 0));
    r.setM12(rm.at<float>(2, 1));
    r.setM22(rm.at<float>(2, 2));

    return PoseTools::fromOpenCVToOpenGL(PoseFactory::create(p, r));
}
}