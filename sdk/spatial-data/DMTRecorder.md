# DMT Recorder

When setting up a [posemesh domain](https://www.aukilabs.com/posemesh/domains), the `Domain Management Tool`[^1] application is used to accurately measure the pose of all portal QR codes, and storing them all into a single persistent coordinate system.

The application uses ARKit/ARCore on the phone to track the phone's movement between each scanned QR code. How accurately the portals are positioned is largely dependent on the accuracy of the phone's AR tracking, and the accuracy of the QR code scanner.

In order to further improve the accuracy, a new setup flow is currently in development, specifically targeting larger domains. ARKit and ARCore are accurate enough for many use cases already, but for large domain setups the SLAM drift becomes an issue. Domain setup does not need to be real-time, so the new flow uses a separate server to run a more computationally intense refinement process.

While scanning portals in this new mode, the application records all sensor data needed from the phone, and stores it on the domain data server. The refinement server then downloads this data and processes it with a custom structure-from-motion (SfM) algorithm. The output is a more accurate camera trajectory, portal poses, and a sparse 3D point cloud, stored back as domain data.

Initially, the recording flow will be just built into the DMT application. However, all this spatial data can be fetched through the posemesh SDK by any application.

[^1]: Available on [iOS](https://apps.apple.com/us/app/domain-management-tool/id6499270503) and [Android](https://play.google.com/store/apps/details?id=com.aukilabs.domainmanagementtool&pli=1)

# Data Recorded

This is the current format of data recorded by development version of the Domain Management Tool. The feature is in active development and the format is still open to change. Feedback on the format is welcome, preferrably in the Discussions tab here on github.

## Note about coordinate frames

Unless otherwise specified, all poses are stored according to the OpenGL coordinate convention, where +X is to the right, +Y is up, and +Z is to the back. All poses within the same scan are reported within the same reference frame where Y points in negative gravity direction (handled by ARKit/ARCore).

## Accessing the data

The data is currently stored as CSV files and images. However, when storing it into the domain server, the data encoding is handled inside of SDK helper functions. The SDK will expose all the data structs and helper functions needed to download and convert the data either as data structs in code, or converted to binary or CSV. These helper functions follow the same cross-platform approach as the rest of the SDK to ensure data consistency.

## Current Format

```bash
📦dmt_scan_yyyy_mm_dd_hh_mm_ss
 ┣ 📂 Frames
 ┃ ┣ 📜 360684.431489.jpg
 ┃ ┣ 📜 360684.531523.jpg
 ┃ ┣ 📜 ...
 ┣ 📜 Frames.csv
 ┣ 📜 ARposes.csv
 ┣ 📜 CameraIntrinsics.csv
 ┣ 📜 Observations.csv
 ┣ 📜 Accel.csv
 ┣ 📜 Gyro.csv
 ┣ 📜 gyro_accel.csv
```

- Frames folder
    - Contains the jpeg encoded images for each frame
    - Each image is named based on timestamp in seconds

- Frames.csv
    - Contains two columns of data: timestamp and the image name.
    
    ```bash
    Timestamp [s], Filename 
    ```
    
- ARposes.csv
    - Contains the camera poses output for each recorded camera frame.
    
    ```bash
    Timestamp [s], Pose x [m], Pose y [m], Pose z [m], Quaternion x, Quaternion y, Quaternion z, Quaternion w
    ```
    
- CameraIntrinsics.csv
    - Contains the camera intrinsics output from ARKit for each recorded camera frame
    
    ```bash
    Timestamp [s], FocalLength x, FocalLength y, PrincipalPoint x, PrincipalPoint y, Resolution x, Resolution y
    ```
    
- Observations.csv
    - Contains the each successful detected QRCode and its pose in world coordinates
    
    ```bash
    Timestamp [s], QRcode ID, Pose x [m], Pose y [m], Pose z [m], Quaternion x, Quaternion y, Quaternion z, Quaternion w,
    ```
    
- Accel.csv
    - Contains the raw Accelerometer data. Note that iOS and Android report the data differently, but the recorder converts it to a common format. Specifically, on iOS the x/y/z values have to be negated and divided by 9.80.
    
    ```bash
    Timestamp [s], Acceleration x [m/s^2], Acceleration y [m/s^2], Acceleration z [m/s^2]
    ```
    
- Gyro.csv
    - Contains the Gyroscope data
    
    ```bash
    Timestamp [s], Angular Velocity x [rad/s], Angular Velocity y [rad/s], Angular Velocity z [rad/s]
    ```
    
- gyro_accel.csv
    - Contains the linear interpreted IMU sensor data based on Gyro Timestamp
    
    ```bash
    Timestamp [s], Angular Velocity x [rad/s], Angular Velocity y [rad/s], Angular Velocity z [rad/s], Acceleration x [m/s^2], Acceleration y [m/s^2], Acceleration z [m/s^2]
    ```

## Planned Changes
- We want the format to support encoding RGB frames into video files in order to save storage space. The filename in that case could be either a jpg file or a specific frame index of a video file, written as filename.mp4@frame_idx
- Record observations in image space, with support for other kinds of markers.
  - Currently the Observations.csv is specifically for portal QR codes which are recorded as 3D poses. The current ConjureKit SDK has a custom QR code scanner which outputs a very accurate 3D pose (using the known physical size).
  - To support other kinds of markers which usually output image space coordinates the format should be changed to contain image-space coordinates instead of poses. For portals, the 3D pose could be still recorded into PortalObservations.csv