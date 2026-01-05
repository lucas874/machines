use machine_core::types::typescript_types::{InterfacingProtocols, SubscriptionsWrapped};
use machine_core::types::proto_info;
use super::*;

mod composition_machine;
mod composition_swarm;

#[wasm_bindgen]
pub fn check_composed_swarm(protos: InterfacingProtocols, subs: SubscriptionsWrapped) -> CheckResult {
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
