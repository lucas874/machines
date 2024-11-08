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