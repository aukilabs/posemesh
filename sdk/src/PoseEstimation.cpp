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

bool PoseEstimation::solvePnP(
    const Vector3 objectPoints[],
    const Vector2 imagePoints[],
    const Matrix3x3& cameraMatrix,
    Matrix3x3& outR,
    Vector3& outT,
    SolvePnpMethod method)
{
    std::vector<cv::Point3f> cvObjectPoints;
    cvObjectPoints.reserve(4);
    for (int i = 0; i < 4; ++i) {
        cvObjectPoints.push_back(cv::Point3f(objectPoints[i].getX(), objectPoints[i].getY(), objectPoints[i].getZ()));
    }

    std::vector<cv::Point2f> cvImagePoints;
    cvImagePoints.reserve(4);
    for (int i = 0; i < 4; ++i) {
        cvImagePoints.push_back(cv::Point2f(imagePoints[i].getX(), imagePoints[i].getY()));
    }

    cv::Mat cvCameraMatrix = cv::Mat::zeros(3, 3, CV_32F);
    cvCameraMatrix.at<float>(0) = cameraMatrix.getM00();
    cvCameraMatrix.at<float>(1) = cameraMatrix.getM01();
    cvCameraMatrix.at<float>(2) = cameraMatrix.getM02();
    cvCameraMatrix.at<float>(3) = cameraMatrix.getM10();
    cvCameraMatrix.at<float>(4) = cameraMatrix.getM11();
    cvCameraMatrix.at<float>(5) = cameraMatrix.getM12();
    cvCameraMatrix.at<float>(6) = cameraMatrix.getM20();
    cvCameraMatrix.at<float>(7) = cameraMatrix.getM21();
    cvCameraMatrix.at<float>(8) = cameraMatrix.getM22();

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
        return false;
    }

    cv::Mat R = cv::Mat::zeros(3, 3, CV_32F);
    cv::Rodrigues(rvec, R);

    outR.setM00(R.at<float>(0));
    outR.setM01(R.at<float>(1));
    outR.setM02(R.at<float>(2));
    outR.setM10(R.at<float>(3));
    outR.setM11(R.at<float>(4));
    outR.setM12(R.at<float>(5));
    outR.setM20(R.at<float>(6));
    outR.setM21(R.at<float>(7));
    outR.setM22(R.at<float>(8));

    outT.setX(tvec.at<float>(0));
    outT.setY(tvec.at<float>(1));
    outT.setZ(tvec.at<float>(2));

    return true;
}

bool PoseEstimation::solvePnP(
    const std::vector<Landmark>& landmarks,
    const std::vector<LandmarkObservation>& landmarkObservations,
    const Matrix3x3& cameraMatrix,
    Pose& outPose,
    SolvePnpMethod method)
{
    if (landmarks.size() <= 0) {
        throw std::invalid_argument("landmarks");
    }
    if (landmarkObservations.size() <= 0) {
        throw std::invalid_argument("landmarkObservations");
    }

    Vector3 objectPoints[landmarks.size()];
    for (int i = 0; i < landmarks.size(); ++i) {
        auto landmarkPosition = landmarks[i].getPosition();
        Vector3 objectPoint;
        objectPoint.setX(landmarkPosition.getX());
        objectPoint.setY(landmarkPosition.getY());
        objectPoint.setZ(landmarkPosition.getZ());
        objectPoints[i] = objectPoint;
    }

    Vector2 imagePoints[landmarkObservations.size()];
    for (int i = 0; i < landmarkObservations.size(); ++i) {
        auto landmarkObservationPosition = landmarkObservations[i].getPosition();
        Vector2 imagePoint;
        imagePoint.setX(landmarkObservationPosition.getX());
        imagePoint.setY(landmarkObservationPosition.getY());
        imagePoints[i] = imagePoint;
    }

    Matrix3x3 rotationMatrix;
    Vector3 translationVector;
    bool success = solvePnP(objectPoints, imagePoints, cameraMatrix, rotationMatrix, translationVector, method);

    Pose pose = PoseFactory::create(translationVector, rotationMatrix);
    Pose poseInOpenGL = PoseTools::fromOpenCVToOpenGL(pose);
    outPose.setPosition(poseInOpenGL.getPosition());
    outPose.setRotation(poseInOpenGL.getRotation());

    return success;
}
}
