if [ "$1" = "" ]; then
    echo "No platform specified! Exiting."
    echo ""
    echo "Usage: ./build.sh [iOS|macOS|Mac-Catalyst|iOS-Simulator|Web] [ARM64|AMD64|WASM32] [Debug|Release]"
    exit 1
fi

if [ "$2" = "" ]; then
    echo "No architecture specified! Exiting."
    echo ""
    echo "Usage: ./build.sh [iOS|macOS|Mac-Catalyst|iOS-Simulator|Web] [ARM64|AMD64|WASM32] [Debug|Release]"
    exit 1
fi

if [ "$3" = "" ]; then
    echo "No build mode specified! Exiting."
    echo ""
    echo "Usage: ./build.sh [iOS|macOS|Mac-Catalyst|iOS-Simulator|Web] [ARM64|AMD64|WASM32] [Debug|Release]"
    exit 1
fi

### OpenCV
pushd sdk/
echo "Cloning OpenCV..."
if [[ ! -e third-party ]]; then
    mkdir -p third-party
fi
pushd third-party/

git clone git@github.com:opencv/opencv.git
pushd opencv

echo "Building OpenCV..."
if [[ ! -e build ]]; then
    mkdir -p build
fi
pushd build/

cmake .. -DBUILD_SHARED_LIBS=OFF -DBUILD_OPENEXR=OFF -DWITH_OPENEXR=OFF -DCMAKE_INSTALL_PREFIX=./install
cmake --build . --parallel 8
make -j 8 install
echo "OpenCV build complete"

popd # build/
popd # opencv/
popd # third-party/
popd # sdk/

### Networking & SDK
PLATFORM="$1"
ARCHITECTURE="$2"
BUILD_MODE="$3"
./networking/scripts/Build-Library.ps1 $PLATFORM $ARCHITECTURE $BUILD_MODE
./sdk/scripts/Build-Library.ps1 $PLATFORM $ARCHITECTURE $BUILD_MODE