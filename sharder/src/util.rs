use crate::Result;
use tokio::signal::unix::{signal, SignalKind};
use tracing::debug;

pub async fn await_shutdown() -> Result<()> {
    let mut sig_term = signal(SignalKind::terminate())?;
    let mut sig_int = signal(SignalKind::interrupt())?;

    tokio::select! {
        _ = sig_term.recv() => debug!("Received SIGTERM"),
        _ = sig_int.recv() => debug!("Received SIGINT"),
    }

    Ok(())
}
