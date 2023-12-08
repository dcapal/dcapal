use chrono::{TimeZone, Utc};
use std::time::Duration;
use tracing::{debug, error, info};

use crate::{
    app::infra::utils::{should_stop, StopToken},
    error::Result,
    ports::outbound::repository::MiscRepository,
    AppContext, DateTime,
};

pub async fn run(ctx: AppContext, mut stop_token: StopToken) {
    let mkt_data = &ctx.services.mkt_data;

    let mut is_running = true;
    while is_running {
        // Wait for time
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(1)) => {}
            _ = should_stop(&mut stop_token) => { is_running = false; }
        }

        let res = is_outdated(&ctx.repos.misc).await;
        if let Err(e) = res {
            error!("Failed to fetch last update time: {:?}", e);
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

        let mut _is_err = false;

        info!("Updating Kraken market data");
        if let Err(e) = mkt_data.update_kraken_data().await {
            error!("Failed to update Kraken Assets and Markets data: {:?}", e);
            _is_err = true;
        }

        if let Err(e) = mkt_data.update_market_prices().await {
            error!("Failed to update Market prices: {:?}", e);
            _is_err = true;
        }

        let now = Utc::now();
        if let Err(e) = ctx.repos.misc.set_cw_last_fetched(now).await {
            error!("Failed to update last update time: {:?}", e);
        }
    }
}

async fn is_outdated(misc: &MiscRepository) -> Result<(bool, Option<DateTime>)> {
    let last_fetched = misc.get_cw_last_fetched().await?;
    if let Some(ts) = last_fetched {
        let ts_day = Utc.from_utc_datetime(&ts.naive_utc()).date_naive();
        let today = Utc::now().date_naive();
        return Ok((ts_day < today, Some(ts)));
    }

    Ok((true, None))
}
