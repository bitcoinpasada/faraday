// Media Foundation capture for the Windows shells (docs/PLANNING.md
// §16.33, docs/STARTER.md §8): the video capture devices, one opened as
// a source reader that gives NV12 near 1280 x 720, and synchronous reads
// whose Y and UV planes go to a C callback. Plane Y is the luma the core
// decodes; plane UV is already the interleaved chroma the core draws the
// preview in colour from.
//
// The C ABI, mirrored in src/lib.rs:
//
//     typedef void (*osk_mf_device_fn)(void *ctx,
//                                      const uint16_t *name, uint32_t name_len,
//                                      const uint16_t *link, uint32_t link_len);
//     typedef int32_t (*osk_mf_frame_fn)(void *ctx, const uint8_t *y,
//                                        uint32_t width, uint32_t height,
//                                        uint32_t stride, const uint8_t *uv,
//                                        uint32_t uv_stride);
//     int32_t osk_mf_list(void *ctx, osk_mf_device_fn on_device);
//     int32_t osk_mf_capture(void *ctx, const uint16_t *link,
//                            osk_mf_frame_fn on_frame);
//
// Both functions return 0, 1, 3 or an HRESULT, as the OSK_MF_* codes
// below say.
//
// osk_mf_list calls on_device once for each video capture device, with
// its friendly name and its symbolic link as UTF-16 (not terminated),
// valid for that call only, and returns when the list is done.
//
// osk_mf_capture opens the device whose symbolic link is `link` (a
// NUL-terminated UTF-16 string, compared without case), or the first
// device when `link` is NULL, and reads frames until on_frame returns
// non-zero or the device fails. It runs on the caller's thread, which it
// joins to COM's multithreaded apartment for its own duration, and every
// on_frame call is made on that thread before it returns. After a read
// that gave no frame, on_frame is called with `y` NULL, so the caller can
// still stop. When `y` is not NULL, `width` and `height` are even, `y`
// points at `stride * (height - 1) + width` readable bytes and `uv` is
// NULL or points at `uv_stride * (height / 2 - 1) + width` readable
// bytes, for the duration of the call.
//
// Media Foundation's DLLs are loaded here when needed, from the system
// folder only, and never imported: Windows' N editions come without them,
// and an import would stop the whole program from starting there. Without
// them there is no camera (OSK_MF_NO_CAMERA). They stay loaded for the
// life of the process: Media Foundation's threads may outlive MFShutdown
// by a moment.
//
// No state outlives a call: everything is on the stack of the call that
// made it, and every interface is released before the call returns.

#define COBJMACROS
#define WIN32_LEAN_AND_MEAN
#include <windows.h>

#include <mfapi.h>
#include <mferror.h>
#include <mfidl.h>
#include <mfreadwrite.h>

#include <stdint.h>

// Finished: the list is complete, or on_frame asked to stop.
#define OSK_MF_OK 0
// No video capture device, the chosen one is gone, or no Media
// Foundation on this Windows.
#define OSK_MF_NO_CAMERA 1
// A failure with no HRESULT behind it (a stream that ended, a mode that
// cannot be read), before the first frame or after it.
#define OSK_MF_FAILED 3
// Any failure that has an HRESULT is returned as that HRESULT, which is
// negative: the Rust side tells E_ACCESSDENIED (the camera privacy
// setting) and the "in use" codes from the rest (src/lib.rs,
// `Failure::of_status`).

// The size asked for: a dense code needs about this many pixels to read.
// A camera without the mode gives the nearest one it has.
#define WANT_WIDTH 1280
#define WANT_HEIGHT 720

typedef void (*osk_mf_device_fn)(void *ctx, const uint16_t *name,
                                 uint32_t name_len, const uint16_t *link,
                                 uint32_t link_len);
typedef int32_t (*osk_mf_frame_fn)(void *ctx, const uint8_t *y, uint32_t width,
                                   uint32_t height, uint32_t stride,
                                   const uint8_t *uv, uint32_t uv_stride);

int32_t osk_mf_list(void *ctx, osk_mf_device_fn on_device);
int32_t osk_mf_capture(void *ctx, const uint16_t *link,
                       osk_mf_frame_fn on_frame);

static const DWORD STREAM = (DWORD)MF_SOURCE_READER_FIRST_VIDEO_STREAM;

// The six Media Foundation functions this file calls, from three DLLs.
struct mf {
    HRESULT(WINAPI *startup)(ULONG version, DWORD flags);
    HRESULT(WINAPI *shutdown)(void);
    HRESULT(WINAPI *create_attributes)(IMFAttributes **out, UINT32 size);
    HRESULT(WINAPI *create_media_type)(IMFMediaType **out);
    HRESULT(WINAPI *enum_device_sources)(IMFAttributes *attributes,
                                         IMFActivate ***out, UINT32 *count);
    HRESULT(WINAPI *create_source_reader)(IMFMediaSource *source,
                                          IMFAttributes *attributes,
                                          IMFSourceReader **out);
};

// A function from a system DLL. The cast goes through `void (*)(void)`,
// which C allows between any two function pointer types.
#define PROC(module, name) ((void (*)(void))GetProcAddress((module), (name)))

// Loads the three DLLs from the system folder, finds the six functions
// and starts Media Foundation. 0 when any of that fails; on 1 the caller
// owes one mf_close.
static int mf_open(struct mf *m) {
    HMODULE plat = LoadLibraryExW(L"mfplat.dll", NULL, LOAD_LIBRARY_SEARCH_SYSTEM32);
    HMODULE mf = LoadLibraryExW(L"mf.dll", NULL, LOAD_LIBRARY_SEARCH_SYSTEM32);
    HMODULE rw = LoadLibraryExW(L"mfreadwrite.dll", NULL, LOAD_LIBRARY_SEARCH_SYSTEM32);
    if (plat == NULL || mf == NULL || rw == NULL) {
        return 0;
    }
    m->startup = (HRESULT(WINAPI *)(ULONG, DWORD))PROC(plat, "MFStartup");
    m->shutdown = (HRESULT(WINAPI *)(void))PROC(plat, "MFShutdown");
    m->create_attributes =
        (HRESULT(WINAPI *)(IMFAttributes **, UINT32))PROC(plat, "MFCreateAttributes");
    m->create_media_type =
        (HRESULT(WINAPI *)(IMFMediaType **))PROC(plat, "MFCreateMediaType");
    m->enum_device_sources =
        (HRESULT(WINAPI *)(IMFAttributes *, IMFActivate ***, UINT32 *))PROC(
            mf, "MFEnumDeviceSources");
    m->create_source_reader =
        (HRESULT(WINAPI *)(IMFMediaSource *, IMFAttributes *, IMFSourceReader **))PROC(
            rw, "MFCreateSourceReaderFromMediaSource");
    if (m->startup == NULL || m->shutdown == NULL || m->create_attributes == NULL ||
        m->create_media_type == NULL || m->enum_device_sources == NULL ||
        m->create_source_reader == NULL) {
        return 0;
    }
    // MFSTARTUP_LITE: no network sources.
    return SUCCEEDED(m->startup(MF_VERSION, MFSTARTUP_LITE)) ? 1 : 0;
}

static void mf_close(const struct mf *m) { m->shutdown(); }

// Joins this thread to COM's multithreaded apartment. 1: joined, and the
// caller owes one CoUninitialize. 0: the thread is already in a
// single-threaded apartment (a window's thread), which Media Foundation
// works in as it is. -1: COM failed.
static int com_enter(void) {
    HRESULT hr = CoInitializeEx(NULL, COINIT_MULTITHREADED);
    if (SUCCEEDED(hr)) {
        return 1;
    }
    return hr == RPC_E_CHANGED_MODE ? 0 : -1;
}

static int32_t failure(HRESULT hr) {
    return (int32_t)hr;
}

// Every video capture device. On success the caller owes free_devices.
static HRESULT enum_devices(const struct mf *m, IMFActivate ***devices,
                            UINT32 *count) {
    IMFAttributes *attributes = NULL;
    *devices = NULL;
    *count = 0;
    HRESULT hr = m->create_attributes(&attributes, 1);
    if (SUCCEEDED(hr)) {
        hr = IMFAttributes_SetGUID(attributes, &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE,
                                   &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_GUID);
    }
    if (SUCCEEDED(hr)) {
        hr = m->enum_device_sources(attributes, devices, count);
    }
    if (attributes != NULL) {
        IMFAttributes_Release(attributes);
    }
    return hr;
}

static void free_devices(IMFActivate **devices, UINT32 count) {
    for (UINT32 i = 0; i < count; i++) {
        IMFActivate_Release(devices[i]);
    }
    CoTaskMemFree(devices);
}

int32_t osk_mf_list(void *ctx, osk_mf_device_fn on_device) {
    int com = com_enter();
    if (com < 0) {
        return OSK_MF_FAILED;
    }
    int32_t status = OSK_MF_NO_CAMERA;
    struct mf m;
    if (mf_open(&m)) {
        IMFActivate **devices = NULL;
        UINT32 count = 0;
        HRESULT hr = enum_devices(&m, &devices, &count);
        if (FAILED(hr)) {
            status = failure(hr);
        } else {
            for (UINT32 i = 0; i < count; i++) {
                WCHAR *name = NULL;
                WCHAR *link = NULL;
                UINT32 name_len = 0;
                UINT32 link_len = 0;
                // A device without either is left out of the list.
                if (SUCCEEDED(IMFActivate_GetAllocatedString(
                        devices[i], &MF_DEVSOURCE_ATTRIBUTE_FRIENDLY_NAME, &name,
                        &name_len)) &&
                    SUCCEEDED(IMFActivate_GetAllocatedString(
                        devices[i], &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_SYMBOLIC_LINK,
                        &link, &link_len))) {
                    on_device(ctx, (const uint16_t *)name, name_len,
                              (const uint16_t *)link, link_len);
                }
                CoTaskMemFree(name);
                CoTaskMemFree(link);
            }
            free_devices(devices, count);
            status = OSK_MF_OK;
        }
        mf_close(&m);
    }
    if (com > 0) {
        CoUninitialize();
    }
    return status;
}

// The device to open: the first, or the one whose symbolic link is
// `link`. NULL when there is none. The reference is borrowed from
// `devices`.
static IMFActivate *pick(IMFActivate **devices, UINT32 count, const WCHAR *link) {
    if (count == 0) {
        return NULL;
    }
    if (link == NULL) {
        return devices[0];
    }
    for (UINT32 i = 0; i < count; i++) {
        WCHAR *own = NULL;
        UINT32 len = 0;
        if (SUCCEEDED(IMFActivate_GetAllocatedString(
                devices[i], &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_SYMBOLIC_LINK, &own,
                &len))) {
            int same = lstrcmpiW(own, link) == 0;
            CoTaskMemFree(own);
            if (same) {
                return devices[i];
            }
        }
    }
    return NULL;
}

static UINT32 distance(UINT32 a, UINT32 b) { return a > b ? a - b : b - a; }

// Picks the device's own format nearest 1280 x 720, so the camera itself
// runs at that size, then asks the reader for NV12 at that size; the
// reader's video processing converts whatever the device gives (YUY2,
// MJPG) into NV12. A device that refuses its own native type still gets
// the NV12 request, at 1280 x 720.
static HRESULT choose_format(const struct mf *m, IMFSourceReader *reader) {
    IMFMediaType *best = NULL;
    UINT64 best_size = ((UINT64)WANT_WIDTH << 32) | WANT_HEIGHT;
    UINT32 best_distance = UINT32_MAX;
    for (DWORD i = 0;; i++) {
        IMFMediaType *type = NULL;
        // MF_E_NO_MORE_TYPES ends the list; any other failure ends it too.
        if (FAILED(IMFSourceReader_GetNativeMediaType(reader, STREAM, i, &type))) {
            break;
        }
        UINT64 size = 0;
        UINT32 d = UINT32_MAX;
        if (SUCCEEDED(IMFMediaType_GetUINT64(type, &MF_MT_FRAME_SIZE, &size))) {
            d = distance((UINT32)(size >> 32), WANT_WIDTH) +
                distance((UINT32)size, WANT_HEIGHT);
        }
        if (d < best_distance) {
            if (best != NULL) {
                IMFMediaType_Release(best);
            }
            best = type;
            best_size = size;
            best_distance = d;
        } else {
            IMFMediaType_Release(type);
        }
    }
    if (best != NULL) {
        // A refusal is not fatal: the NV12 request below still stands.
        (void)IMFSourceReader_SetCurrentMediaType(reader, STREAM, NULL, best);
        IMFMediaType_Release(best);
    }
    IMFMediaType *nv12 = NULL;
    HRESULT hr = m->create_media_type(&nv12);
    if (SUCCEEDED(hr)) {
        hr = IMFMediaType_SetGUID(nv12, &MF_MT_MAJOR_TYPE, &MFMediaType_Video);
    }
    if (SUCCEEDED(hr)) {
        hr = IMFMediaType_SetGUID(nv12, &MF_MT_SUBTYPE, &MFVideoFormat_NV12);
    }
    if (SUCCEEDED(hr)) {
        hr = IMFMediaType_SetUINT64(nv12, &MF_MT_FRAME_SIZE, best_size);
    }
    if (SUCCEEDED(hr)) {
        hr = IMFSourceReader_SetCurrentMediaType(reader, STREAM, NULL, nv12);
    }
    if (nv12 != NULL) {
        IMFMediaType_Release(nv12);
    }
    return hr;
}

// The frames' shape, read from the reader's current type.
struct shape {
    UINT32 width;
    UINT32 height;
    UINT32 stride;
};

// Reads the current output type's size and its row stride. NV12 has no
// padding unless the type says so (MF_MT_DEFAULT_STRIDE), and is never
// bottom-up, so a negative stride is refused.
static HRESULT read_shape(IMFSourceReader *reader, struct shape *s) {
    IMFMediaType *type = NULL;
    HRESULT hr = IMFSourceReader_GetCurrentMediaType(reader, STREAM, &type);
    UINT64 size = 0;
    if (SUCCEEDED(hr)) {
        hr = IMFMediaType_GetUINT64(type, &MF_MT_FRAME_SIZE, &size);
    }
    if (SUCCEEDED(hr)) {
        s->width = (UINT32)(size >> 32);
        s->height = (UINT32)size;
        UINT32 stride = 0;
        if (FAILED(IMFMediaType_GetUINT32(type, &MF_MT_DEFAULT_STRIDE, &stride))) {
            stride = s->width;
        }
        if ((INT32)stride < (INT32)s->width) {
            hr = MF_E_INVALIDMEDIATYPE;
        }
        s->stride = stride;
    }
    if (type != NULL) {
        IMFMediaType_Release(type);
    }
    return hr;
}

// Hands one sample to on_frame, or on_frame(NULL) when it holds no usable
// frame. Returns what on_frame returned.
//
// The sample is made one contiguous buffer and locked; in that form NV12
// is plane Y, `rows` rows of `stride` bytes (`rows` at least the height:
// some decoders pad it to a multiple of 16), then plane UV, half as many
// rows of the same stride. `rows` is worked out from the length.
static int32_t hand_over(IMFSample *sample, const struct shape *s, void *ctx,
                         osk_mf_frame_fn on_frame) {
    int32_t stop;
    IMFMediaBuffer *buffer = NULL;
    BYTE *data = NULL;
    DWORD len = 0;
    if (FAILED(IMFSample_ConvertToContiguousBuffer(sample, &buffer))) {
        return on_frame(ctx, NULL, 0, 0, 0, NULL, 0);
    }
    if (FAILED(IMFMediaBuffer_Lock(buffer, &data, NULL, &len))) {
        IMFMediaBuffer_Release(buffer);
        return on_frame(ctx, NULL, 0, 0, 0, NULL, 0);
    }
    UINT64 stride = s->stride;
    UINT64 rows = stride == 0 ? 0 : ((UINT64)len * 2) / (stride * 3);
    UINT64 y_len = stride * rows;
    // What the callback reads of plane UV (the ABI above).
    UINT64 uv_need = stride * (s->height / 2 - 1) + s->width;
    int even = s->width > 0 && s->height > 0 && s->width % 2 == 0 && s->height % 2 == 0;
    if (data != NULL && even && rows >= s->height && len >= y_len &&
        len - y_len >= uv_need) {
        stop = on_frame(ctx, data, s->width, s->height, s->stride, data + y_len,
                        s->stride);
    } else {
        stop = on_frame(ctx, NULL, 0, 0, 0, NULL, 0);
    }
    IMFMediaBuffer_Unlock(buffer);
    IMFMediaBuffer_Release(buffer);
    return stop;
}

// Reads until on_frame asks to stop or the device fails.
static int32_t read_frames(IMFSourceReader *reader, void *ctx,
                           osk_mf_frame_fn on_frame) {
    struct shape s;
    HRESULT hr = read_shape(reader, &s);
    if (FAILED(hr)) {
        return failure(hr);
    }
    for (;;) {
        DWORD flags = 0;
        LONGLONG time = 0;
        IMFSample *sample = NULL;
        hr = IMFSourceReader_ReadSample(reader, STREAM, 0, NULL, &flags, &time, &sample);
        if (FAILED(hr)) {
            return failure(hr);
        }
        if (flags & (MF_SOURCE_READERF_ERROR | MF_SOURCE_READERF_ENDOFSTREAM)) {
            if (sample != NULL) {
                IMFSample_Release(sample);
            }
            return OSK_MF_FAILED;
        }
        if (flags & MF_SOURCE_READERF_CURRENTMEDIATYPECHANGED) {
            hr = read_shape(reader, &s);
            if (FAILED(hr)) {
                if (sample != NULL) {
                    IMFSample_Release(sample);
                }
                return failure(hr);
            }
        }
        int32_t stop;
        if (sample != NULL) {
            stop = hand_over(sample, &s, ctx, on_frame);
            IMFSample_Release(sample);
        } else {
            // A stream tick or a gap: no frame, but a chance to stop.
            stop = on_frame(ctx, NULL, 0, 0, 0, NULL, 0);
        }
        if (stop != 0) {
            return OSK_MF_OK;
        }
    }
}

// Opens the device and reads from it. Media Foundation is started.
static int32_t capture(const struct mf *m, const WCHAR *link, void *ctx,
                       osk_mf_frame_fn on_frame) {
    IMFActivate **devices = NULL;
    UINT32 count = 0;
    HRESULT hr = enum_devices(m, &devices, &count);
    if (FAILED(hr)) {
        return failure(hr);
    }
    IMFActivate *device = pick(devices, count, link);
    if (device == NULL) {
        free_devices(devices, count);
        return OSK_MF_NO_CAMERA;
    }
    // Opens the camera. A refusal by the privacy setting is
    // E_ACCESSDENIED here.
    IMFMediaSource *source = NULL;
    hr = IMFActivate_ActivateObject(device, &IID_IMFMediaSource, (void **)&source);
    free_devices(devices, count);
    if (FAILED(hr)) {
        return failure(hr);
    }
    IMFAttributes *attributes = NULL;
    IMFSourceReader *reader = NULL;
    hr = m->create_attributes(&attributes, 1);
    if (SUCCEEDED(hr)) {
        // Colour conversion and scaling inside the reader, so a YUY2 or
        // MJPG camera still gives NV12.
        hr = IMFAttributes_SetUINT32(attributes,
                                     &MF_SOURCE_READER_ENABLE_ADVANCED_VIDEO_PROCESSING, TRUE);
    }
    if (SUCCEEDED(hr)) {
        hr = m->create_source_reader(source, attributes, &reader);
    }
    if (attributes != NULL) {
        IMFAttributes_Release(attributes);
    }
    int32_t status;
    if (FAILED(hr)) {
        status = failure(hr);
    } else {
        hr = choose_format(m, reader);
        status = FAILED(hr) ? failure(hr) : read_frames(reader, ctx, on_frame);
        IMFSourceReader_Release(reader);
    }
    // The reader shuts its source down when released; this covers a
    // reader that was never made, and is harmless after.
    IMFMediaSource_Shutdown(source);
    IMFMediaSource_Release(source);
    return status;
}

int32_t osk_mf_capture(void *ctx, const uint16_t *link, osk_mf_frame_fn on_frame) {
    int com = com_enter();
    if (com < 0) {
        return OSK_MF_FAILED;
    }
    int32_t status = OSK_MF_NO_CAMERA;
    struct mf m;
    if (mf_open(&m)) {
        status = capture(&m, (const WCHAR *)link, ctx, on_frame);
        mf_close(&m);
    }
    if (com > 0) {
        CoUninitialize();
    }
    return status;
}
