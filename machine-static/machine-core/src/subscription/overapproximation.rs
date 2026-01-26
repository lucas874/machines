use std::collections::BTreeSet;

use crate::{
    errors::ErrorReport,
    types::{
        proto_info::ProtoInfo,
        typescript_types::{
            EventLabel, EventType, Granularity, InterfacingProtocols, Role, Subscriptions,
        },
    },
};
use crate::types::proto_info;

// Construct wf-subscription compositionally.
// Overapproximates the subscription one would obtain from exact_well_formed_sub().
pub fn overapprox_well_formed_sub(
    protos: InterfacingProtocols,
    subs: &Subscriptions,
    granularity: Granularity,
) -> Result<Subscriptions, ErrorReport> {
    let _span = tracing::info_span!("overapprox_well_formed_sub").entered();
    let combined_proto_info = proto_info::swarms_to_proto_info(protos);
    if !combined_proto_info.no_errors() {
        return Err(combined_proto_info.to_error_report());
    }

    // If we reach this point the protocols can interface and are all confusion free.
    // We construct a ProtoInfo with the composition as the only protocol and all the
    // information about branches etc. from combined_proto_info
    let sub = overapprox_wf_sub(&mut combined_proto_info.clone(), subs, granularity);
    Ok(sub)
}

fn overapprox_wf_sub(
    proto_info: &mut ProtoInfo,
    subscription: &Subscriptions,
    granularity: Granularity,
) -> Subscriptions {
    let _span = tracing::info_span!("overapprox_wf_sub").entered();
    match granularity {
        Granularity::Fine => finer_overapprox_wf_sub(proto_info, subscription, false),
        Granularity::Coarse => finer_overapprox_wf_sub(proto_info, subscription, true),
        Granularity::TwoStep => two_step_overapprox_wf_sub(proto_info, &mut subscription.clone()),
    }
}

fn finer_overapprox_wf_sub(
    proto_info: &mut ProtoInfo,
    subscription: &Subscriptions,
    with_all_interfacing: bool,
) -> Subscriptions {
    let _span = tracing::info_span!("finer_overapprox_wf_sub").entered();
    let mut subscription = subscription.clone();
    proto_info.succeeding_events =
        proto_info::transitive_closure_succeeding(proto_info.succeeding_events.clone());

    // Causal consistency
    for (role, labels) in &proto_info.role_event_map {
        let event_types: BTreeSet<_> = labels.iter().map(|label| label.get_event_type()).collect();
        let preceding_event_types: BTreeSet<_> = event_types
            .iter()
            .flat_map(|e| proto_info.get_preceding(e))
            .collect();
        let mut events_to_add = event_types
            .into_iter()
            .chain(preceding_event_types.into_iter())
            .collect();
        subscription
            .entry(role.clone())
            .and_modify(|set| {
                set.append(&mut events_to_add);
            })
            .or_insert_with(|| events_to_add);
    }

    // Add all interfacing -- 'Medium granularity'.
    if with_all_interfacing {
        for sub in subscription.values_mut() {
            sub.append(&mut proto_info.interfacing_events.clone());
        }
    }

    // Determinacy
    finer_approx_add_branches_and_joins(proto_info, &mut subscription);

    // Add looping event types to the subscription.
    super::add_looping_event_types(proto_info, &mut subscription);

    subscription
}

fn finer_approx_add_branches_and_joins(proto_info: &ProtoInfo, subscription: &mut Subscriptions) {
    let _span = tracing::info_span!("finer_approx_add_branches_and_joins").entered();
    let mut is_stable = false;

    while !is_stable {
        is_stable = true;

        // Determinacy: joins
        for (joining_event, pre_joining_event) in &proto_info.joining_events {
            let interested_roles =
                proto_info::roles_on_path(joining_event.clone(), proto_info, &subscription);
            let join_and_prejoin: BTreeSet<EventType> = [joining_event.clone()]
                .into_iter()
                .chain(pre_joining_event.clone().into_iter())
                .collect();
            for role in interested_roles {
                is_stable = add_to_sub(role, join_and_prejoin.clone(), subscription) && is_stable;
            }
        }

        // Determinacy: branches
        for branching_events in &proto_info.branching_events {
            let interested_roles = branching_events
                .iter()
                .flat_map(|e| proto_info::roles_on_path(e.clone(), proto_info, &subscription))
                .collect::<BTreeSet<_>>();
            for role in interested_roles {
                is_stable = add_to_sub(role, branching_events.clone(), subscription) && is_stable;
            }
        }
    }
}

// Safe, overapproximating subscription generation as described in paper (Algorithm 1).
fn two_step_overapprox_wf_sub(
    proto_info: &ProtoInfo,
    subscription: &mut Subscriptions,
) -> Subscriptions {
    let _span = tracing::info_span!("two_step_overapprox_wf_sub").entered();
    // Causal consistency
    for (role, labels) in &proto_info.role_event_map {
        let event_types: BTreeSet<_> = labels.iter().map(|label| label.get_event_type()).collect();
        let preceding_event_types: BTreeSet<_> = event_types
            .iter()
            .flat_map(|e| proto_info.get_preceding(e))
            .collect();
        let mut events_to_add = event_types
            .into_iter()
            .chain(preceding_event_types.into_iter())
            .collect();
        subscription
            .entry(role.clone())
            .and_modify(|set| {
                set.append(&mut events_to_add);
            })
            .or_insert_with(|| events_to_add);
    }

    let mut is_stable = false;
    while !is_stable {
        is_stable = true;
        // Determinacy: branches
        for branching_events in &proto_info.branching_events {
            let interested_roles = branching_events
                .iter()
                .flat_map(|e| proto_info::roles_on_path(e.clone(), proto_info, &subscription))
                .collect::<BTreeSet<_>>();
            for role in interested_roles {
                is_stable = add_to_sub(role, branching_events.clone(), subscription) && is_stable;
            }
        }

        // Determinacy: joins.
        for (joining_event, pre_joining_event) in &proto_info.joining_events {
            let interested_roles =
                proto_info::roles_on_path(joining_event.clone(), proto_info, &subscription);
            let join_and_prejoin: BTreeSet<EventType> = [joining_event.clone()]
                .into_iter()
                .chain(pre_joining_event.clone().into_iter())
                .collect();
            for role in interested_roles {
                is_stable = add_to_sub(role, join_and_prejoin.clone(), subscription) && is_stable;
            }
        }

        // Interfacing rule from algorithm in paper
        for interfacing_event in &proto_info.interfacing_events {
            let interested_roles =
                proto_info::roles_on_path(interfacing_event.clone(), proto_info, &subscription);
            for role in interested_roles {
                is_stable = add_to_sub(
                    role,
                    BTreeSet::from([interfacing_event.clone()]),
                    subscription,
                ) && is_stable;
            }
        }
    }

    // Add looping event types to the subscription.
    super::add_looping_event_types(proto_info, subscription);

    subscription.clone()
}

// Add events to a subscription, return true of they were already in the subscription and false otherwise
// Mutates subs.
fn add_to_sub(role: Role, mut event_types: BTreeSet<EventType>, subs: &mut Subscriptions) -> bool {
    if subs.contains_key(&role) && event_types.iter().all(|e| subs[&role].contains(e)) {
        return true;
    }
    subs.entry(role)
        .and_modify(|curr| {
            curr.append(&mut event_types);
        })
        .or_insert(event_types);
    false
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::test_utils;

    #[test]
    fn test_well_formed_sub() {
        test_utils::setup_logger();

        // Test interfacing_swarms_1
        // Coarse (this granularity used to be called medium)
        let result_1_coarse = overapprox_well_formed_sub(
            test_utils::get_interfacing_swarms_1(),
            &BTreeMap::new(),
            Granularity::Coarse,
        );
        assert!(result_1_coarse.is_ok());
        let subs_1_coarse = result_1_coarse.unwrap();
        let expected_subs_1_coarse: Subscriptions = BTreeMap::from([
            (
                Role::from("T"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("pos"),
                    EventType::from("part"),
                    EventType::from("time"),
                ]),
            ),
            (
                Role::from("FL"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("pos"),
                    EventType::from("part"),
                    EventType::from("time"),
                ]),
            ),
            (
                Role::from("D"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                ]),
            ),
            (
                Role::from("F"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                ]),
            ),
        ]);
        assert_eq!(subs_1_coarse, expected_subs_1_coarse);

        // Fine. Should be equal to exact for this example.
        let result_1_fine = overapprox_well_formed_sub(
            test_utils::get_interfacing_swarms_1(),
            &BTreeMap::new(),
            Granularity::Fine,
        );
        assert!(result_1_fine.is_ok());
        let subs_1_fine = result_1_fine.unwrap();
        let expected_subs_1_fine: Subscriptions = BTreeMap::from([
            (
                Role::from("T"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("pos"),
                    EventType::from("part"),
                    EventType::from("time"),
                ]),
            ),
            (
                Role::from("FL"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("pos"),
                    EventType::from("time"),
                ]),
            ),
            (
                Role::from("D"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                ]),
            ),
            (
                Role::from("F"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                ]),
            ),
        ]);
        assert_eq!(subs_1_fine, expected_subs_1_fine);

        // 'Algorithm 1'/'Two Step'
        let result_1_two_step = overapprox_well_formed_sub(
            test_utils::get_interfacing_swarms_1(),
            &BTreeMap::new(),
            Granularity::TwoStep,
        );
        assert!(result_1_two_step.is_ok());
        let subs_1_two_step = result_1_two_step.unwrap();
        let expected_subs_1_two_step: Subscriptions = BTreeMap::from([
            (
                Role::from("T"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("pos"),
                    EventType::from("part"),
                    EventType::from("time"),
                ]),
            ),
            (
                Role::from("FL"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("pos"),
                    EventType::from("part"),
                    EventType::from("time"),
                ]),
            ),
            (
                Role::from("D"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                ]),
            ),
            (
                Role::from("F"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                ]),
            ),
        ]);
        assert_eq!(subs_1_two_step, expected_subs_1_two_step);
        assert_eq!(subs_1_two_step, subs_1_coarse);

        // Test interfacing_swarms_2
        // Coarse (this granularity used to be called medium)
        let result_2_coarse = overapprox_well_formed_sub(
            test_utils::get_interfacing_swarms_2(),
            &BTreeMap::new(),
            Granularity::Coarse,
        );
        assert!(result_2_coarse.is_ok());
        let subs_2_coarse = result_2_coarse.unwrap();
        let expected_subs_2_coarse: Subscriptions = BTreeMap::from([
            (
                Role::from("T"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("pos"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                    EventType::from("report1"),
                ]),
            ),
            (
                Role::from("FL"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("pos"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                    EventType::from("report1"),
                ]),
            ),
            (
                Role::from("D"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                    EventType::from("report1"),
                ]),
            ),
            (
                Role::from("F"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                    EventType::from("report1"),
                ]),
            ),
            (
                Role::from("TR"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                    EventType::from("report1"),
                    EventType::from("report2"),
                ]),
            ),
            (
                Role::from("QCR"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                    EventType::from("report1"),
                    EventType::from("report2"),
                    EventType::from("ok"),
                    EventType::from("notOk"),
                ]),
            ),
        ]);
        assert_eq!(subs_2_coarse, expected_subs_2_coarse);

        // Fine
        let result_2_fine = overapprox_well_formed_sub(
            test_utils::get_interfacing_swarms_2(),
            &BTreeMap::new(),
            Granularity::Fine,
        );
        assert!(result_2_fine.is_ok());
        let subs_2_fine = result_2_fine.unwrap();
        let expected_subs_2_fine: Subscriptions = BTreeMap::from([
            (
                Role::from("T"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("pos"),
                    EventType::from("part"),
                    EventType::from("time"),
                ]),
            ),
            (
                Role::from("FL"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("pos"),
                    EventType::from("time"),
                ]),
            ),
            (
                Role::from("D"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                ]),
            ),
            (
                Role::from("F"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                    EventType::from("report1"),
                ]),
            ),
            (
                Role::from("TR"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                    EventType::from("report1"),
                    EventType::from("report2"),
                ]),
            ),
            (
                Role::from("QCR"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                    EventType::from("report1"),
                    EventType::from("report2"),
                    EventType::from("ok"),
                    EventType::from("notOk"),
                ]),
            ),
        ]);
        assert_eq!(subs_2_fine, expected_subs_2_fine);

        // 'Algorithm 1'/'Two Step'
        let result_2_two_step = overapprox_well_formed_sub(
            test_utils::get_interfacing_swarms_2(),
            &BTreeMap::new(),
            Granularity::TwoStep,
        );
        assert!(result_2_two_step.is_ok());
        let subs_2_two_step = result_2_two_step.unwrap();
        let expected_subs_2_two_step: Subscriptions = BTreeMap::from([
            (
                Role::from("T"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("pos"),
                    EventType::from("part"),
                    EventType::from("time"),
                ]),
            ),
            (
                Role::from("FL"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("pos"),
                    EventType::from("part"),
                    EventType::from("time"),
                ]),
            ),
            (
                Role::from("D"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                ]),
            ),
            (
                Role::from("F"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                    EventType::from("report1"),
                ]),
            ),
            (
                Role::from("TR"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                    EventType::from("report1"),
                    EventType::from("report2"),
                ]),
            ),
            (
                Role::from("QCR"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                    EventType::from("report1"),
                    EventType::from("report2"),
                    EventType::from("ok"),
                    EventType::from("notOk"),
                ]),
            ),
        ]);
        assert_eq!(subs_2_two_step, expected_subs_2_two_step);

        // Test interfacing_swarms_3
        // Coarse
        let result_3_coarse = overapprox_well_formed_sub(
            test_utils::get_interfacing_swarms_3(),
            &BTreeMap::new(),
            Granularity::Coarse,
        );
        assert!(result_3_coarse.is_ok());
        let subs_3_coarse = result_3_coarse.unwrap();
        let expected_subs_3_coarse: Subscriptions = BTreeMap::from([
            (
                Role::from("T"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("pos"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                    EventType::from("report2"),
                ]),
            ),
            (
                Role::from("FL"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("pos"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                    EventType::from("report2"),
                ]),
            ),
            (
                Role::from("D"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                    EventType::from("report2"),
                ]),
            ),
            (
                Role::from("F"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                    EventType::from("report2"),
                ]),
            ),
            (
                Role::from("QCR"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                    EventType::from("report1"),
                    EventType::from("report2"),
                    EventType::from("report3"),
                ]),
            ),
        ]);
        assert_eq!(subs_3_coarse, expected_subs_3_coarse);

        // Fine
        let result_3_fine = overapprox_well_formed_sub(
            test_utils::get_interfacing_swarms_3(),
            &BTreeMap::new(),
            Granularity::Fine,
        );
        assert!(result_3_fine.is_ok());
        let subs_3_fine = result_3_fine.unwrap();
        let expected_subs_3_fine: Subscriptions = BTreeMap::from([
            (
                Role::from("T"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("pos"),
                    EventType::from("part"),
                    EventType::from("time"),
                ]),
            ),
            (
                Role::from("FL"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("pos"),
                    EventType::from("time"),
                ]),
            ),
            (
                Role::from("D"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                ]),
            ),
            (
                Role::from("F"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                    EventType::from("report2"),
                ]),
            ),
            (
                Role::from("QCR"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                    EventType::from("report1"),
                    EventType::from("report2"),
                    EventType::from("report3"),
                ]),
            ),
        ]);
        assert_eq!(subs_3_fine, expected_subs_3_fine);

        // 'Algorithm 1'/'Two Step'
        let result_3_two_step = overapprox_well_formed_sub(
            test_utils::get_interfacing_swarms_3(),
            &BTreeMap::new(),
            Granularity::TwoStep,
        );
        assert!(result_3_two_step.is_ok());
        let subs_3_two_step = result_3_two_step.unwrap();
        let expected_subs_3_two_step: Subscriptions = BTreeMap::from([
            (
                Role::from("T"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("pos"),
                    EventType::from("part"),
                    EventType::from("time"),
                ]),
            ),
            (
                Role::from("FL"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("pos"),
                    EventType::from("part"),
                    EventType::from("time"),
                ]),
            ),
            (
                Role::from("D"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                ]),
            ),
            (
                Role::from("F"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                    EventType::from("report2"),
                ]),
            ),
            (
                Role::from("QCR"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                    EventType::from("report1"),
                    EventType::from("report2"),
                    EventType::from("report3"),
                ]),
            ),
        ]);
        assert_eq!(subs_3_two_step, expected_subs_3_two_step);
    }

    #[test]
    fn test_well_formed_sub_1() {
        test_utils::setup_logger();

        // Test empty set if input protocols
        let empty_coarse = overapprox_well_formed_sub(
            InterfacingProtocols(vec![]),
            &BTreeMap::new(),
            Granularity::Coarse,
        );
        assert!(empty_coarse.is_ok());
        assert_eq!(empty_coarse.unwrap(), BTreeMap::new());

        let empty_fine = overapprox_well_formed_sub(
            InterfacingProtocols(vec![]),
            &BTreeMap::new(),
            Granularity::Fine,
        );
        assert!(empty_fine.is_ok());
        assert_eq!(empty_fine.unwrap(), BTreeMap::new());

        let empty_two_step = overapprox_well_formed_sub(
            InterfacingProtocols(vec![]),
            &BTreeMap::new(),
            Granularity::TwoStep,
        );
        assert!(empty_two_step.is_ok());
        assert_eq!(empty_two_step.unwrap(), BTreeMap::new());

        // Test interfacing_swarms_4
        // Coarse (this granularity used to be called medium)
        let result_4_coarse = overapprox_well_formed_sub(
            test_utils::get_interfacing_swarms_4(),
            &BTreeMap::new(),
            Granularity::Coarse,
        );
        assert!(result_4_coarse.is_ok());
        let subs_4_coarse = result_4_coarse.unwrap();
        let expected_subs_4_coarse: Subscriptions = BTreeMap::from([
            (
                Role::from("T"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("pos"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                    EventType::from("observing"),
                ]),
            ),
            (
                Role::from("FL"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("pos"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                    EventType::from("observing"),
                ]),
            ),
            (
                Role::from("D"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                    EventType::from("observing"),
                ]),
            ),
            (
                Role::from("F"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                    EventType::from("observing"),
                ]),
            ),
            (
                Role::from("QCR"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                    EventType::from("observing"),
                    EventType::from("report"),
                ]),
            ),
        ]);
        assert_eq!(subs_4_coarse, expected_subs_4_coarse);

        // Fine
        let result_4_fine = overapprox_well_formed_sub(
            test_utils::get_interfacing_swarms_4(),
            &BTreeMap::new(),
            Granularity::Fine,
        );
        assert!(result_4_fine.is_ok());
        let subs_4_fine = result_4_fine.unwrap();
        let expected_subs_4_fine: Subscriptions = BTreeMap::from([
            (
                Role::from("T"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("pos"),
                    EventType::from("part"),
                    EventType::from("time"),
                ]),
            ),
            (
                Role::from("FL"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("pos"),
                    EventType::from("time"),
                ]),
            ),
            (
                Role::from("D"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                ]),
            ),
            (
                Role::from("F"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                    EventType::from("observing"),
                ]),
            ),
            (
                Role::from("QCR"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                    EventType::from("observing"),
                    EventType::from("report"),
                ]),
            ),
        ]);
        assert_eq!(subs_4_fine, expected_subs_4_fine);

        // 'Algorithm 1'/'Two Step'
        let result_4_two_step = overapprox_well_formed_sub(
            test_utils::get_interfacing_swarms_4(),
            &BTreeMap::new(),
            Granularity::TwoStep,
        );
        assert!(result_4_two_step.is_ok());
        let subs_4_two_step = result_4_two_step.unwrap();
        let expected_subs_4_two_step: Subscriptions = BTreeMap::from([
            (
                Role::from("T"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("pos"),
                    EventType::from("part"),
                    EventType::from("time"),
                ]),
            ),
            (
                Role::from("FL"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("pos"),
                    EventType::from("part"),
                    EventType::from("time"),
                ]),
            ),
            (
                Role::from("D"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                ]),
            ),
            (
                Role::from("F"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                    EventType::from("observing"),
                ]),
            ),
            (
                Role::from("QCR"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("time"),
                    EventType::from("car"),
                    EventType::from("observing"),
                    EventType::from("report"),
                ]),
            ),
        ]);
        assert_eq!(subs_4_two_step, expected_subs_4_two_step);

        // Test interfacing_swarms_5
        // Coarse (this granularity used to be called medium)
        let result_5_coarse = overapprox_well_formed_sub(
            test_utils::get_interfacing_swarms_5(),
            &BTreeMap::new(),
            Granularity::Coarse,
        );
        assert!(result_5_coarse.is_ok());
        let subs_5_coarse = result_5_coarse.unwrap();
        let expected_subs_5_coarse: Subscriptions = BTreeMap::from([
            (
                Role::from("IR"),
                BTreeSet::from([
                    EventType::from("e_ir_0"),
                    EventType::from("e_ir_1"),
                    EventType::from("e_r0_0"),
                    EventType::from("e_r0_1"),
                    EventType::from("e_r1_0"),
                ]),
            ),
            (
                Role::from("R0"),
                BTreeSet::from([
                    EventType::from("e_ir_0"),
                    EventType::from("e_ir_1"),
                    EventType::from("e_r0_0"),
                    EventType::from("e_r0_1"),
                    EventType::from("e_r1_0"),
                ]),
            ),
            (
                Role::from("R1"),
                BTreeSet::from([
                    EventType::from("e_ir_0"),
                    EventType::from("e_ir_1"),
                    EventType::from("e_r0_0"),
                    EventType::from("e_r0_1"),
                    EventType::from("e_r1_0"),
                ]),
            ),
        ]);
        assert_eq!(subs_5_coarse, expected_subs_5_coarse);

        // Fine
        let result_5_fine = overapprox_well_formed_sub(
            test_utils::get_interfacing_swarms_5(),
            &BTreeMap::new(),
            Granularity::Fine,
        );
        assert!(result_5_fine.is_ok());
        let subs_5_fine = result_5_fine.unwrap();
        let expected_subs_5_fine: Subscriptions = BTreeMap::from([
            (
                Role::from("IR"),
                BTreeSet::from([
                    EventType::from("e_ir_0"),
                    EventType::from("e_ir_1"),
                    EventType::from("e_r0_0"),
                    EventType::from("e_r0_1"),
                    EventType::from("e_r1_0"),
                ]),
            ),
            (
                Role::from("R0"),
                BTreeSet::from([
                    EventType::from("e_ir_0"),
                    EventType::from("e_ir_1"),
                    EventType::from("e_r0_0"),
                    EventType::from("e_r0_1"),
                    EventType::from("e_r1_0"),
                ]),
            ),
            (
                Role::from("R1"),
                BTreeSet::from([EventType::from("e_ir_0"), EventType::from("e_r1_0")]),
            ),
        ]);
        assert_eq!(subs_5_fine, expected_subs_5_fine);

        // 'Algorithm 1'/'Two Step'
        let result_5_two_step = overapprox_well_formed_sub(
            test_utils::get_interfacing_swarms_5(),
            &BTreeMap::new(),
            Granularity::TwoStep,
        );
        assert!(result_5_two_step.is_ok());
        let subs_5_two_step = result_5_two_step.unwrap();
        let expected_subs_5_two_step: Subscriptions = BTreeMap::from([
            (
                Role::from("IR"),
                BTreeSet::from([
                    EventType::from("e_ir_0"),
                    EventType::from("e_ir_1"),
                    EventType::from("e_r0_0"),
                    EventType::from("e_r0_1"),
                    EventType::from("e_r1_0"),
                ]),
            ),
            (
                Role::from("R0"),
                BTreeSet::from([
                    EventType::from("e_ir_0"),
                    EventType::from("e_ir_1"),
                    EventType::from("e_r0_0"),
                    EventType::from("e_r0_1"),
                    EventType::from("e_r1_0"),
                ]),
            ),
            (
                Role::from("R1"),
                BTreeSet::from([EventType::from("e_ir_0"), EventType::from("e_r1_0")]),
            ),
        ]);
        assert_eq!(subs_5_two_step, expected_subs_5_two_step);
    }

    #[test]
    fn test_refinement_pattern() {
        test_utils::setup_logger();

        // Coarse (this granularity used to be called medium)
        let result_coarse = overapprox_well_formed_sub(
            test_utils::get_ref_pat_protos(),
            &BTreeMap::new(),
            Granularity::Coarse,
        );
        assert!(result_coarse.is_ok());
        let subs_coarse = result_coarse.unwrap();
        let expected_subs_coarse: Subscriptions = BTreeMap::from([
            (
                Role::from("IR0"),
                BTreeSet::from([
                    EventType::from("e_ir0_0"),
                    EventType::from("e_ir0_1"),
                    EventType::from("e_ir1_0"),
                    EventType::from("e_ir1_1"),
                    EventType::from("e_ra"),
                    EventType::from("e_rb"),
                ]),
            ),
            (
                Role::from("IR1"),
                BTreeSet::from([
                    EventType::from("e_ir0_0"),
                    EventType::from("e_ir0_1"),
                    EventType::from("e_ir1_0"),
                    EventType::from("e_ir1_1"),
                    EventType::from("e_ra"),
                    EventType::from("e_rc"),
                ]),
            ),
            (
                Role::from("RA"),
                BTreeSet::from([
                    EventType::from("e_ir0_0"),
                    EventType::from("e_ir0_1"),
                    EventType::from("e_ir1_0"),
                    EventType::from("e_ir1_1"),
                    EventType::from("e_ra"),
                ]),
            ),
            (
                Role::from("RB"),
                BTreeSet::from([
                    EventType::from("e_ir0_0"),
                    EventType::from("e_ir0_1"),
                    EventType::from("e_ir1_0"),
                    EventType::from("e_ir1_1"),
                    EventType::from("e_ra"),
                    EventType::from("e_rb"),
                ]),
            ),
            (
                Role::from("RC"),
                BTreeSet::from([
                    EventType::from("e_ir0_0"),
                    EventType::from("e_ir0_1"),
                    EventType::from("e_ir1_0"),
                    EventType::from("e_ir1_1"),
                    EventType::from("e_ra"),
                    EventType::from("e_rc"),
                ]),
            ),
        ]);
        assert_eq!(subs_coarse, expected_subs_coarse);

        // Fine
        let result_fine = overapprox_well_formed_sub(
            test_utils::get_ref_pat_protos(),
            &BTreeMap::new(),
            Granularity::Fine,
        );
        assert!(result_fine.is_ok());
        let subs_fine = result_fine.unwrap();
        let expected_subs_fine: Subscriptions = BTreeMap::from([
            (
                Role::from("IR0"),
                BTreeSet::from([
                    EventType::from("e_ir0_0"),
                    EventType::from("e_ir0_1"),
                    EventType::from("e_ir1_0"),
                    EventType::from("e_ra"),
                    EventType::from("e_rb"),
                ]),
            ),
            (
                Role::from("IR1"),
                BTreeSet::from([
                    EventType::from("e_ir0_0"),
                    EventType::from("e_ir1_0"),
                    EventType::from("e_ir1_1"),
                    EventType::from("e_ra"),
                    EventType::from("e_rc"),
                ]),
            ),
            (
                Role::from("RA"),
                BTreeSet::from([
                    EventType::from("e_ir0_0"),
                    EventType::from("e_ir1_0"),
                    EventType::from("e_ra"),
                ]),
            ),
            (
                Role::from("RB"),
                BTreeSet::from([
                    EventType::from("e_ir1_0"),
                    EventType::from("e_ir1_1"),
                    EventType::from("e_ra"),
                    EventType::from("e_rb"),
                ]),
            ),
            (
                Role::from("RC"),
                BTreeSet::from([
                    EventType::from("e_ir1_0"),
                    EventType::from("e_ra"),
                    EventType::from("e_rc"),
                ]),
            ),
        ]);
        assert_eq!(subs_fine, expected_subs_fine);

        // 'Algorithm 1'/'Two Step'
        let result_two_step = overapprox_well_formed_sub(
            test_utils::get_ref_pat_protos(),
            &BTreeMap::new(),
            Granularity::TwoStep,
        );
        assert!(result_two_step.is_ok());
        let subs_two_step = result_two_step.unwrap();
        let expected_subs_two_step: Subscriptions = BTreeMap::from([
            (
                Role::from("IR0"),
                BTreeSet::from([
                    EventType::from("e_ir0_0"),
                    EventType::from("e_ir0_1"),
                    EventType::from("e_ir1_0"),
                    EventType::from("e_ir1_1"),
                    EventType::from("e_ra"),
                    EventType::from("e_rb"),
                ]),
            ),
            (
                Role::from("IR1"),
                BTreeSet::from([
                    EventType::from("e_ir0_0"),
                    EventType::from("e_ir1_0"),
                    EventType::from("e_ir1_1"),
                    EventType::from("e_ra"),
                    EventType::from("e_rc"),
                ]),
            ),
            (
                Role::from("RA"),
                BTreeSet::from([
                    EventType::from("e_ir0_0"),
                    EventType::from("e_ir1_0"),
                    EventType::from("e_ra"),
                ]),
            ),
            (
                Role::from("RB"),
                BTreeSet::from([
                    EventType::from("e_ir0_0"),
                    EventType::from("e_ir1_0"),
                    EventType::from("e_ir1_1"),
                    EventType::from("e_ra"),
                    EventType::from("e_rb"),
                ]),
            ),
            (
                Role::from("RC"),
                BTreeSet::from([
                    EventType::from("e_ir0_0"),
                    EventType::from("e_ir1_0"),
                    EventType::from("e_ra"),
                    EventType::from("e_rc"),
                ]),
            ),
        ]);
        assert_eq!(subs_two_step, expected_subs_two_step);
    }
    #[test]
    fn test_extend_subs() {
        test_utils::setup_logger();
        let sub_to_extend = BTreeMap::from([
            (Role::new("D"), BTreeSet::from([EventType::new("pos")])),
            (Role::new("TR"), BTreeSet::from([EventType::new("ok")])),
        ]);

        // Coarse (this granularity used to be called medium)
        let result_coarse = overapprox_well_formed_sub(
            test_utils::get_interfacing_swarms_2(),
            &sub_to_extend,
            Granularity::Coarse,
        );
        assert!(result_coarse.is_ok());
        let subs_coarse = result_coarse.unwrap();
        assert!(subs_coarse[&Role::new("D")].contains(&EventType::new("pos")));
        assert!(subs_coarse[&Role::new("TR")].contains(&EventType::new("ok")));

        // Fine
        let result_fine = overapprox_well_formed_sub(
            test_utils::get_interfacing_swarms_2(),
            &sub_to_extend,
            Granularity::Fine,
        );
        assert!(result_fine.is_ok());
        let subs_fine = result_fine.unwrap();
        assert!(subs_fine[&Role::new("D")].contains(&EventType::new("pos")));
        assert!(subs_fine[&Role::new("TR")].contains(&EventType::new("ok")));

        // 'Algorithm 1'/'Two Step'
        let result_two_step = overapprox_well_formed_sub(
            test_utils::get_interfacing_swarms_2(),
            &sub_to_extend,
            Granularity::TwoStep,
        );
        assert!(result_two_step.is_ok());
        let subs_two_step = result_two_step.unwrap();
        assert!(subs_two_step[&Role::new("D")].contains(&EventType::new("pos")));
        assert!(subs_two_step[&Role::new("TR")].contains(&EventType::new("ok")));
    }
}
