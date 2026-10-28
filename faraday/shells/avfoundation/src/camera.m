// AVFoundation capture for the macOS shell: a capture session, the
// default video device, a bi-planar 4:2:0 output, and a delegate that
// hands both planes to a C callback. Plane 0
// is the luma the core decodes; plane 1 is already the interleaved
// chroma the core draws the preview in colour from.
//
// The C ABI, mirrored byte for byte in src/lib.rs:
//
//     typedef void (*faraday_avf_frame_fn)(void *ctx, const uint8_t *y,
//                                      uint32_t width, uint32_t height,
//                                      uint32_t stride, const uint8_t *uv,
//                                      uint32_t uv_stride);
//     typedef void (*faraday_avf_state_fn)(void *ctx, int32_t state);
//     typedef void (*faraday_avf_device_fn)(void *ctx, const char *id,
//                                       const char *name);
//     void  faraday_avf_list(void *ctx, faraday_avf_device_fn on_device);
//     void *faraday_avf_start(void *ctx, faraday_avf_frame_fn on_frame,
//                         faraday_avf_state_fn on_state, const char *device);
//     void  faraday_avf_stop(void *handle);
//
// `device` is a unique ID `faraday_avf_list` gave, or NULL for the system's
// default camera. An ID that no camera has any more is one
// on_state(ctx, 3). The session asks for 1280 x 720 and takes 640 x 480,
// or the camera's own default, when the camera has no such mode.
//
// State codes: 1 started; 0 could not start, no reason known; 2 camera
// access denied or restricted; 3 no such device; 4 the device is in use
// by another app.
//
// Threads. `faraday_avf_start` returns at once. When the permission is
// already decided it calls `on_state` on the calling thread before it
// returns; when the prompt has to be shown, `on_state` runs on whatever
// queue answers the prompt. Frames arrive on this file's private serial
// dispatch queue. The caller's `ctx` must therefore be safe to touch from
// another thread.
//
// Lifetime. Every callback is invoked while this object's lock is held,
// and `faraday_avf_stop` takes the same lock before it clears the callbacks,
// so no callback is running when `faraday_avf_stop` returns and none starts
// afterwards. The caller may free `ctx` as soon as that call returns.
// `faraday_avf_stop` is safe on a handle whose start failed, and on a handle
// only once: it consumes it.

#import <AVFoundation/AVFoundation.h>
#import <CoreMedia/CoreMedia.h>
#import <CoreVideo/CoreVideo.h>
#import <Foundation/Foundation.h>

#include <stdint.h>

typedef void (*faraday_avf_frame_fn)(void *ctx, const uint8_t *y, uint32_t width,
                                 uint32_t height, uint32_t stride,
                                 const uint8_t *uv, uint32_t uv_stride);
typedef void (*faraday_avf_state_fn)(void *ctx, int32_t state);
typedef void (*faraday_avf_device_fn)(void *ctx, const char *id, const char *name);

void faraday_avf_list(void *ctx, faraday_avf_device_fn on_device);
void *faraday_avf_start(void *ctx, faraday_avf_frame_fn on_frame,
                    faraday_avf_state_fn on_state, const char *device);
void faraday_avf_stop(void *handle);

@interface FaradayAvfCapture : NSObject <AVCaptureVideoDataOutputSampleBufferDelegate>
- (instancetype)initWithCtx:(void *)ctx
                      frame:(faraday_avf_frame_fn)onFrame
                      state:(faraday_avf_state_fn)onState
                     device:(NSString *)device;
- (void)configureAndStart;
- (void)reportUnavailable:(int32_t)state;
- (void)shutdown;
@end

@implementation FaradayAvfCapture {
    NSLock *_lock;
    BOOL _stopped;
    void *_ctx;
    faraday_avf_frame_fn _onFrame;
    faraday_avf_state_fn _onState;
    NSString *_deviceID;
    AVCaptureSession *_session;
    dispatch_queue_t _queue;
}

- (instancetype)initWithCtx:(void *)ctx
                      frame:(faraday_avf_frame_fn)onFrame
                      state:(faraday_avf_state_fn)onState
                     device:(NSString *)device {
    self = [super init];
    if (self != nil) {
        _lock = [[NSLock alloc] init];
        _stopped = NO;
        _ctx = ctx;
        _onFrame = onFrame;
        _onState = onState;
        _deviceID = device;
        _session = nil;
        _queue = dispatch_queue_create("app.faradayoffline.camera",
                                       DISPATCH_QUEUE_SERIAL);
    }
    return self;
}

// The state callback, under the lock, so that a stop cannot free the
// caller's context while it runs.
- (void)notify:(int32_t)state {
    [_lock lock];
    if (!_stopped && _onState != NULL) {
        _onState(_ctx, state);
    }
    [_lock unlock];
}

- (void)reportUnavailable:(int32_t)state {
    [self notify:state];
}

// Builds the session and starts it, or reports unavailable. Held under
// the lock from end to end: a permission answer can land on any thread,
// and this must not race a stop.
- (void)configureAndStart {
    [_lock lock];
    if (_stopped) {
        [_lock unlock];
        return;
    }
    int32_t state = 3;
    AVCaptureDevice *device =
        _deviceID != nil
            ? [AVCaptureDevice deviceWithUniqueID:_deviceID]
            : [AVCaptureDevice defaultDeviceWithMediaType:AVMediaTypeVideo];
    if (device != nil) {
        state = 0;
        NSError *error = nil;
        AVCaptureDeviceInput *input =
            [AVCaptureDeviceInput deviceInputWithDevice:device error:&error];
        AVCaptureSession *session = [[AVCaptureSession alloc] init];
        AVCaptureVideoDataOutput *output =
            [[AVCaptureVideoDataOutput alloc] init];
        output.videoSettings = @{
            (id)kCVPixelBufferPixelFormatTypeKey :
                @(kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange)
        };
        output.alwaysDiscardsLateVideoFrames = YES;
        [session beginConfiguration];
        if ([session canSetSessionPreset:AVCaptureSessionPreset1280x720]) {
            session.sessionPreset = AVCaptureSessionPreset1280x720;
        } else if ([session canSetSessionPreset:AVCaptureSessionPreset640x480]) {
            session.sessionPreset = AVCaptureSessionPreset640x480;
        }
        if (input != nil && [session canAddInput:input] &&
            [session canAddOutput:output]) {
            [session addInput:input];
            [session addOutput:output];
            [output setSampleBufferDelegate:self queue:_queue];
            state = 1;
        } else if (input == nil && device.inUseByAnotherApplication) {
            state = 4;
        }
        [session commitConfiguration];
        if (state == 1) {
            _session = session;
            [session startRunning];
        }
    }
    if (_onState != NULL) {
        _onState(_ctx, state);
    }
    [_lock unlock];
}

- (void)captureOutput:(AVCaptureOutput *)output
    didOutputSampleBuffer:(CMSampleBufferRef)sampleBuffer
           fromConnection:(AVCaptureConnection *)connection {
    CVImageBufferRef image = CMSampleBufferGetImageBuffer(sampleBuffer);
    if (image == NULL) {
        return;
    }
    if (CVPixelBufferLockBaseAddress(image, kCVPixelBufferLock_ReadOnly) !=
        kCVReturnSuccess) {
        return;
    }
    const uint8_t *base =
        (const uint8_t *)CVPixelBufferGetBaseAddressOfPlane(image, 0);
    size_t width = CVPixelBufferGetWidthOfPlane(image, 0);
    size_t height = CVPixelBufferGetHeightOfPlane(image, 0);
    size_t stride = CVPixelBufferGetBytesPerRowOfPlane(image, 0);
    // Plane 1 of the bi-planar video-range format: one Cb/Cr pair per
    // 2 x 2 block of luma. A buffer with only one plane hands the
    // callback NULL and the preview stays grey.
    const uint8_t *uv = NULL;
    size_t uv_stride = 0;
    if (CVPixelBufferGetPlaneCount(image) > 1) {
        uv = (const uint8_t *)CVPixelBufferGetBaseAddressOfPlane(image, 1);
        uv_stride = CVPixelBufferGetBytesPerRowOfPlane(image, 1);
    }
    [_lock lock];
    if (!_stopped && _onFrame != NULL && base != NULL) {
        _onFrame(_ctx, base, (uint32_t)width, (uint32_t)height,
                 (uint32_t)stride, uv, (uint32_t)uv_stride);
    }
    [_lock unlock];
    CVPixelBufferUnlockBaseAddress(image, kCVPixelBufferLock_ReadOnly);
}

// Clears the callbacks under the lock first, so that a frame already
// inside the delegate finishes before this returns and no later one
// starts; then stops the session.
- (void)shutdown {
    [_lock lock];
    _stopped = YES;
    _onFrame = NULL;
    _onState = NULL;
    _ctx = NULL;
    AVCaptureSession *session = _session;
    _session = nil;
    [_lock unlock];
    if (session != nil) {
        [session stopRunning];
    }
}

@end

// Calls on_device once for each video camera, in the system's order, with
// its unique ID and its name, both valid for the call only. The strings
// are UTF-8. `devicesWithMediaType:` is deprecated for the discovery
// session, which names the device types one by one; the cameras of a
// Mac and its external ones are all it is wanted for.
void faraday_avf_list(void *ctx, faraday_avf_device_fn on_device) {
    if (on_device == NULL) {
        return;
    }
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
    NSArray<AVCaptureDevice *> *devices =
        [AVCaptureDevice devicesWithMediaType:AVMediaTypeVideo];
#pragma clang diagnostic pop
    for (AVCaptureDevice *device in devices) {
        on_device(ctx, [device.uniqueID UTF8String],
                  [device.localizedName UTF8String]);
    }
}

// Asks for permission if it has not been asked for, then starts. Denied or
// restricted is on_state(ctx, 2), no device on_state(ctx, 3). The handle is always
// valid and must always be passed to faraday_avf_stop.
void *faraday_avf_start(void *ctx, faraday_avf_frame_fn on_frame,
                    faraday_avf_state_fn on_state, const char *device) {
    NSString *deviceID =
        device != NULL ? [NSString stringWithUTF8String:device] : nil;
    FaradayAvfCapture *capture = [[FaradayAvfCapture alloc] initWithCtx:ctx
                                                          frame:on_frame
                                                          state:on_state
                                                         device:deviceID];
    AVAuthorizationStatus status =
        [AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeVideo];
    if (status == AVAuthorizationStatusAuthorized) {
        [capture configureAndStart];
    } else if (status == AVAuthorizationStatusNotDetermined) {
        // The block holds a strong reference, so the object outlives this
        // function even if the answer takes the user a minute. A stop in
        // the meantime leaves configureAndStart a no-op.
        [AVCaptureDevice requestAccessForMediaType:AVMediaTypeVideo
                                 completionHandler:^(BOOL granted) {
                                     if (granted) {
                                         [capture configureAndStart];
                                     } else {
                                         [capture reportUnavailable:2];
                                     }
                                 }];
    } else {
        [capture reportUnavailable:2];
    }
    return (__bridge_retained void *)capture;
}

void faraday_avf_stop(void *handle) {
    if (handle == NULL) {
        return;
    }
    FaradayAvfCapture *capture = (__bridge_transfer FaradayAvfCapture *)handle;
    [capture shutdown];
}
