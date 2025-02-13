#include <Posemesh/PoseEstimation.hpp>
#include <opencv2/calib3d.hpp>
#include <opencv2/opencv.hpp>

namespace psm {

PoseEstimation::~PoseEstimation() = default;

bool PoseEstimation::solvePnP(
    const float* objectPoints,
    const float* imagePoints,
    const float* cameraMatrix,
    Matrix3x3f* outR,
    Vector3f* outT)
{
    std::vector<cv::Point3f> cvObjectPoints;
    for (int i = 0; i < 12; i += 3) {
        cvObjectPoints.push_back(cv::Point3f(objectPoints[i + 0],
            objectPoints[i + 1],
            objectPoints[i + 2]));
    }

    std::vector<cv::Point2f> cvImagePoints;
    for (int i = 0; i < 8; i += 2)
    {
        cvImagePoints.push_back(cv::Point2f(imagePoints[i], imagePoints[i + 1]));
    }

    cv::Mat cvCameraMatrix = cv::Mat::zeros(3, 3, CV_32F);
    for (int i = 0; i < 9; i++) {
        cvCameraMatrix.at<float>(i) = cameraMatrix[i];
    }

    cv::Mat distCoeffs = cv::Mat::zeros(4, 1, CV_32F);
    cv::Mat rvec = cv::Mat::zeros(3, 1, CV_32F);
    cv::Mat tvec = cv::Mat::zeros(3, 1, CV_32F);

    try
    {
        cv::solvePnP(cvObjectPoints,
                     cvImagePoints,
                     cvCameraMatrix,
                     distCoeffs,
                     rvec,
                     tvec,
                     false,
                     cv::SOLVEPNP_IPPE_SQUARE);
    }
    catch (cv::Exception &e)
    {
        std::cout << "OpenCV exception caught: " << e.what() << std::endl;
        return false;
    }

    cv::Mat R = cv::Mat::zeros(3, 3, CV_32F);
    cv::Rodrigues(rvec, R);

    outR->setM00(R.at<float>(0));
    outR->setM01(R.at<float>(1));
    outR->setM02(R.at<float>(2));
    outR->setM10(R.at<float>(3));
    outR->setM11(R.at<float>(4));
    outR->setM12(R.at<float>(5));
    outR->setM20(R.at<float>(6));
    outR->setM21(R.at<float>(7));
    outR->setM22(R.at<float>(8));

    // for (int i = 0; i < 9; i++)
    // {
    //     outR[i] = R.at<float>(i);
    // }

    // outT[0] = tvec.at<float>(0);
    // outT[1] = tvec.at<float>(1);
    // outT[2] = tvec.at<float>(2);
    
    outT->setX(tvec.at<float>(0));
    outT->setY(tvec.at<float>(1));
    outT->setZ(tvec.at<float>(2));
    
    // outR is a Matrix3x3f, maybe converto OpenGL before returning?
    // outT is a Vector3f
    return true;
}
}
