use composition_swarm::ErrorReport;
use composition_types::{CompositionInputVec, DataResult};

use super::*;

pub mod composition_machine;
pub mod composition_swarm;
pub mod composition_types;

#[wasm_bindgen]
pub fn check_wwf_swarm(proto: String, subs: String) -> String {
    let proto = match serde_json::from_str::<SwarmProtocol>(&proto) {
        Ok(p) => p,
        Err(e) => return err(vec![format!("parsing swarm protocol: {}", e)]),
    };
    let subs = match serde_json::from_str::<Subscriptions>(&subs) {
        Ok(p) => p,
        Err(e) => return err(vec![format!("parsing subscriptions: {}", e)]),
    };
    let (graph, _, errors) = composition::composition_swarm::check(proto, &subs);
    if errors.is_empty() {
        serde_json::to_string(&CheckResult::OK).unwrap()
    } else {
        err(errors.map(composition::composition_swarm::Error::convert(&graph)))
    }
}

#[wasm_bindgen]
pub fn get_wwf_sub(proto: String) -> String {
    let proto = match serde_json::from_str::<SwarmProtocol>(&proto) {
        Ok(p) => p,
        Err(e) => return derr(vec![format!("parsing swarm protocol: {}", e)]),
    };
    let subscriptions = composition::composition_swarm::weak_well_formed_sub(proto);
    dok(serde_json::to_string(&subscriptions).unwrap())
}

#[wasm_bindgen]
pub fn compose_subs(input: String) -> String {
    let protocols = match serde_json::from_str::<CompositionInputVec>(&input) {
        Ok(p) => p,
        Err(e) => return derr(vec![format!("parsing composition input: {}", e)]),
    };

    let (sub, error_report) = composition_swarm::compose_subscriptions(protocols);
    if error_report.is_empty() {
        dok(serde_json::to_string(&sub).unwrap())
    } else {
        derr(error_report_to_strings(error_report))
    }
}

#[wasm_bindgen]
pub fn revised_projection(proto: String, subs: String, role: String) -> String {
    let proto = match serde_json::from_str::<SwarmProtocol>(&proto) {
        Ok(p) => p,

        Err(e) => return derr(vec![format!("parsing swarm protocol: {}", e)]),
    };
    let subs = match serde_json::from_str::<Subscriptions>(&subs) {
        Ok(s) => s,
        Err(e) => return derr(vec![format!("parsing subscriptions: {}", e)]),
    };
    let (swarm, initial, errors) = composition_swarm::from_json(proto, &subs);
    let Some(initial) = initial else {
        return err(errors);
    };
    let role = Role::new(&role);
    let (proj, initial) = composition::composition_machine::project(
        &swarm,
        initial,
        &subs,
        role
    );

    dok(serde_json::to_string(&composition::composition_machine::to_json_machine(proj, initial)).unwrap())

}


fn derr(errors: Vec<String>) -> String {
    serde_json::to_string(&DataResult::ERROR { errors }).unwrap()
}

fn dok(data: String) -> String {
    serde_json::to_string(&DataResult::OK { data }).unwrap()
}

fn error_report_to_strings(error_report: ErrorReport) -> Vec<String> {
    error_report
        .errors()
        .into_iter()
        .flat_map(|(g, e)| e.map(composition::composition_swarm::Error::convert(&g)))
        .collect()
}