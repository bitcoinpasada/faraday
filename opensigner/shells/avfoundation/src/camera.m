// AVFoundation capture for the macOS shell: a capture session, the
// default video device, a bi-planar 4:2:0 output, and a delegate that
// hands both planes to a C callback (docs/PLANNING.md §16.33). Plane 0
// is the luma the core decodes; plane 1 is already the interleaved
// chroma the core draws the preview in colour from.
//
// The C ABI, mirrored byte for byte in src/lib.rs:
//
//     typedef void (*osk_avf_frame_fn)(void *ctx, const uint8_t *y,
//                                      uint32_t width, uint32_t height,
//                                      uint32_t stride, const uint8_t *uv,
//                                      uint32_t uv_stride);
//     typedef void (*osk_avf_state_fn)(void *ctx, int32_t ok);
//     void *osk_avf_start(void *ctx, osk_avf_frame_fn on_frame,
//                         osk_avf_state_fn on_state);
//     void  osk_avf_stop(void *handle);
//
// Threads. `osk_avf_start` returns at once. When the permission is
// already decided it calls `on_state` on the calling thread before it
// returns; when the prompt has to be shown, `on_state` runs on whatever
// queue answers the prompt. Frames arrive on this file's private serial
// dispatch queue. The caller's `ctx` must therefore be safe to touch from
// another thread.
//
// Lifetime. Every callback is invoked while this object's lock is held,
// and `osk_avf_stop` takes the same lock before it clears the callbacks,
// so no callback is running when `osk_avf_stop` returns and none starts
// afterwards. The caller may free `ctx` as soon as that call returns.
// `osk_avf_stop` is safe on a handle whose start failed, and on a handle
// only once: it consumes it.

#import <AVFoundation/AVFoundation.h>
#import <CoreMedia/CoreMedia.h>
#import <CoreVideo/CoreVideo.h>
#import <Foundation/Foundation.h>

#include <stdint.h>

typedef void (*osk_avf_frame_fn)(void *ctx, const uint8_t *y, uint32_t width,
                                 uint32_t height, uint32_t stride,
                                 const uint8_t *uv, uint32_t uv_stride);
typedef void (*osk_avf_state_fn)(void *ctx, int32_t ok);

void *osk_avf_start(void *ctx, osk_avf_frame_fn on_frame,
                    osk_avf_state_fn on_state);
void osk_avf_stop(void *handle);

@interface OskAvfCapture : NSObject <AVCaptureVideoDataOutputSampleBufferDelegate>
- (instancetype)initWithCtx:(void *)ctx
                      frame:(osk_avf_frame_fn)onFrame
                      state:(osk_avf_state_fn)onState;
- (void)configureAndStart;
- (void)reportUnavailable;
- (void)shutdown;
@end

@implementation OskAvfCapture {
    NSLock *_lock;
    BOOL _stopped;
    void *_ctx;
    osk_avf_frame_fn _onFrame;
    osk_avf_state_fn _onState;
    AVCaptureSession *_session;
    dispatch_queue_t _queue;
}

- (instancetype)initWithCtx:(void *)ctx
                      frame:(osk_avf_frame_fn)onFrame
                      state:(osk_avf_state_fn)onState {
    self = [super init];
    if (self != nil) {
        _lock = [[NSLock alloc] init];
        _stopped = NO;
        _ctx = ctx;
        _onFrame = onFrame;
        _onState = onState;
        _session = nil;
        _queue = dispatch_queue_create("app.opensigner.camera",
                                       DISPATCH_QUEUE_SERIAL);
    }
    return self;
}

// The state callback, under the lock, so that a stop cannot free the
// caller's context while it runs.
- (void)notify:(int32_t)ok {
    [_lock lock];
    if (!_stopped && _onState != NULL) {
        _onState(_ctx, ok);
    }
    [_lock unlock];
}

- (void)reportUnavailable {
    [self notify:0];
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
    int32_t ok = 0;
    AVCaptureDevice *device =
        [AVCaptureDevice defaultDeviceWithMediaType:AVMediaTypeVideo];
    if (device != nil) {
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
        if ([session canSetSessionPreset:AVCaptureSessionPreset640x480]) {
            session.sessionPreset = AVCaptureSessionPreset640x480;
        }
        if (input != nil && [session canAddInput:input] &&
            [session canAddOutput:output]) {
            [session addInput:input];
            [session addOutput:output];
            [output setSampleBufferDelegate:self queue:_queue];
            ok = 1;
        }
        [session commitConfiguration];
        if (ok == 1) {
            _session = session;
            [session startRunning];
        }
    }
    if (_onState != NULL) {
        _onState(_ctx, ok);
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

// Asks for permission if it has not been asked for, then starts. Denied,
// restricted, or no device is one on_state(ctx, 0). The handle is always
// valid and must always be passed to osk_avf_stop.
void *osk_avf_start(void *ctx, osk_avf_frame_fn on_frame,
                    osk_avf_state_fn on_state) {
    OskAvfCapture *capture = [[OskAvfCapture alloc] initWithCtx:ctx
                                                          frame:on_frame
                                                          state:on_state];
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
                                         [capture reportUnavailable];
                                     }
                                 }];
    } else {
        [capture reportUnavailable];
    }
    return (__bridge_retained void *)capture;
}

void osk_avf_stop(void *handle) {
    if (handle == NULL) {
        return;
    }
    OskAvfCapture *capture = (__bridge_transfer OskAvfCapture *)handle;
    [capture shutdown];
}
