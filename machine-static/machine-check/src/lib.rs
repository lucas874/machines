use serde::Serialize;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

mod composition;
mod machine;
mod swarm;

use machine_core::types::proto_info;
use machine_core::types::typescript_types::InterfacingProtocols;
use machine_core::types::typescript_types::{
    DataResult, MachineType, Role, Subscriptions, SubscriptionsWrapped, SwarmProtocolType,
};

#[derive(Tsify, Serialize)]
#[serde(tag = "type")]
#[tsify(into_wasm_abi)]
pub enum CheckResult {
    OK,
    ERROR { errors: Vec<String> },
}

#[wasm_bindgen]
pub fn check_swarm(proto: SwarmProtocolType, subs: SubscriptionsWrapped) -> CheckResult {
    let (graph, _, errors) = swarm::check(proto, &subs.0);
    if errors.is_empty() {
        CheckResult::OK
    } else {
        CheckResult::ERROR {
            errors: errors.map(machine_core::errors::Error::convert(&graph)),
        }
    }
}

#[wasm_bindgen]
pub fn well_formed_sub(
    proto: SwarmProtocolType,
    subs: SubscriptionsWrapped,
) -> DataResult<Subscriptions> {
    match swarm::well_formed_sub(proto, &subs.0) {
        Ok(subscriptions) => DataResult::OK {
            data: subscriptions,
        },
        Err((graph, _, errors)) => DataResult::ERROR {
            errors: errors.map(machine_core::errors::Error::convert(&graph)),
        },
    }
}

#[wasm_bindgen]
pub fn check_projection(
    swarm: SwarmProtocolType,
    subs: SubscriptionsWrapped,
    role: Role,
    machine: MachineType,
) -> CheckResult {
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

#[wasm_bindgen]
pub fn check_composed_swarm(
    protos: InterfacingProtocols,
    subs: SubscriptionsWrapped,
) -> CheckResult {
    let error_report = composition::composition_swarm::check(protos, &subs.0);
    if error_report.is_empty() {
        CheckResult::OK
    } else {
        CheckResult::ERROR {
            errors: error_report.to_strings(),
        }
    }
}

#[wasm_bindgen]
pub fn check_composed_projection(
    protos: InterfacingProtocols,
    subs: SubscriptionsWrapped,
    role: Role,
    machine: MachineType,
) -> CheckResult {
    let proto_info = proto_info::swarms_to_proto_info(protos.clone());
    if !proto_info.no_errors() {
        return CheckResult::ERROR {
            errors: proto_info.to_error_report().to_strings(),
        };
    }
    let proj_machine = match machine_core::project(protos, subs, role.clone(), false, false) {
        DataResult::OK { data } => data,
        DataResult::ERROR { errors } => return CheckResult::ERROR { errors },
    };
    let (proj, proj_initial, _) = machine::from_json(proj_machine);
    let (machine, json_initial, m_errors) = machine::from_json(machine);
    let machine_problem = !m_errors.is_empty();
    let mut errors = vec![];
    errors.extend(m_errors);
    let Some(json_initial) = json_initial else {
        errors.push(format!("initial machine state has no transitions"));
        return CheckResult::ERROR { errors };
    };
    if machine_problem {
        return CheckResult::ERROR { errors };
    }

    errors.extend(
        composition::composition_machine::equivalent(&proj, proj_initial.unwrap(), &machine, json_initial)
            .into_iter()
            .map(machine::Error::convert(&proj, &machine)),
    );

    if errors.is_empty() {
        CheckResult::OK
    } else {
        CheckResult::ERROR { errors }
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
