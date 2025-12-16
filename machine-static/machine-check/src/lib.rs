use wasm_bindgen::prelude::*;

mod machine;
mod swarm;
pub mod composition;

use machine_core::types::typescript_types::{CheckResult, DataResult, MachineType, Role, Subscriptions, SubscriptionsWrapped, SwarmProtocolType};

#[wasm_bindgen]
pub fn check_swarm(proto: SwarmProtocolType, subs: SubscriptionsWrapped) -> CheckResult {
    let (graph, _, errors) = swarm::check(proto, &subs.0);
    if errors.is_empty() {
        CheckResult::OK
    } else {
        CheckResult::ERROR { errors: errors.map(machine_core::errors::swarm_errors::Error::convert(&graph)) }
    }
}

#[wasm_bindgen]
pub fn well_formed_sub(proto: SwarmProtocolType, subs: SubscriptionsWrapped) -> DataResult<Subscriptions> {
    match swarm::well_formed_sub(proto, &subs.0) {
        Ok(subscriptions) => DataResult::OK {
            data: subscriptions,
        },
        Err((graph, _, errors)) => DataResult::ERROR {
            errors: errors.map(machine_core::errors::swarm_errors::Error::convert(&graph))
        },
    }
}

#[wasm_bindgen]
pub fn check_projection(swarm: SwarmProtocolType, subs: SubscriptionsWrapped, role: Role, machine: MachineType) -> CheckResult {
    let (swarm, initial, mut errors) = swarm::from_json(swarm, &subs.0);
    let Some(initial) = initial else {
        return CheckResult::ERROR { errors: errors };
    };
    let (proj, proj_initial) = machine::project(&swarm, initial, &subs.0, role);
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
