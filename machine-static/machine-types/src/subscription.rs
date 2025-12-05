use std::collections::BTreeSet;

use crate::types::{proto_info::ProtoInfo, typescript_types::{EventType, Role, Subscriptions}};
use crate::types::proto_info;

pub mod exact;
pub mod overapproximation;

// Handle looping event types.
// For each event type t that does not lead to a terminal state, check looping condition from determinacy:
// if t is not in subscriptions, add it to all roles in roles(t, G).
// Awkwardly placed here because it is used by exact and overapproximation.
fn add_looping_event_types(proto_info: &ProtoInfo, subscriptions: &mut Subscriptions) {
    let _span = tracing::info_span!("add_looping_event_types").entered();

    // For each event type t in the set of event types that can not reach a terminal state, check predicate adding t to subs of all involved roles if false.
    for t in &proto_info.infinitely_looping_events {
        let t_and_after_t: BTreeSet<EventType> = [t.clone()]
            .into_iter()
            .chain(
                proto_info
                    .succeeding_events
                    .get(t)
                    .cloned()
                    .unwrap_or_else(|| BTreeSet::new()),
            )
            .collect();
        let involved_roles = proto_info::roles_on_path(t.clone(), proto_info, subscriptions);

        // If there is not an event type among t_and_after_t such that all roles subscribe to this event type, add t to the subscription of all involved roles.
        if !all_roles_sub_to_same(t_and_after_t, &involved_roles, &subscriptions) {
            for r in involved_roles.iter() {
                subscriptions
                    .entry(r.clone())
                    .and_modify(|set| {
                        set.insert(t.clone());
                    })
                    .or_insert_with(|| BTreeSet::from([t.clone()]));
            }
        }
    }
}

// True if there exists an event type in event_types such that all roles in involved_roles subscribe to it.
fn all_roles_sub_to_same(
    event_types: BTreeSet<EventType>,
    involved_roles: &BTreeSet<Role>,
    subs: &Subscriptions,
) -> bool {
    let _span = tracing::info_span!("all_roles_sub_to_same").entered();
    let empty = BTreeSet::new();
    event_types
        .into_iter()
        .any(|t_|  involved_roles
            .iter()
            .all(|r| subs.get(r).unwrap_or(&empty).contains(&t_)))
}

/*
    TODO: Find out what to do about this. We can not use check!

    mod subscription_generation_tests {
        use super::*;

        // Tests relating to subscription generation.
        #[test]
        fn test_well_formed_sub() {
            setup_logger();

            // Test interfacing_swarms_1
            let result = exact_well_formed_sub(get_interfacing_swarms_1(), &BTreeMap::new());
            assert!(result.is_ok());
            let subs1 = result.unwrap();
            let error_report = check(get_interfacing_swarms_1(), &subs1);
            assert!(error_report.is_empty());

            let result = overapprox_well_formed_sub(
                get_interfacing_swarms_1(),
                &BTreeMap::new(),
                Granularity::Coarse,
            );
            assert!(result.is_ok());
            let subs2 = result.unwrap();
            let error_report = check(get_interfacing_swarms_1(), &subs2);
            assert!(error_report.is_empty());
            assert!(is_sub_subscription(subs1.clone(), subs2));

            let result = overapprox_well_formed_sub(
                get_interfacing_swarms_1(),
                &BTreeMap::new(),
                Granularity::Medium,
            );
            assert!(result.is_ok());
            let subs2 = result.unwrap();
            let error_report = check(get_interfacing_swarms_1(), &subs2);
            assert!(error_report.is_empty());
            assert!(is_sub_subscription(subs1.clone(), subs2));

            let result = overapprox_well_formed_sub(
                get_interfacing_swarms_1(),
                &BTreeMap::new(),
                Granularity::Fine,
            );
            assert!(result.is_ok());
            let subs2 = result.unwrap();
            let error_report = check(get_interfacing_swarms_1(), &subs2);
            assert!(error_report.is_empty());
            assert!(is_sub_subscription(subs1.clone(), subs2));

            let result = overapprox_well_formed_sub(
                get_interfacing_swarms_1(),
                &BTreeMap::new(),
                Granularity::TwoStep,
            );
            assert!(result.is_ok());
            let subs2 = result.unwrap();
            let error_report = check(get_interfacing_swarms_1(), &subs2);
            assert!(error_report.is_empty());
            assert!(is_sub_subscription(subs1.clone(), subs2));

            // Test interfacing_swarms_2
            let result = exact_well_formed_sub(get_interfacing_swarms_2(), &BTreeMap::new());
            assert!(result.is_ok());
            let subs1 = result.unwrap();
            let error_report = check(get_interfacing_swarms_2(), &subs1);
            assert!(error_report.is_empty());

            let result = overapprox_well_formed_sub(
                get_interfacing_swarms_2(),
                &BTreeMap::new(),
                Granularity::Coarse,
            );
            assert!(result.is_ok());
            let subs2 = result.unwrap();
            let error_report = check(get_interfacing_swarms_2(), &subs2);
            assert!(error_report.is_empty());
            assert!(is_sub_subscription(subs1.clone(), subs2));

            let result = overapprox_well_formed_sub(
                get_interfacing_swarms_2(),
                &BTreeMap::new(),
                Granularity::Medium,
            );
            assert!(result.is_ok());
            let subs2 = result.unwrap();
            let error_report = check(get_interfacing_swarms_2(), &subs2);
            assert!(error_report.is_empty());
            assert!(is_sub_subscription(subs1.clone(), subs2));

            let result = overapprox_well_formed_sub(
                get_interfacing_swarms_2(),
                &BTreeMap::new(),
                Granularity::Fine,
            );
            assert!(result.is_ok());
            let subs2 = result.unwrap();
            let error_report = check(get_interfacing_swarms_2(), &subs2);
            assert!(error_report.is_empty());
            assert!(is_sub_subscription(subs1.clone(), subs2));

            let result = overapprox_well_formed_sub(
                get_interfacing_swarms_2(),
                &BTreeMap::new(),
                Granularity::TwoStep,
            );
            assert!(result.is_ok());
            let subs2 = result.unwrap();
            let error_report = check(get_interfacing_swarms_2(), &subs2);
            assert!(error_report.is_empty());
            assert!(is_sub_subscription(subs1.clone(), subs2));

            // Test interfacing_swarms_3
            let result = exact_well_formed_sub(get_interfacing_swarms_3(), &BTreeMap::new());
            assert!(result.is_ok());
            let subs1 = result.unwrap();
            let error_report = check(get_interfacing_swarms_3(), &subs1);
            assert!(error_report.is_empty());

            let result = overapprox_well_formed_sub(
                get_interfacing_swarms_3(),
                &BTreeMap::new(),
                Granularity::Coarse,
            );
            assert!(result.is_ok());
            let subs2 = result.unwrap();
            let error_report = check(get_interfacing_swarms_3(), &subs2);
            assert!(error_report.is_empty());
            assert!(is_sub_subscription(subs1.clone(), subs2));

            let result = overapprox_well_formed_sub(
                get_interfacing_swarms_3(),
                &BTreeMap::new(),
                Granularity::Medium,
            );
            assert!(result.is_ok());
            let subs2 = result.unwrap();
            let error_report = check(get_interfacing_swarms_3(), &subs2);
            assert!(error_report.is_empty());
            assert!(is_sub_subscription(subs1.clone(), subs2));

            let result = overapprox_well_formed_sub(
                get_interfacing_swarms_3(),
                &BTreeMap::new(),
                Granularity::Fine,
            );
            assert!(result.is_ok());
            let subs2 = result.unwrap();
            let error_report = check(get_interfacing_swarms_3(), &subs2);
            assert!(error_report.is_empty());
            assert!(is_sub_subscription(subs1.clone(), subs2));

            let result = overapprox_well_formed_sub(
                get_interfacing_swarms_3(),
                &BTreeMap::new(),
                Granularity::TwoStep,
            );
            assert!(result.is_ok());
            let subs2 = result.unwrap();
            let error_report = check(get_interfacing_swarms_3(), &subs2);
            assert!(error_report.is_empty());
            assert!(is_sub_subscription(subs1.clone(), subs2));
        }

        #[test]
        fn test_well_formed_sub_1() {
            setup_logger();
            let empty = exact_well_formed_sub(InterfacingProtocols(vec![]), &BTreeMap::new());
            assert!(empty.is_ok());
            assert_eq!(empty.unwrap(), BTreeMap::new());
            // Test interfacing_swarms_4
            let result = exact_well_formed_sub(get_interfacing_swarms_4(), &BTreeMap::new());
            assert!(result.is_ok());
            let subs1 = result.unwrap();
            let error_report = check(get_interfacing_swarms_4(), &subs1);
            assert!(error_report.is_empty());
            let result = overapprox_well_formed_sub(
                get_interfacing_swarms_4(),
                &BTreeMap::new(),
                Granularity::Coarse,
            );
            assert!(result.is_ok());
            let subs2 = result.unwrap();
            let error_report = check(get_interfacing_swarms_4(), &subs2);
            assert!(error_report.is_empty());
            assert!(is_sub_subscription(subs1.clone(), subs2));

            let result = overapprox_well_formed_sub(
                get_interfacing_swarms_4(),
                &BTreeMap::new(),
                Granularity::Medium,
            );
            assert!(result.is_ok());
            let subs2 = result.unwrap();
            let error_report = check(get_interfacing_swarms_4(), &subs2);
            assert!(error_report.is_empty());
            assert!(is_sub_subscription(subs1.clone(), subs2));

            let result = overapprox_well_formed_sub(
                get_interfacing_swarms_4(),
                &BTreeMap::new(),
                Granularity::Fine,
            );
            assert!(result.is_ok());
            let subs2 = result.unwrap();
            let error_report = check(get_interfacing_swarms_4(), &subs2);
            assert!(error_report.is_empty());
            assert!(is_sub_subscription(subs1.clone(), subs2));

            let result = overapprox_well_formed_sub(
                get_interfacing_swarms_4(),
                &BTreeMap::new(),
                Granularity::TwoStep,
            );
            assert!(result.is_ok());
            let subs2 = result.unwrap();
            let error_report = check(get_interfacing_swarms_4(), &subs2);
            assert!(error_report.is_empty());
            assert!(is_sub_subscription(subs1.clone(), subs2));

            // Test interfacing_swarms_5
            let result = exact_well_formed_sub(get_interfacing_swarms_5(), &BTreeMap::new());
            assert!(result.is_ok());
            let subs1 = result.unwrap();
            let error_report = check(get_interfacing_swarms_5(), &subs1);
            assert!(error_report.is_empty());
            let result = overapprox_well_formed_sub(
                get_interfacing_swarms_5(),
                &BTreeMap::new(),
                Granularity::Coarse,
            );
            assert!(result.is_ok());
            let subs2 = result.unwrap();
            let error_report = check(get_interfacing_swarms_5(), &subs2);
            assert!(error_report.is_empty());
            assert!(is_sub_subscription(subs1.clone(), subs2));

            let result = overapprox_well_formed_sub(
                get_interfacing_swarms_5(),
                &BTreeMap::new(),
                Granularity::Medium,
            );
            assert!(result.is_ok());
            let subs2 = result.unwrap();
            let error_report = check(get_interfacing_swarms_5(), &subs2);
            assert!(error_report.is_empty());
            assert!(is_sub_subscription(subs1.clone(), subs2));

            let result = overapprox_well_formed_sub(
                get_interfacing_swarms_5(),
                &BTreeMap::new(),
                Granularity::Fine,
            );
            assert!(result.is_ok());
            let subs2 = result.unwrap();
            let error_report = check(get_interfacing_swarms_5(), &subs2);
            assert!(error_report.is_empty());
            assert!(is_sub_subscription(subs1.clone(), subs2));

            let result = overapprox_well_formed_sub(
                get_interfacing_swarms_5(),
                &BTreeMap::new(),
                Granularity::TwoStep,
            );
            assert!(result.is_ok());
            let subs2 = result.unwrap();
            let error_report = check(get_interfacing_swarms_5(), &subs2);
            assert!(error_report.is_empty());
            assert!(is_sub_subscription(subs1.clone(), subs2));
        }

        #[test]
        fn test_refinement_pattern() {
            setup_logger();
            let composition = compose_protocols(get_ref_pat_protos());
            assert!(composition.is_ok());
            let protos = get_ref_pat_protos();
            let result_composition = exact_well_formed_sub(protos.clone(), &BTreeMap::new());
            assert!(result_composition.is_ok());
            let subs_composition = result_composition.unwrap();
            assert!(check(protos.clone(), &subs_composition).is_empty());
            let result_composition =
                overapprox_well_formed_sub(protos.clone(), &BTreeMap::new(), Granularity::Fine);
            assert!(result_composition.is_ok());
            let subs_composition = result_composition.unwrap();
            assert!(check(protos.clone(), &subs_composition).is_empty());
            let result_composition =
                overapprox_well_formed_sub(protos.clone(), &BTreeMap::new(), Granularity::TwoStep);
            assert!(result_composition.is_ok());
            let subs_composition = result_composition.unwrap();
            assert!(check(protos.clone(), &subs_composition).is_empty());
        }

        #[test]
        fn test_extend_subs() {
            setup_logger();
            let sub_to_extend = BTreeMap::from([
                (Role::new("D"), BTreeSet::from([EventType::new("pos")])),
                (Role::new("TR"), BTreeSet::from([EventType::new("ok")])),
            ]);
            let result1 = exact_well_formed_sub(get_interfacing_swarms_2(), &sub_to_extend);
            let result2 = overapprox_well_formed_sub(
                get_interfacing_swarms_2(),
                &sub_to_extend,
                Granularity::Coarse,
            );
            assert!(result1.is_ok());
            assert!(result2.is_ok());
            let subs1 = result1.unwrap();
            let subs2 = result2.unwrap();
            assert!(check(get_interfacing_swarms_2(), &subs1).is_empty());
            assert!(check(get_interfacing_swarms_2(), &subs2).is_empty());
            assert!(subs1[&Role::new("D")].contains(&EventType::new("pos")));
            assert!(subs2[&Role::new("D")].contains(&EventType::new("pos")));
            assert!(subs1[&Role::new("TR")].contains(&EventType::new("ok")));
            assert!(subs2[&Role::new("TR")].contains(&EventType::new("ok")));

            let result2 = overapprox_well_formed_sub(
                get_interfacing_swarms_2(),
                &sub_to_extend,
                Granularity::Medium,
            );
            assert!(result2.is_ok());
            let subs2 = result2.unwrap();
            assert!(check(get_interfacing_swarms_2(), &subs2).is_empty());
            assert!(subs2[&Role::new("D")].contains(&EventType::new("pos")));
            assert!(subs2[&Role::new("TR")].contains(&EventType::new("ok")));

            let result2 = overapprox_well_formed_sub(
                get_interfacing_swarms_2(),
                &sub_to_extend,
                Granularity::Fine,
            );
            assert!(result2.is_ok());
            let subs2 = result2.unwrap();
            assert!(check(get_interfacing_swarms_2(), &subs2).is_empty());
            assert!(subs2[&Role::new("D")].contains(&EventType::new("pos")));
            assert!(subs2[&Role::new("TR")].contains(&EventType::new("ok")));
        }
    }




*/