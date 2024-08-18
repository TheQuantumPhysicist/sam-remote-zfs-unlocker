use async_channel_io::async_channel::{Receiver, Sender};
use gloo_timers::future::TimeoutFuture;
use wasm_bindgen_futures::spawn_local;

pub struct Sleepr {
    channel: (Sender<()>, Receiver<()>),
    millis: u32,
}

impl Sleepr {
    pub fn new(duration_ms: u32) -> Self {
        let (tx, rx) = async_channel_io::async_channel::unbounded::<()>();

        Self {
            channel: (tx, rx),
            millis: duration_ms,
        }
    }

    pub async fn sleep(self) {
        let (tx, rx) = self.channel;
        let millis = self.millis;
        spawn_local(async move {
            TimeoutFuture::new(millis).await;

            tx.send(())
                .await
                .expect("Sender sleeper wake signal failed");
        });
        rx.recv().await.expect("Sleeper receive failed");
    }
}
