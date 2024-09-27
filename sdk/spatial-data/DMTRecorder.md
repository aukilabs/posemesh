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
 ┃ ┣ 📜 frames_1.mp4
 ┃ ┣ 📜 ...
 ┣ 📜 Manifest.json
 ┣ 📜 Frames.csv
 ┣ 📜 ARposes.csv
 ┣ 📜 CameraIntrinsics.csv
 ┣ 📜 Observations.csv
 ┣ 📜 Accel.csv
 ┣ 📜 Gyro.csv
 ┣ 📜 gyro_accel.csv
```

- Manifest file (1 per scan)
    - Metadata about the scan. See “Manifest file” below.

- Frames folder
    - Contains the jpeg encoded images for each frame, and/or video encoded frames.
    - Images are currently named based on timestamp in seconds for human readability, but that should not be assumed by scripts. Instead, timestamps should be read from the Frames.csv file.

- Frames.csv
    - Contains the timestamp and filename for each recorded camera frame.
    
    - Format:
        - Timestamp [seconds]
        - Filename (image or video):
            - image: `filename.jpg`
            - video @ frame index: `filename.mp4@34`

    ```csv
    timestamp, filename
    ```

    - Both jpg and mp4 files are under the "Frames" folder. Frames can also be split into multiple video files if needed, to avoid too large single files for example.
    - In general, video encoding is preferred to reduce the amount of data stored, but both are supported.

- ARposes.csv
    - Contains the camera poses output for each recorded camera frame.
    
    - Format:
        - Timestamp [seconds]
        - Position (x,y,z) [meters]
        - Quaternion (x,y,z,w)

    ```csv
    timestamp, px, py, pz, qx, qy, qz, qw
    ```
    
- CameraIntrinsics.csv
    - Contains the camera intrinsics output from ARKit for each recorded camera frame
    
    - Format:
        - Timestamp [seconds]
        - Focal length (fx, fy)
        - Principal point (cx, cy)
        - Resolution (width x height) [pixels]
    
    ```csv
    timestamp, fx, fy, cx, cy, width, height
    ```
    
- Observations.csv
    - Contains each successful detected QRCode and its pose in world coordinates, and the four corners detected in image space (with sub-pixel accuracy).
    
    - Format:
        - Timestamp [seconds]
        - Portal ShortID
        - Position (x,y,z) [meters]
        - Quaternion (x,y,z,w)

    ```csv
    timestamp, shortID, px, py, pz, qx, qy, qz, qw
    ```
    
- Accel.csv
    - Contains the raw Accelerometer data. Note that iOS and Android report the data differently, but the recorder converts it to a common format. Specifically, on iOS the x/y/z values have to be negated and divided by 9.80.
    
    - Format:
        - Timestamp [seconds]
        - Acceleration [m/s²]

    ```csv
    timestamp, ax, ay, az
    ```
    
- Gyro.csv
    - Contains the Gyroscope data

    - Format:
        - Timestamp [seconds]
        - Angular Velocity (ω) [rad/s]
    
    ```csv
    timestamp, ωx, ωy, ωz
    ```
    
- gyro_accel.csv
    - Contains the linear interpreted IMU sensor data based on Gyro Timestamp
    
    - Format:
        - Timestamp [seconds]
        - Angular Velocity (ω) [rad/s]
        - Acceleration [m/s²]
    
    ```csv
    timestamp, ωx, ωy, ωz, ax, ay, az
    ```


## Manifest file

The manifest file, one per scan, is a singel file containing metadata about the scan. The values are mostly aggregated from the other files, but stored as a single small JSON file on the domain server. Keeping it small allows DMT to download the manifests of all scans in a single batch request to the domain server.

```json
{
	"portals": [ // no duplicates of same portal
		{
			"shortId": "ABCDE12345",
			"pose": { // first observed pose
                "position": {"x": px, "y": py, "z": pz},
                "rotation": {"x": qx, "y": qy, "z": qz, "w": qw}
            },
            "averagePose": { // average observed pose
                "position": {"x": px, "y": py, "z": pz},
                "rotation": {"x": qx, "y": qy, "z": qz, "w": qw}
            },
            "physicalSize": 0.1, // meters along side of QR code (from backend)
			"firstSeenTimestamp": 0.03, // seconds since scan start
			"lastSeenTimestamp": 42.0, // seconds since scan start
		} 
	],
	"coordinateSystemID": (see note below),
	"portalsBounds": {
        "center": {"x": x, "y": y, "z": z}, // meters
        "extent": {"x": x, "y": y, "z": z} // meters
    },
    "pointCloudBounds": {
        "center": {"x": x, "y": y, "z": z}, // meters
        "extent": {"x": x, "y": y, "z": z} // meters
    },
	"duration": 42.0123, // seconds,
	"frameCount": 420,
	"scanStartTime": "2024-09-23T20:57:26.4883020Z", // Time the recording started, formatted as RFC 3339
	"nickname": "dairy aisle", // allowed to mutate, "untitled recording"
	"dmtVersion": "1.0.0" // For debugging, and to enable migration logic
},
```

### Coordinate System ID

The coordinate system ID will be set such that each new scan gets the same ID as the other scan it overlapped with. The scan also transforms all its recorded data into the coordinate system of the previous scan, so that the resulting coordinate system is the same as if the two scans were performed in one continuous scan. This is primarily for human readability and debugging of the data. No downstream code depends on this transformation of the data.

**First scan of a new coordinate system:**

→ portalPoses[0].shortID (which has position 0,0,0)

**Overlapping with previous scan:**

→ previous_manifest.coordinateSystemID

where ‘previous_manifest’ is the scan which contains the portal which this scan started from.

## Planned Changes
- Record observations in image space, with support for other kinds of markers.
  - Currently the Observations.csv is specifically for portal QR codes which are recorded as 3D poses. The current ConjureKit SDK has a custom QR code scanner which outputs a very accurate 3D pose (using the known physical size).
  - To support other kinds of markers which usually output image space coordinates the format should be extended to contain image-space coordinates for any other kinds of markers too.