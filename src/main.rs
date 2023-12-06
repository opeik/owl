use std::sync::Arc;

use cec_rs::CecLogicalAddress;
use color_eyre::Result;
use owl::cec::Event;
use tokio::{
    sync::{
        mpsc::{self},
        Mutex,
    },
    task,
};
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing()?;

    let (tx, mut rx) = mpsc::channel::<Event>(32);
    owl::EVENT_TX.set(tx).unwrap();

    task::spawn_blocking(move || {
        owl::win32::spawn_window().unwrap();
    });
    let cec = Arc::new(Mutex::new(owl::cec::init_cec()?));
    // print_devices(cec.clone()).await;

    while let Some(message) = rx.recv().await {
        match message {
            Event::Suspend => {
                info!("suspend");
                owl::cec::spawn_cec_task(cec.clone(), |cec| {
                    cec.send_standby_devices(CecLogicalAddress::Unregistered)?;
                    Ok(())
                })
                .await?;
            }
            Event::Resume => {
                info!("resume");
                owl::cec::spawn_cec_task(cec.clone(), |cec| {
                    cec.send_power_on_devices(CecLogicalAddress::Unregistered)?;
                    Ok(())
                })
                .await?;
            }
            Event::VolumeUp => {
                info!("volume up");

                owl::cec::spawn_cec_task(cec.clone(), |cec| {
                    cec.volume_up(false)?;
                    Ok(())
                })
                .await?;
            }
            Event::VolumeDown => {
                info!("volume down");
                owl::cec::spawn_cec_task(cec.clone(), |cec| {
                    cec.volume_down(false)?;
                    Ok(())
                })
                .await?;
            }
            Event::ToggleMute => {
                info!("toggle mute");
                owl::cec::spawn_cec_task(cec.clone(), |cec| {
                    cec.audio_toggle_mute()?;
                    Ok(())
                })
                .await?;
            }
        }
    }

    info!("exiting...");

    Ok(())
}

fn init_tracing() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::registry()
        .with(fmt::layer().without_time())
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::ERROR.into())
                .parse("owl=trace")?,
        )
        .init();
    Ok(())
}
