use composition_swarm::{swarms_to_error_report, ErrorReport};
use composition_types::{CompositionInputVec, DataResult, InterfacingSwarms};

use super::*;

mod composition_machine;
mod composition_swarm;
pub mod composition_types;

#[wasm_bindgen]
pub fn check_wwf_swarm(protos: String, subs: String) -> String {
    let protos = match serde_json::from_str::<InterfacingSwarms<Role>>(&protos) {
        Ok(p) => p,
        Err(e) => return err(vec![format!("parsing swarm protocol: {}", e)]),
    };
    let subs = match serde_json::from_str::<Subscriptions>(&subs) {
        Ok(p) => p,
        Err(e) => return err(vec![format!("parsing subscriptions: {}", e)]),
    };

    let error_report = composition::composition_swarm::check(protos, &subs);
    if error_report.is_empty() {
        serde_json::to_string(&CheckResult::OK).unwrap()
    } else {
        err(error_report_to_strings(error_report))
    }
}

#[wasm_bindgen]
pub fn exact_weak_well_formed_sub(protos: String) -> String {
    let protos = match serde_json::from_str::<InterfacingSwarms<Role>>(&protos) {
        Ok(p) => p,
        Err(e) => return derr(vec![format!("parsing swarm protocol: {}", e)]),
    };
    /* let (subscriptions, error_report) = composition::composition_swarm::weak_well_formed_sub(proto);
    if error_report.is_empty() {
        dok(serde_json::to_string(&subscriptions).unwrap())
    } else {
        derr(error_report_to_strings(error_report))
    } */
    unimplemented!()
}

/* #[wasm_bindgen]
pub fn get_wwf_sub(proto: String) -> String {
    let proto = match serde_json::from_str::<SwarmProtocol>(&proto) {
        Ok(p) => p,
        Err(e) => return derr(vec![format!("parsing swarm protocol: {}", e)]),
    };
    let (subscriptions, error_report) = composition::composition_swarm::weak_well_formed_sub(proto);
    if error_report.is_empty() {
        dok(serde_json::to_string(&subscriptions).unwrap())
    } else {
        derr(error_report_to_strings(error_report))
    }

}

#[wasm_bindgen]
pub fn compose_subs(input: String) -> String {
    let protocols = match serde_json::from_str::<CompositionInputVec>(&input) {
        Ok(p) => p,
        Err(e) => return derr(vec![format!("parsing composition input: {}", e)]),
    };

    let (subscriptions, error_report) = composition_swarm::compose_subscriptions(protocols);
    if error_report.is_empty() {
        dok(serde_json::to_string(&subscriptions).unwrap())
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
    let (proj, initial) = composition::composition_machine::project(&swarm, initial, &subs, role);

    dok(
        serde_json::to_string(&composition::composition_machine::to_json_machine(
            proj, initial,
        ))
        .unwrap(),
    )
}

#[wasm_bindgen]
pub fn project_combine(input: String, role: String) -> String {
    let protocols = match serde_json::from_str::<CompositionInputVec>(&input) {
        Ok(p) => p,
        Err(e) => return derr(vec![format!("parsing composition input: {}", e)]),
    };
    let role = Role::new(&role);

    let (composed_protos, subs) = composition_swarm::implicit_composition_swarms(protocols);

    // does not look nice? reconsider
    let swarms = match composed_protos
        .iter()
        .any(|((_, i, e), _)| i.is_none() || !e.is_empty())
    {
        false => composed_protos
            .into_iter()
            .map(|((g, i, _), s)| (g, i.unwrap(), s))
            .collect(),
        true => {
            return derr(error_report_to_strings(swarms_to_error_report(
                composed_protos,
            )))
        }
    };

    let (proj, proj_initial) = composition_machine::project_combine(&swarms, &subs, role);

    dok(
        serde_json::to_string(&composition::composition_machine::from_option_to_machine(
            proj,
            proj_initial.unwrap(),
        ))
        .unwrap(),
    )
}

#[wasm_bindgen]
pub fn project_combine_all(input: String) -> String {
    let protocols = match serde_json::from_str::<CompositionInputVec>(&input) {
        Ok(p) => p,
        Err(e) => return derr(vec![format!("parsing composition input: {}", e)]),
    };

    let (composed_protos, subs) = composition_swarm::implicit_composition_swarms(protocols);

    // does not look nice? reconsider
    let swarms = match composed_protos
        .iter()
        .any(|((_, i, e), _)| i.is_none() || !e.is_empty())
    {
        false => composed_protos
            .into_iter()
            .map(|((g, i, _), s)| (g, i.unwrap(), s))
            .collect(),
        true => {
            return derr(error_report_to_strings(swarms_to_error_report(
                composed_protos,
            )))
        }
    };

    let projections = composition_machine::project_combine_all(&swarms, &subs);
    // do not think we need this check here
    if projections.iter().any(|(_, i)| i.is_none()) {
        return derr(vec![]);
    }
    let machines: Vec<_> = projections
        .into_iter()
        .map(|(g, i)| {
            serde_json::to_string(&composition::composition_machine::from_option_to_machine(
                g,
                i.unwrap(),
            ))
            .unwrap()
        })
        .collect();
    dok(format!("[{}]", machines.join(", ")))
}

// take a sub instead of inferring one or combining the ones in the input vec...
#[wasm_bindgen]
pub fn check_composed_projection(
    swarms: String,
    subs: String,
    role: String,
    machine: String,
) -> String {
    let protocols = match serde_json::from_str::<CompositionInputVec>(&swarms) {
        Ok(p) => p,
        Err(e) => return derr(vec![format!("parsing composition input: {}", e)]),
    };
    let subs = match serde_json::from_str::<Subscriptions>(&subs) {
        Ok(p) => p,
        Err(e) => return err(vec![format!("parsing subscriptions: {}", e)]),
    };
    let role = Role::new(&role);
    let machine = match serde_json::from_str::<Machine>(&machine) {
        Ok(p) => p,
        Err(e) => return err(vec![format!("parsing machine: {}", e)]),
    };
    let (composed_protos, _) = composition_swarm::implicit_composition_swarms(protocols);

    let swarms = match composed_protos
        .iter()
        .any(|((_, i, e), _)| i.is_none() || !e.is_empty())
    {
        false => composed_protos
            .into_iter()
            .map(|((g, i, _), s)| (g, i.unwrap(), s))
            .collect(),
        true => {
            return derr(error_report_to_strings(swarms_to_error_report(
                composed_protos,
            )))
        }
    };

    let (proj, proj_initial) = composition_machine::project_combine(&swarms, &subs, role);
    let (machine, json_initial, m_errors) = machine::from_json(machine);
    let machine_problem = !m_errors.is_empty();
    let mut errors = vec![];
    errors.extend(m_errors);
    let Some(json_initial) = json_initial else {
        errors.push(format!("initial machine state has no transitions"));
        return err(errors);
    };
    if machine_problem {
        return err(errors);
    }

    errors.extend(
        machine::equivalent(&proj, proj_initial.unwrap(), &machine, json_initial)
            .into_iter()
            .map(machine::Error::convert(&proj, &machine)),
    );

    if errors.is_empty() {
        serde_json::to_string(&CheckResult::OK).unwrap()
    } else {
        err(errors)
    }
}

#[wasm_bindgen]
pub fn compose_protocols(protos: String) -> String {
    let protocols = match serde_json::from_str::<CompositionInputVec>(&protos) {
        Ok(p) => p,
        Err(e) => return derr(vec![format!("parsing composition input: {}", e)]),
    };
    let composition = composition_swarm::compose_protocols(protocols);

    match composition {
        Ok((graph, initial)) => {
            dok(serde_json::to_string(&composition_swarm::to_swarm_json(graph, initial)).unwrap())
        }
        Err(errors) => derr(error_report_to_strings(errors)),
    }
} */

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
