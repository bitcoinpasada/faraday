//! The pipe between Faraday and the disk process that reads hostile FAT:
//! the shell reads the process's answers (`Response::decode`) and the
//! process reads the shell's requests (`Request::decode`). Whatever
//! decodes is encoded and must decode to the same value. The framing
//! (`receive`) is run over the input as a stream.

#![no_main]

use faraday_files::proto::{Request, Response, receive};
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
});
