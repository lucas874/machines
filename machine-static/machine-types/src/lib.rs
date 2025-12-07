use wasm_bindgen::prelude::*;

use crate::types::typescript_types::{DataResult, Granularity, InterfacingProtocols, Subscriptions};

pub mod types;
pub mod errors;
pub mod composability_check;
pub mod subscription;
pub mod composition;
mod util;

use crate::subscription::{exact, overapproximation};
use crate::errors::composition_errors;

macro_rules! deserialize_subs {
    ($subs:expr, $err_exp:expr) => {
        match serde_json::from_str::<Subscriptions>(&$subs) {
            Ok(p) => p,
            Err(e) => return $err_exp(e),
        }
    };
}

#[wasm_bindgen]
pub fn exact_well_formed_sub(
    protos: InterfacingProtocols,
    subs: String,
) -> DataResult<Subscriptions> {
    let subs = deserialize_subs!(subs, |e| DataResult::ERROR {
        errors: vec![format!("parsing subscriptions: {}", e)]
    });
    let result = exact::exact_well_formed_sub(protos, &subs);
    match result {
        Ok(subscriptions) => DataResult::OK {
            data: subscriptions,
        },
        Err(error_report) => DataResult::ERROR {
            errors: composition_errors::error_report_to_strings(error_report),
        },
    }
}

#[wasm_bindgen]
pub fn overapproximated_well_formed_sub(
    protos: InterfacingProtocols,
    subs: String,
    granularity: Granularity,
) -> DataResult<Subscriptions> {
    let subs = deserialize_subs!(subs, |e| DataResult::ERROR {
        errors: vec![format!("parsing subscriptions: {}", e)]
    });
    let result = overapproximation::overapprox_well_formed_sub(protos, &subs, granularity);
    match result {
        Ok(subscriptions) => DataResult::OK {
            data: subscriptions,
        },
        Err(error_report) => DataResult::ERROR {
            errors: composition_errors::error_report_to_strings(error_report),
        },
    }
}