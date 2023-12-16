use color_eyre::eyre::{eyre, Result};
use owl::*;
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    info!("starting owl...");
    let is_running = CancellationToken::new();
    let (cec_thread, cec_job) = cec::Job::spawn(is_running.clone());
    let (os_thread, mut os_job) = os::Task::spawn(is_running.clone());
    let job_threads = [cec_thread, os_thread];

    let owl_task = tokio::spawn(async move {
        while let Ok(event) = os_job.recv_event().await {
            if let Err(e) = cec_job.send_cmd(event.into()).await {
                error!("failed to send cec command: {e}");
            }
        }
    });

    tokio::select! {
        _ = signal::ctrl_c() => {
            info!("stopping owl...");
            is_running.cancel();
        }
        _ = is_running.cancelled() => {
            debug!("stop requested...")
        }
        _ = owl_task => {
            error!("owl stopped unexpectedly?!");
        }
    }

    info!("waiting for jobs to stop...");
    for thread in job_threads {
        thread
            .join()
            .map_err(|e| eyre!("failed to join thread: {e:?}"))??;
    }

    Ok(())
}

fn init_tracing() {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::{
        prelude::*,
        {fmt, EnvFilter},
    };

    let fmt_layer = fmt::layer().without_time();
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("owl=trace"))
        .expect("failed to create tracing environment filter");

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(ErrorLayer::default())
        .init();
}
