use wasm_bindgen::prelude::*;

use crate::machine::util::to_json_machine;
use crate::machine::{adaptation, projection};
use crate::types::typescript_types::{
    DataResult, Granularity, InterfacingProtocols, MachineType, ProjectionInfo, Role,
    Subscriptions, SubscriptionsWrapped, SwarmProtocolType,
};
use crate::types::{proto_info, typescript_types};

mod composability_check;
mod composition;
pub mod errors;
mod machine;
mod subscription;
pub mod types;

#[cfg(test)]
mod test_utils;

use crate::subscription::{exact, overapproximation};

#[wasm_bindgen]
pub fn exact_well_formed_sub(
    protos: InterfacingProtocols,
    subs: SubscriptionsWrapped,
) -> DataResult<Subscriptions> {
    let result = exact::exact_well_formed_sub(protos, &subs.0);
    match result {
        Ok(subscriptions) => DataResult::OK {
            data: subscriptions,
        },
        Err(error_report) => DataResult::ERROR {
            errors: error_report.to_strings(),
        },
    }
}

#[wasm_bindgen]
pub fn overapproximated_well_formed_sub(
    protos: InterfacingProtocols,
    subs: SubscriptionsWrapped,
    granularity: Granularity,
) -> DataResult<Subscriptions> {
    let result = overapproximation::overapprox_well_formed_sub(protos, &subs.0, granularity);
    match result {
        Ok(subscriptions) => DataResult::OK {
            data: subscriptions,
        },
        Err(error_report) => DataResult::ERROR {
            errors: error_report.to_strings(),
        },
    }
}

#[wasm_bindgen]
pub fn project(
    protos: InterfacingProtocols,
    subs: SubscriptionsWrapped,
    role: Role,
    minimize: bool,
    expand_protos: bool,
) -> DataResult<MachineType> {
    // Expand the protocol composition if expand_protos, otherwise project each protocol and compose machines.
    let machine = if expand_protos {
        match proto_info::compose_protocols(protos) {
            Ok((swarm, initial)) => {
                let (proj, proj_initial) =
                    projection::project(&swarm, initial, &subs.0, role, minimize);
                to_json_machine(proj, proj_initial)
            }
            Err(error_report) => {
                return DataResult::ERROR {
                    errors: error_report.to_strings(),
                };
            }
        }
    } else {
        let proto_info = proto_info::swarms_to_proto_info(protos);
        match proto_info.no_errors() {
            true => {
                let (proj, proj_initial) =
                    projection::project_combine(&proto_info, &subs.0, role, minimize);
                machine::util::option_to_json_machine(proj, proj_initial.unwrap())
            }
            false => {
                return DataResult::ERROR {
                    errors: proto_info.to_error_report().to_strings(),
                };
            }
        }
    };
    DataResult::OK { data: machine }
}

#[wasm_bindgen]
pub fn projection_information(
    role: Role,
    protos: InterfacingProtocols,
    k: usize,
    subs: SubscriptionsWrapped,
    machine: MachineType,
    minimize: bool,
) -> DataResult<ProjectionInfo> {
    let proto_info = proto_info::swarms_to_proto_info(protos);
    if !proto_info.no_errors() {
        return DataResult::ERROR {
            errors: proto_info.to_error_report().to_strings(),
        };
    }
    let (machine, initial, m_errors) = machine::util::from_json(machine);
    let machine_problem = !m_errors.is_empty();
    let mut errors = vec![];
    errors.extend(m_errors);
    let Some(initial) = initial else {
        errors.push(format!("initial machine state has no transitions"));
        return DataResult::ERROR { errors };
    };
    if machine_problem {
        return DataResult::ERROR { errors };
    }
    match adaptation::projection_information(
        &proto_info,
        &subs.0,
        role,
        (machine, initial),
        k,
        minimize,
    ) {
        Some(projection_info) => DataResult::OK {
            data: projection_info,
        },
        None => DataResult::ERROR {
            errors: vec![format!("invalid index {}", k)],
        },
    }
}

#[wasm_bindgen]
pub fn compose_protocols(protos: InterfacingProtocols) -> DataResult<SwarmProtocolType> {
    let composition = proto_info::compose_protocols(protos);

    match composition {
        Ok((graph, initial)) => DataResult::OK {
            data: typescript_types::to_json_swarm(graph, initial),
        },
        Err(errors) => DataResult::ERROR {
            errors: errors.to_strings(),
        },
    }
}
