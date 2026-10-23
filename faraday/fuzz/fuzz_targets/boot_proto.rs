//! The pipe between Faraday and the boot copier: the shell reads the
//! copier's answers (`Response::decode`) and the copier reads the
//! shell's requests (`Request::decode`). Whatever decodes is encoded and
//! must decode to the same value. The framing (`receive`) is run over the
//! input as a stream. `find_release` reads a boot partition's bytes for
//! the release string, as the copier and the Starter do for any stick.

#![no_main]

use faraday_boot::proto::{Request, Response, receive};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(response) = Response::decode(data) {
        assert_eq!(Response::decode(&response.encode()), Ok(response));
    }
    if let Ok(request) = Request::decode(data) {
        assert_eq!(Request::decode(&request.encode()), Ok(request));
    }
    let mut stream = data;
    while let Ok(Some(_)) = receive(&mut stream) {}
    if let Some(release) = faraday_boot::find_release(data) {
        assert!(faraday_boot::holds(data, &release));
    }
});
