// ghcv — the greyhound OpenCV shim: ArUco detection behind a flat C ABI.
//
// Layering rule (mirrors the OCCT shim): ALL OpenCV knowledge lives here.
// The ABI passes encoded image bytes in and marker tag ids out; no OpenCV
// type crosses the boundary, and the shim holds no state — every call is
// self-contained, so the C# side can treat it as a pure function.
//
// Dictionary: OpenCV's own predefined DICT_4X4_1000 (the dictionary the
// physical markers and the capture corpus use).

#include <opencv2/core.hpp>
#include <opencv2/imgcodecs.hpp>
#include <opencv2/objdetect/aruco_detector.hpp>
#include <opencv2/objdetect/aruco_dictionary.hpp>

#include <cstddef>
#include <vector>

extern "C" {

// The ABI version of this shim; bump on any signature change. The Rust
// side pins the same value (src/lib.rs GHCV_ABI_VERSION).
__declspec(dllexport) int ghcv_abi_version()
{
    return 1;
}

// Detect DICT_4X4_1000 markers in one encoded image (PNG, JPEG, ...).
//
//   data/len   — the encoded image bytes (any format cv::imdecode reads).
//   ids_out    — caller buffer receiving the detected tag ids in
//                detection order; may be null when max_ids is 0.
//   max_ids    — capacity of ids_out; excess detections are dropped.
//
// Returns the number of markers found (which may exceed max_ids; only
// max_ids are written), or a negative value on failure:
//   -1  the bytes did not decode to an image (bad/empty input)
//   -2  internal error (an OpenCV exception escaped detection)
__declspec(dllexport) int ghcv_aruco_detect(
    const unsigned char* data, std::size_t len,
    int* ids_out, std::size_t max_ids)
{
    try
    {
        if (data == nullptr || len == 0)
        {
            return -1;
        }
        // Wrap without copying: imdecode owns a copy of the decoded
        // pixels, the byte view is read-only for the decode step.
        const cv::Mat encoded(
            1, static_cast<int>(len), CV_8UC1,
            const_cast<unsigned char*>(data));
        const cv::Mat image = cv::imdecode(encoded, cv::IMREAD_COLOR);
        if (image.empty())
        {
            return -1;
        }
        const auto dictionary = cv::aruco::getPredefinedDictionary(
            cv::aruco::DICT_4X4_1000);
        const cv::aruco::ArucoDetector detector(dictionary);
        std::vector<int> ids;
        std::vector<std::vector<cv::Point2f>> corners;
        detector.detectMarkers(image, corners, ids);
        const std::size_t written =
            ids.size() < max_ids ? ids.size() : max_ids;
        for (std::size_t i = 0; i < written; ++i)
        {
            ids_out[i] = ids[i];
        }
        return static_cast<int>(ids.size());
    }
    catch (...)
    {
        return -2;
    }
}

} // extern "C"
