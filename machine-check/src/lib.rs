use std::collections::{BTreeMap, BTreeSet};
use wasm_bindgen::prelude::*;
use tsify::{declare};

mod machine;
mod swarm;
pub mod types;
pub mod composition;

use petgraph::visit::GraphBase;
use types::{CheckResult, EventType, MachineLabel, ProtocolType, Role, State, SwarmLabel};

use crate::types::DataResult;

#[declare]
pub type Subscriptions = BTreeMap<Role, BTreeSet<EventType>>;
#[declare]
pub type SwarmProtocolType = ProtocolType<SwarmLabel>;
#[declare]
pub type MachineType = ProtocolType<MachineLabel>;

pub type Graph = petgraph::Graph<State, SwarmLabel>;
pub type NodeId = <petgraph::Graph<(), ()> as GraphBase>::NodeId;
pub type EdgeId = <petgraph::Graph<(), ()> as GraphBase>::EdgeId;

#[wasm_bindgen]
pub fn check_swarm(proto: SwarmProtocolType, subs: String) -> CheckResult {
    let subs = match serde_json::from_str::<Subscriptions>(&subs) {
        Ok(p) => p,
        Err(e) => return CheckResult::ERROR { errors: vec![format!("parsing subscriptions: {}", e)] },
    };
    let (graph, _, errors) = swarm::check(proto, &subs);
    if errors.is_empty() {
        CheckResult::OK
    } else {
        CheckResult::ERROR { errors: errors.map(swarm::Error::convert(&graph)) }
    }
}

#[wasm_bindgen]
pub fn well_formed_sub(proto: SwarmProtocolType, subs: String) -> DataResult<Subscriptions> {
    let subs = match serde_json::from_str::<Subscriptions>(&subs) {
        Ok(p) => p,
        Err(e) => return DataResult::ERROR { errors: vec![format!("parsing subscriptions: {}", e)] },
    };
    match swarm::well_formed_sub(proto, &subs) {
        Ok(subscriptions) => DataResult::OK {
            data: subscriptions,
        },
        Err((graph, _, errors)) => DataResult::ERROR {
            errors: errors.map(swarm::Error::convert(&graph))
        },
    }
}

#[wasm_bindgen]
pub fn check_projection(swarm: SwarmProtocolType, subs: String, role: Role, machine: MachineType) -> CheckResult {
    let subs = match serde_json::from_str::<Subscriptions>(&subs) {
        Ok(p) => p,
        Err(e) => return CheckResult::ERROR { errors: vec![format!("parsing subscriptions: {}", e)] },
    };

    let (swarm, initial, mut errors) = swarm::from_json(swarm, &subs);
    let Some(initial) = initial else {
        return CheckResult::ERROR { errors: errors };
    };
    let (proj, proj_initial) = machine::project(&swarm, initial, &subs, role);
    let (machine, json_initial, m_errors) = machine::from_json(machine);
    let machine_problem = !m_errors.is_empty();
    errors.extend(m_errors);
    let Some(json_initial) = json_initial else {
        errors.push(format!("initial machine state has no transitions"));
        return CheckResult::ERROR { errors: errors };
    };
    if machine_problem {
        return CheckResult::ERROR { errors: errors };
    }

    errors.extend(
        machine::equivalent(&proj, proj_initial, &machine, json_initial)
            .into_iter()
            .map(machine::Error::convert(&proj, &machine)),
    );

    if errors.is_empty() {
        CheckResult::OK
    } else {
        CheckResult::ERROR { errors: errors }
    }
}

trait MapVec<T> {
    fn map<U>(self, f: impl Fn(T) -> U) -> Vec<U>;
}
impl<T> MapVec<T> for Vec<T> {
    fn map<U>(self, f: impl Fn(T) -> U) -> Vec<U> {
        self.into_iter().map(f).collect()
    }
}
