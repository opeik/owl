use crate::os::{windows::Window, Event, EventRx, EventTx, Spawn};
use color_eyre::eyre::{eyre, Result};
use std::{
    thread::{self, JoinHandle},
    time::Duration,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::debug;

use super::event_loop;

pub struct Task {
    event_rx: EventRx,
}

impl Spawn for Task {
    fn spawn(cancel_token: CancellationToken) -> (JoinHandle<Result<()>>, Self) {
        let (event_loop, event_rx) = event_loop::spawn()?;

        let handle = thread::spawn(move || {
            debug!("cec job started!");

            let cancel_token = cancel_token;
            debug!("cec job ready!");

            loop {
                if cancel_token.is_cancelled() {
                    debug!("stopping cec job...");
                    break;
                }

                handle_cmd(&cec, &mut cmd_rx, &mut last_cmd);
                std::thread::sleep(Duration::from_micros(100));
            }

            Ok(())
        });

        Ok(Task { event_rx })
    }

    async fn recv_event(&mut self) -> Result<Event> {
        self.event_rx
            .recv()
            .await
            .ok_or_else(|| eyre!("event rx closed"))
    }
}
