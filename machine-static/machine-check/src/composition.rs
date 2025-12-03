use composition_swarm::{proto_info_to_error_report, swarms_to_proto_info, ErrorReport};
use composition_types::{
    Granularity, ProjectionInfo,
};

use crate::composition::composition_types::InterfacingProtocols;

use super::*;

mod composition_machine;
mod composition_swarm;
pub mod composition_types;

macro_rules! deserialize_subs {
    ($subs:expr, $err_exp:expr) => {
        match serde_json::from_str::<Subscriptions>(&$subs) {
            Ok(p) => p,
            Err(e) => return $err_exp(e),
        }
    };
}

#[wasm_bindgen]
pub fn check_composed_swarm(protos: InterfacingProtocols, subs: String) -> CheckResult {
    let subs = deserialize_subs!(subs, |e| CheckResult::ERROR {
        errors: vec![format!("parsing subscriptions: {}", e)]
    });
    let error_report = composition::composition_swarm::check(protos, &subs);
    if error_report.is_empty() {
        CheckResult::OK
    } else {
        CheckResult::ERROR {
            errors: error_report_to_strings(error_report),
        }
    }
}

#[wasm_bindgen]
pub fn exact_well_formed_sub(
    protos: InterfacingProtocols,
    subs: String,
) -> DataResult<Subscriptions> {
    let subs = deserialize_subs!(subs, |e| DataResult::ERROR {
        errors: vec![format!("parsing subscriptions: {}", e)]
    });
    let result = composition_swarm::exact_well_formed_sub(protos, &subs);
    match result {
        Ok(subscriptions) => DataResult::OK {
            data: subscriptions,
        }, //dok(subscriptions),
        Err(error_report) => DataResult::ERROR {
            errors: error_report_to_strings(error_report),
        }, //derr::<Subscriptions>(error_report_to_strings(error_report)),
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
    let result = composition_swarm::overapprox_well_formed_sub(protos, &subs, granularity);
    match result {
        Ok(subscriptions) => DataResult::OK {
            data: subscriptions,
        },
        Err(error_report) => DataResult::ERROR {
            errors: error_report_to_strings(error_report),
        },
    }
}

#[wasm_bindgen]
pub fn revised_projection(
    proto: SwarmProtocolType,
    subs: String,
    role: Role,
    minimize: bool,
) -> DataResult<MachineType> {
    let subs = deserialize_subs!(subs, |e| DataResult::ERROR {
        errors: vec![format!("parsing subscriptions: {}", e)]
    });
    let (swarm, initial, errors) = composition_swarm::from_json(proto);
    let Some(initial) = initial else {
        return DataResult::ERROR { errors };
    };
    let (proj, initial) =
        composition::composition_machine::project(&swarm, initial, &subs, role, minimize);
    DataResult::OK {
        data: composition::composition_machine::to_json_machine(proj, initial),
    }
}

#[wasm_bindgen]
pub fn project_combine(
    protos: InterfacingProtocols,
    subs: String,
    role: Role,
    minimize: bool,
) -> DataResult<MachineType> {
    let subs = deserialize_subs!(subs, |e| DataResult::ERROR {
        errors: vec![format!("parsing subscriptions: {}", e)]
    });
    let proto_info = swarms_to_proto_info(protos);
    if !proto_info.no_errors() {
        return DataResult::ERROR {
            errors: error_report_to_strings(proto_info_to_error_report(proto_info)),
        }; //derr::<Machine>(error_report_to_strings(proto_info_to_error_report(proto_info)));
    }

    let (proj, proj_initial) =
        composition_machine::project_combine(&proto_info, &subs, role, minimize);
    DataResult::OK {
        data: composition::composition_machine::from_option_to_machine(proj, proj_initial.unwrap()),
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
    let proto_info = swarms_to_proto_info(protos);
    if !proto_info.no_errors() {
        return DataResult::ERROR {
            errors: error_report_to_strings(proto_info_to_error_report(proto_info)),
        };
    }
    let (machine, initial, m_errors) = machine::from_json(machine);
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
    match composition::composition_machine::projection_information(
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
pub fn check_composed_projection(
    protos: InterfacingProtocols,
    subs: String,
    role: Role,
    machine: MachineType,
) -> CheckResult {
    let subs = deserialize_subs!(subs, |e| CheckResult::ERROR {
        errors: vec![format!("parsing subscriptions: {}", e)]
    });
    let proto_info = swarms_to_proto_info(protos);
    if !proto_info.no_errors() {
        return CheckResult::ERROR {
            errors: error_report_to_strings(proto_info_to_error_report(proto_info)),
        };
    }

    let (proj, proj_initial) =
        composition_machine::project_combine(&proto_info, &subs, role, false);
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
        composition_machine::equivalent(&proj, proj_initial.unwrap(), &machine, json_initial)
            .into_iter()
            .map(machine::Error::convert(&proj, &machine)),
    );

    if errors.is_empty() {
        CheckResult::OK
    } else {
        CheckResult::ERROR { errors }
    }
}

#[wasm_bindgen]
pub fn compose_protocols(protos: InterfacingProtocols) -> DataResult<SwarmProtocolType> {
    let composition = composition_swarm::compose_protocols(protos);

    match composition {
        Ok((graph, initial)) => DataResult::OK {
            data: composition_swarm::to_swarm_json(graph, initial),
        },
        Err(errors) => DataResult::ERROR {
            errors: error_report_to_strings(errors),
        },
    }
}

fn error_report_to_strings(error_report: ErrorReport) -> Vec<String> {
    error_report
        .errors()
        .into_iter()
        .flat_map(|(g, e)| e.map(composition::composition_swarm::Error::convert(&g)))
        .collect()
}
