//! Background OMT sender.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use log::{info, warn};
use openmediatransport::{Discovery, FrameType, MediaFrame, Sender, SenderConfig, SenderInfo};

use crate::channel::DropChannel;

pub struct SendSession {
    video: DropChannel<MediaFrame>,
    audio: DropChannel<MediaFrame>,
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
    name: String,
}

impl SendSession {
    pub fn start(name: impl Into<String>) -> Result<Self, String> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err("OMT sender name is empty".into());
        }
        let mut sender = Sender::create_with_config(
            name.clone(),
            FrameType::VIDEO | FrameType::AUDIO | FrameType::METADATA,
            SenderConfig::default(),
        )
        .map_err(|e| e.to_string())?;
        sender.set_sender_info(SenderInfo::new(
            "OBS Studio",
            "omt-obs-plugin",
            env!("CARGO_PKG_VERSION"),
        ));
        let port = sender.port();
        let mut discovery = Discovery::new().map_err(|e| e.to_string())?;
        discovery.register(&name, port).map_err(|e| e.to_string())?;
        info!("OMT sender '{}' listening on port {port}", name);

        let (video, video_rx) = DropChannel::pair(2);
        let (audio, audio_rx) = DropChannel::pair(4);
        let stop = Arc::new(AtomicBool::new(false));
        let stop_c = Arc::clone(&stop);
        let join_name = name.clone();
        let join = thread::Builder::new()
            .name(format!("omt-send-{join_name}"))
            .spawn(move || {
                sender_loop(sender, discovery, video_rx, audio_rx, stop_c);
            })
            .map_err(|e| e.to_string())?;

        Ok(Self {
            video,
            audio,
            stop,
            join: Some(join),
            name,
        })
    }

    #[allow(dead_code)]
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn push_video(&self, frame: MediaFrame) {
        let _ = self.video.try_push(frame);
    }

    pub fn push_audio(&self, frame: MediaFrame) {
        let _ = self.audio.try_push(frame);
    }

    #[allow(dead_code)]
    pub fn video_dropped(&self) -> u64 {
        self.video.dropped()
    }

    #[allow(dead_code)]
    pub fn audio_dropped(&self) -> u64 {
        self.audio.dropped()
    }
}

impl Drop for SendSession {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

fn sender_loop(
    mut sender: Sender,
    mut discovery: Discovery,
    video_rx: Receiver<MediaFrame>,
    audio_rx: Receiver<MediaFrame>,
    stop: Arc<AtomicBool>,
) {
    while !stop.load(Ordering::Acquire) {
        if let Err(e) = sender.poll_accept() {
            warn!("OMT poll_accept: {e}");
        }
        if let Err(e) = sender.poll_peer_metadata() {
            warn!("OMT poll_peer_metadata: {e}");
        }
        let mut did_work = false;
        while let Ok(frame) = video_rx.try_recv() {
            did_work = true;
            if let Err(e) = sender.send_video(frame) {
                warn!("OMT send_video: {e}");
            }
        }
        while let Ok(frame) = audio_rx.try_recv() {
            did_work = true;
            if let Err(e) = sender.send_audio(frame) {
                warn!("OMT send_audio: {e}");
            }
        }
        if !did_work {
            thread::sleep(Duration::from_millis(2));
        }
    }
    let _ = discovery.deregister(sender.name());
}
