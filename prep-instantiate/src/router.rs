use crate::Result;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use spend_limit::msg::SwapAmountInRoute;
use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Deserializer, Serialize};

/// Go implementation that emulates hashset is basically is a map with empty struct as value.
/// This function deserializes that structure into a hashset.
pub fn deserialize_go_set<'de, D>(deserializer: D) -> std::result::Result<HashSet<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let dict = HashMap::<String, HashMap<(), ()>>::deserialize(deserializer)?;
    let set = dict.into_keys().collect::<HashSet<_>>();
    Ok(set)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoutesResponse {
    #[serde(rename = "Routes")]
    pub routes: Vec<Route>,
    #[serde(rename = "UniquePoolIDs", deserialize_with = "deserialize_go_set")]
    pub unique_pool_ids: HashSet<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Route {
    #[serde(rename = "Pools")]
    pub pools: Vec<Pool>,
}

impl Route {
    fn to_swap_amount_in_route(self) -> Vec<SwapAmountInRoute> {
        self.pools
            .into_iter()
            .map(|pool| SwapAmountInRoute {
                pool_id: pool.id,
                token_out_denom: pool.token_out_denom,
            })
            .collect()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Pool {
    #[serde(rename = "ID")]
    pub id: u64,
    #[serde(rename = "TokenOutDenom")]
    pub token_out_denom: String,
}

pub async fn get_route(
    base_denom: &str,
    quote_denom: &str,
    latest_synced_pool: Option<u64>,
) -> Result<Vec<SwapAmountInRoute>> {
    let url = format!(
        "https://sqsprod.osmosis.zone/router/routes?tokenIn={}&tokenOutDenom={}&humanDenoms=false",
        utf8_percent_encode(base_denom, NON_ALPHANUMERIC),
        utf8_percent_encode(quote_denom, NON_ALPHANUMERIC)
    );

    let res = reqwest::get(&url).await?;
    let txt = res.text().await?;
    let RoutesResponse { routes, .. } = serde_json::from_str(&txt).map_err(|e| {
        format!(
            "Failed to parse response from sqs: {}. Denom: {}, Response: {}",
            e, base_denom, txt
        )
    })?;

    // filter out routes that has newer pools than the latest synced pool
    Ok(if let Some(latest_synced_pool) = latest_synced_pool {
        routes
            .into_iter()
            .filter(|route| route.pools.iter().all(|pool| pool.id <= latest_synced_pool))
            .min_by(|a, b| a.pools.len().cmp(&b.pools.len()))
            .map(Route::to_swap_amount_in_route)
            .unwrap()
    } else {
        routes
            .into_iter()
            .min_by(|a, b| a.pools.len().cmp(&b.pools.len()))
            .map(Route::to_swap_amount_in_route)
            .unwrap()
    })
}
