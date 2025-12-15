use wasm_bindgen::prelude::*;

use crate::machine::util::to_json_machine;
use crate::machine::{adaptation, projection};
use crate::types::{proto_info, typescript_types};
use crate::types::typescript_types::{DataResult, Granularity, InterfacingProtocols, MachineType, ProjectionInfo, Role, Subscriptions, SwarmProtocolType};

pub mod types;
pub mod errors;
pub mod composability_check;
pub mod subscription;
pub mod composition;
pub mod machine;
mod util;

#[cfg(test)]
mod test_utils;

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

#[wasm_bindgen]
pub fn project(
    protos: InterfacingProtocols,
    subs: String,
    role: Role,
    minimize: bool,
    expand_protos: bool,
) -> DataResult<MachineType> {
    let subs = deserialize_subs!(subs, |e| DataResult::ERROR {
        errors: vec![format!("parsing subscriptions: {}", e)]
    });

    // Expand the protocol composition of expand_protos, otherwise project each protocol and compose machines.
    let machine = if expand_protos {
        match proto_info::compose_protocols(protos) {
            Ok((swarm, initial)) => {
                let (proj, proj_initial) = projection::project(&swarm, initial, &subs, role, minimize);
                to_json_machine(proj, proj_initial)
            },
            Err(error_report) => return DataResult::ERROR {
                errors: composition_errors::error_report_to_strings(error_report),
            }
        }
    } else {
        let proto_info = proto_info::swarms_to_proto_info(protos);
        match proto_info.no_errors() {
            true => {
                let (proj, proj_initial) = projection::project_combine(&proto_info, &subs, role, minimize);
                machine::util::from_option_to_machine(proj, proj_initial.unwrap())
            },
            false => return DataResult::ERROR {
                errors: composition_errors::error_report_to_strings(proto_info::proto_info_to_error_report(proto_info)) }
        }
    };
    DataResult::OK {
        data: machine
    }
}

#[wasm_bindgen]
pub fn projection_information(
    role: Role,
    protos: InterfacingProtocols,
    k: usize,
    subs: String,
    machine: MachineType,
    minimize: bool,
) -> DataResult<ProjectionInfo> {
    let subs = deserialize_subs!(subs, |e| DataResult::ERROR {
        errors: vec![format!("parsing subscriptions: {}", e)]
    });
    let proto_info = proto_info::swarms_to_proto_info(protos);
    if !proto_info.no_errors() {
        return DataResult::ERROR {
            errors: composition_errors::error_report_to_strings(proto_info::proto_info_to_error_report(proto_info)),
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
        &subs,
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
            data: typescript_types::to_swarm_json(graph, initial),
        },
        Err(errors) => DataResult::ERROR {
            errors: composition_errors::error_report_to_strings(errors),
        },
    }
}

/*
#[wasm_bindgen]
pub fn compose_protocols(protos: InterfacingProtocols) -> Result<SwarmProtocolType, Vec<String>> {
    let composition = proto_info::compose_protocols(protos);

    match composition {
        Ok((graph, initial)) => Ok (
            typescript_types::to_swarm_json(graph, initial),
        ),
        Err(errors) => Err(
            composition_errors::error_report_to_strings(errors),
        ),
    }
}

*/