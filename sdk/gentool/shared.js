// Ignore all enums and interfaces in 'CompileTests' subdirectory:
const ignoreCompileTests = false;

// Put your manually written Objective-C header names that need to be added to umbrella and bridging headers here:
const manualUmbrellaAndBridgingHeaderNames = new Set([
  'Config',
  'PoseEstimation',
  'Posemesh',
  'QRDetection',
  'ArucoDetection',
  'CalibrationHelpers'
]);

// Put your class names that will be used within an std::vector type in JavaScript here:
const requiredVectorsOfClasses = new Set([
  'Vector2',
  'Vector3',
  'Landmark',
  'LandmarkObservation'
]);

module.exports = {
  ignoreCompileTests,
  manualUmbrellaAndBridgingHeaderNames,
  requiredVectorsOfClasses
};
