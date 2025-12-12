use machine_types::machine::projection;
use machine_types::types::typescript_types::InterfacingProtocols;
use machine_types::{errors::composition_errors, types::proto_info};
use super::*;

mod composition_machine;
mod composition_swarm;

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
            errors: composition_errors::error_report_to_strings(error_report),
        }
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
    let proto_info = proto_info::swarms_to_proto_info(protos);
    if !proto_info.no_errors() {
        return CheckResult::ERROR {
            errors: composition_errors::error_report_to_strings(proto_info::proto_info_to_error_report(proto_info)),
        };
    }

    let (proj, proj_initial) =
        projection::project_combine(&proto_info, &subs, role, false);
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
