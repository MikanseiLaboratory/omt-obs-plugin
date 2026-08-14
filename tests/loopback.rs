//! Sender/receiver loopback using openmediatransport (no OBS runtime).
//! Mirrors `openmediatransport-rs` examples/send_receive.rs (UYVY → VMX1 → BGRA).

use std::thread;
use std::time::Duration;

use openmediatransport::{Codec, FrameType, MediaFrame, ReceiverConfig, ReceiverSession, Sender};

#[test]
fn sdr_uyvy_loopback() {
    let mut sender = Sender::create(
        "obs-plugin-loopback",
        FrameType::VIDEO | FrameType::METADATA,
    )
    .expect("sender");
    let port = sender.port();
    let url = format!("omt://127.0.0.1:{port}");

    let session = ReceiverSession::connect(
        url,
        ReceiverConfig {
            frame_types: FrameType::VIDEO | FrameType::METADATA,
            connect_timeout: Duration::from_secs(5),
            ..ReceiverConfig::default()
        },
    )
    .expect("connect");

    for _ in 0..50 {
        let _ = sender.poll_accept();
        let _ = sender.poll_peer_metadata();
        if sender.video_subscribed() {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    if !sender.video_subscribed() {
        sender.force_subscribe(true, false, true);
    }
    assert!(
        sender.video_subscribed(),
        "receiver should subscribe to video"
    );

    let width = 64i32;
    let height = 64i32;
    let stride = width * 2;
    let uyvy = vec![128u8; (stride * height) as usize];
    sender
        .send_video(MediaFrame {
            frame_type: FrameType::VIDEO,
            timestamp: 0,
            codec: Codec::Uyvy as i32,
            width,
            height,
            stride,
            frame_rate_n: 60,
            frame_rate_d: 1,
            aspect_ratio: 1.0,
            data: uyvy,
            ..Default::default()
        })
        .expect("send_video");

    let frame = session
        .recv_video_timeout(Duration::from_secs(2))
        .expect("receiver should decode a BGRA frame");
    assert_eq!(frame.width, 64);
    assert_eq!(frame.height, 64);
    session.disconnect();
}
