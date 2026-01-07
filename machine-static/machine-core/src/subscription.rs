use std::collections::BTreeSet;

use crate::types::proto_info::{self, roles_on_path};
use crate::types::{
    proto_info::ProtoInfo,
    typescript_types::{EventType, Role, Subscriptions},
};

pub mod exact;
pub mod overapproximation;

// Handle looping event types.
// For each event type t that does not lead to a terminal state, check looping condition from determinacy:
// if t is not in subscriptions, add it to all roles in roles(t, G).
// Awkwardly placed here because it is used by exact and overapproximation.
fn add_looping_event_types(proto_info: &ProtoInfo, subscriptions: &mut Subscriptions) {
    let _span = tracing::info_span!("add_looping_event_types").entered();

    // For each event type t in the set of event types that can not reach a terminal state, check predicate adding t to subs of all involved roles if false.
    // iter() for BTreeSet, gets an iterator that visits the elements in the BTreeSet in ascending order.
    // https://doc.rust-lang.org/std/collections/struct.BTreeSet.html#method.iter
    for t in proto_info.infinitely_looping_events.iter() {
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
    subscriptions: &Subscriptions,
) -> bool {
    let _span = tracing::info_span!("all_roles_sub_to_same").entered();
    event_types.into_iter().any(|t_| {
        involved_roles
            .iter()
            .all(|r| subscriptions.get(r).is_some_and(|event_types_r| event_types_r.contains(&t_)))
    })
}

// Identify those infinitely looping event types G-t-> present in subscriptions of roles(t, G, subscriptions)
// If multiple event types in the same loop satisfy the condition for being a looping event type in subscription
// then return all of them instead of picking one which would be enough. Design choice.
// Instead we could take the smallest event type (according to some order) from each loop.
fn infinitely_looping_event_types_in_sub(
    proto_info: &ProtoInfo,
    subscriptions: &Subscriptions,
) -> BTreeSet<EventType> {
    // infinitely_looping.filter(|t| if all roles in roles_on_path subscribe to t then true otherwise false)
    let _span = tracing::info_span!("infinitely_looping_event_types_in_sub").entered();
    proto_info
        .infinitely_looping_events
        .iter()
        .filter(|t|
            roles_on_path((*t).clone(), proto_info, subscriptions)
            .iter()
            .all(|r| subscriptions.get(r).is_some_and(|event_types_r| event_types_r.contains(*t)))
        )
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils;
    use crate::types::typescript_types::{InterfacingProtocols, Granularity};

    mod loop_tests {
        use std::collections::BTreeMap;
        use super::*;

        macro_rules! check_looping_event_types {
            ($protocol:expr, $expected_infinitely_looping_in_sub:expr) => {
                let interfacing_protocols = InterfacingProtocols(vec![$protocol.clone()]);
                let exact_subscriptions = exact::exact_well_formed_sub(interfacing_protocols.clone(), &BTreeMap::new()).unwrap();
                let overapproximated_subscriptions = overapproximation::overapprox_well_formed_sub(interfacing_protocols.clone(), &BTreeMap::new(), Granularity::TwoStep).unwrap();
                let proto_info = proto_info::prepare_proto_info($protocol);
                let infinitely_looping_in_exact = infinitely_looping_event_types_in_sub(&proto_info, &exact_subscriptions);
                let infinitely_looping_in_approx = infinitely_looping_event_types_in_sub(&proto_info, &overapproximated_subscriptions);

                assert_eq!(infinitely_looping_in_exact, $expected_infinitely_looping_in_sub);
                assert_eq!(infinitely_looping_in_approx, $expected_infinitely_looping_in_sub);
            };
        }
        // This module contains tests for relating to looping event types.
        // Specifically, we test that looping event types in subscriptions are correctly identified.
        #[test]
        fn looping_1() {
            test_utils::setup_logger();
            let proto = InterfacingProtocols(vec![test_utils::get_looping_proto_1()]);
            let exact_subscriptions = exact::exact_well_formed_sub( proto.clone(), &BTreeMap::new()).unwrap();
            let overapproximated_subscriptions = overapproximation::overapprox_well_formed_sub(proto.clone(), &BTreeMap::new(), Granularity::TwoStep).unwrap();
            let proto_info = proto_info::prepare_proto_info(test_utils::get_looping_proto_1());
            let infinitely_looping_in_exact = infinitely_looping_event_types_in_sub(&proto_info, &exact_subscriptions);
            let infinitely_looping_in_approx = infinitely_looping_event_types_in_sub(&proto_info, &overapproximated_subscriptions);

            // Two event types satisfy the conditions of being a looping event type. Right now we use the approach is to return all instead of picking one.
            let expected_infinitely_looping_in_sub = BTreeSet::from([EventType::new("c"), EventType::new("d")]);

            assert_eq!(infinitely_looping_in_exact, expected_infinitely_looping_in_sub);
            assert_eq!(infinitely_looping_in_approx, expected_infinitely_looping_in_sub);
        }

         #[test]
        fn looping_2() {
            test_utils::setup_logger();
            let proto = test_utils::get_looping_proto_2();
            let expected_infinitely_looping_in_sub = BTreeSet::from([EventType::new("c")]);
            check_looping_event_types!(proto, expected_infinitely_looping_in_sub);
        }

        #[test]
        fn looping_3() {
            test_utils::setup_logger();
            let proto = test_utils::get_looping_proto_3();
            let expected_infinitely_looping_in_sub = BTreeSet::from([EventType::new("c"), EventType::new("f")]);
            check_looping_event_types!(proto, expected_infinitely_looping_in_sub);
        }

        #[test]
        fn looping_4() {
            test_utils::setup_logger();
            let proto = test_utils::get_looping_proto_4();
            let expected_infinitely_looping_in_sub = BTreeSet::from([EventType::new("c")]);
            check_looping_event_types!(proto, expected_infinitely_looping_in_sub);
        }

        #[test]
        fn looping_5() {
            test_utils::setup_logger();
            let proto = test_utils::get_looping_proto_5();
            let expected_infinitely_looping_in_sub = BTreeSet::from([EventType::new("a"), EventType::new("e")]);
            check_looping_event_types!(proto, expected_infinitely_looping_in_sub);
        }

        #[test]
        fn looping_6() {
            test_utils::setup_logger();
            let proto = test_utils::get_looping_proto_6();
            // We get b and c. These are branching, so nothing was added in the looping step of computing subs. Change to just return b? Same as comment for looping_1
            let expected_infinitely_looping_in_sub = BTreeSet::from([EventType::new("b"), EventType::new("c")]);
            check_looping_event_types!(proto, expected_infinitely_looping_in_sub);
        }
    }
}
