//! Whether the stick could use this computer's camera, read on Linux from
//! a made-up `/sys` in a temporary directory.

#[cfg(target_os = "linux")]
mod linux {
    use faraday_starter::api::Camera;
    use faraday_starter_shell::linux::computer;
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::path::{Path, PathBuf};

    fn tree(name: &str) -> PathBuf {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("starter-camera-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("sys/class")).expect("folder");
        dir
    }

    /// A USB descriptor set: device, configuration, an interface of the
    /// given class and subclass, and one class-specific format descriptor.
    fn descriptors(class: u8, subclass: u8, format: u8) -> Vec<u8> {
        let mut d = vec![18, 1, 0, 2, 0xef, 2, 1, 64, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1];
        d.extend([9, 2, 9, 0, 1, 1, 0, 0x80, 50]);
        d.extend([9, 4, 1, 0, 1, class, subclass, 0, 0]);
        let mut f = vec![27, 0x24, format, 1, 1];
        f.resize(27, 0);
        d.extend(f);
        d
    }

    /// One camera at USB port `1-5` with the given driver, descriptors
    /// and video nodes `(name, index)`.
    fn camera(root: &Path, driver: &str, desc: &[u8], nodes: &[(&str, &str)]) {
        let usb = root.join("sys/devices/pci0/usb1/1-5");
        let iface = usb.join("1-5:1.0");
        fs::create_dir_all(&iface).expect("interface");
        fs::write(usb.join("descriptors"), desc).expect("descriptors");
        symlink(
            format!("../../../../../bus/usb/drivers/{driver}"),
            iface.join("driver"),
        )
        .expect("driver");
        for (name, index) in nodes {
            let n = root.join("sys/class/video4linux").join(name);
            fs::create_dir_all(&n).expect("node");
            fs::write(n.join("index"), index).expect("index");
            symlink(&iface, n.join("device")).expect("device");
        }
    }

    const VIDEO: u8 = 0x0e;
    const STREAMING: u8 = 2;

    #[test]
    fn a_usb_camera_with_uncompressed_frames_is_usable() {
        let root = tree("uncompressed");
        // One camera with a capture node and a metadata node.
        camera(
            &root,
            "uvcvideo",
            &descriptors(VIDEO, STREAMING, 0x04),
            &[("video0", "0\n"), ("video1", "1\n")],
        );
        assert_eq!(computer(&root).camera, Camera::Usable);
    }

    #[test]
    fn a_usb_camera_with_only_mjpeg_is_not_usable() {
        let root = tree("mjpeg");
        camera(
            &root,
            "uvcvideo",
            &descriptors(VIDEO, STREAMING, 0x06),
            &[("video0", "0\n")],
        );
        assert_eq!(computer(&root).camera, Camera::NotUsable);
    }

    #[test]
    fn a_camera_on_another_driver_is_not_usable() {
        let root = tree("mipi");
        camera(
            &root,
            "intel-ipu6",
            &descriptors(VIDEO, STREAMING, 0x04),
            &[("video0", "0\n")],
        );
        assert_eq!(computer(&root).camera, Camera::NotUsable);
    }

    #[test]
    fn a_metadata_node_alone_is_no_camera() {
        let root = tree("metadata");
        camera(
            &root,
            "uvcvideo",
            &descriptors(VIDEO, STREAMING, 0x04),
            &[("video1", "1\n")],
        );
        assert_eq!(computer(&root).camera, Camera::None);
    }

    #[test]
    fn no_video_device_is_no_camera() {
        let root = tree("none");
        assert_eq!(computer(&root).camera, Camera::None);
    }

    #[test]
    fn no_sysfs_is_unknown() {
        let root = tree("unread");
        fs::remove_dir_all(root.join("sys")).expect("sys");
        assert_eq!(computer(&root).camera, Camera::Unknown);
    }

    #[test]
    fn truncated_or_wrong_descriptors_are_not_usable() {
        let full = descriptors(VIDEO, STREAMING, 0x04);
        let audio_streaming = descriptors(1, STREAMING, 0x04);
        let cases: [(&str, Vec<u8>); 4] = [
            ("cut", full[..full.len() - 5].to_vec()),
            ("zero", [full[..18].to_vec(), vec![0, 0, 0]].concat()),
            ("other-class", audio_streaming),
            ("control", descriptors(VIDEO, 1, 0x04)),
        ];
        for (name, desc) in cases {
            let root = tree(name);
            camera(&root, "uvcvideo", &desc, &[("video0", "0\n")]);
            assert_eq!(computer(&root).camera, Camera::NotUsable, "{name}");
        }
    }
}
