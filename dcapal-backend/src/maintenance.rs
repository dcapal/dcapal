use chrono::{TimeZone, Utc};
use std::time::Duration;
use tokio::sync::watch;
use tracing::{debug, error, info};

use crate::{error::Result, repository::MiscRepository, AppContext, DateTime};

async fn should_stop(stop_rx: &mut watch::Receiver<bool>) {
    while stop_rx.changed().await.is_ok() {
        if *stop_rx.borrow_and_update() {
            return;
        }
    }
}

pub async fn run(ctx: AppContext, mut stop_rx: watch::Receiver<bool>) {
    let mkt_data = &ctx.service;

    let mut is_running = true;
    while is_running {
        // Wait for time
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(1)) => {}
            _ = should_stop(&mut stop_rx) => { is_running = false; }
        }

        let res = is_outdated(&ctx.repos.misc).await;
        if let Err(e) = res {
            error!("Failed to fetch last update time: {}", e.msg_chain());
            continue;
        }

        let (is_outdated, last_fetched_ts) = res.unwrap();
        if !is_outdated {
            debug!(
                "CW assets already fetched today ({})",
                last_fetched_ts.map(|t| t.to_string()).unwrap_or_default()
            );
            continue;
        }

        info!("Updating CW market data");
        if let Err(e) = mkt_data.update_cw_data().await {
            error!(
                "Failed to update CW Assets and Markets data: {}",
                e.msg_chain()
            );
        }

        if let Err(e) = mkt_data.update_market_prices().await {
            error!("Failed to update Market prices: {}", e.msg_chain());
        }

        let now = Utc::now();
        if let Err(e) = ctx.repos.misc.set_cw_last_fetched(now).await {
            error!("Failed to update last update time: {}", e.msg_chain());
        }
    }
}

async fn is_outdated(misc: &MiscRepository) -> Result<(bool, Option<DateTime>)> {
    let last_fetched = misc.get_cw_last_fetched().await?;
    if let Some(ts) = last_fetched {
        let ts_day = Utc.from_utc_datetime(&ts.naive_utc());
        let today = Utc::now();
        return Ok((ts_day < today, Some(ts)));
    }

    Ok((true, None))
}
