use std::collections::{BTreeMap, BTreeSet};

use petgraph::{Direction::{Incoming, Outgoing}, visit::{Dfs, EdgeRef, Walker}};

use crate::{errors::ErrorReport, types::{proto_graph::{Graph, NodeId}, proto_info::{ProtoInfo, ProtoStruct, UnordEventPair}, typescript_types::{EventLabel, EventType, InterfacingProtocols, Role, Subscriptions}}};
use crate::types::{proto_graph, proto_info};

// Construct a wf-subscription by constructing the composition of all protocols in protos and analyzing the result
pub fn exact_well_formed_sub(
    protos: InterfacingProtocols,
    subs: &Subscriptions,
) -> Result<Subscriptions, ErrorReport> {
    let _span = tracing::info_span!("exact_well_formed_sub").entered();
    let combined_proto_info = proto_info::swarms_to_proto_info(protos);
    if !combined_proto_info.no_errors() {
        return Err(combined_proto_info.to_error_report());
    }

    // If we reach this point the protocols can interface and are all confusion free.
    // We construct a ProtoInfo with the composition as the only protocol and all the
    // information about branches etc. from combined_proto_info
    // and the succeeding_events field updated using the expanded composition.
    let composition = proto_info::explicit_composition_proto_info(combined_proto_info);
    let sub = exact_wf_sub(composition, 0, subs);

    Ok(sub)
}

/*
 * Given a swarm protocol return smallest WF-subscription. WF according to new compositional definition.
 * Expand composition and apply rules from definition of WF until subscription stabilizes.
 */
fn exact_wf_sub(
    proto_info: ProtoInfo,
    proto_pointer: usize,
    subscriptions: &Subscriptions,
) -> Subscriptions {
    let _span = tracing::info_span!("exact_wf_sub").entered();
    let (graph, initial) = match proto_info.get_ith_proto(proto_pointer) {
        Some(ProtoStruct {
            graph: g,
            initial: Some(i),
            errors: _,
            roles: _,
        }) => (g, i),
        _ => return BTreeMap::new(),
    };
    let mut subscriptions = subscriptions.clone();
    let mut is_stable = exact_wf_sub_step(&proto_info, &graph, initial, &mut subscriptions);
    while !is_stable {
        is_stable = exact_wf_sub_step(&proto_info, &graph, initial, &mut subscriptions);
    }

    // Handle looping event types
    super::add_looping_event_types(&proto_info, &mut subscriptions);

    subscriptions
}

// Apply rules from WF defintion to add event types to subscription.
fn exact_wf_sub_step(
    proto_info: &ProtoInfo,
    graph: &Graph,
    initial: NodeId,
    subscriptions: &mut Subscriptions,
) -> bool {
    let _span = tracing::info_span!("exact_wf_sub_step").entered();
    if graph.node_count() == 0 || initial == NodeId::end() {
        return true
    }
    let mut is_stable = true;
    let add_to_sub =
        |role: Role, mut event_types: BTreeSet<EventType>, subs: &mut Subscriptions| -> bool {
            if subs.contains_key(&role) && event_types.iter().all(|e| subs[&role].contains(e)) {
                return true;
            }
            subs.entry(role)
                .and_modify(|curr| {
                    curr.append(&mut event_types);
                })
                .or_insert(event_types);
            false
        };
    for node in Dfs::new(&graph, initial).iter(&graph) {
        // For each edge going out of node:
        //  Extend subscriptions to satisfy conditions for causal consistency
        //  Make role performing the command subscribe to the emitted event type
        //  Make roles active in continuations subscribe to the event type
        //  Make an overapproximation of the roles in roles(e.G) subscribe to branching events.
        for edge in graph.edges_directed(node, Outgoing) {
            let event_type = edge.weight().get_event_type();

            // Causal consistency 1: roles subscribe to the event types they emit
            is_stable = add_to_sub(
                edge.weight().role.clone(),
                BTreeSet::from([event_type.clone()]),
                subscriptions,
            ) && is_stable;

            // Causal consistency 2: roles subscribe to the event types that immediately precede their own commands
            for active in proto_graph::active_transitions_not_conc(
                edge.target(),
                &graph,
                &event_type,
                &proto_info.concurrent_events,
            ) {
                is_stable = add_to_sub(
                    active.role,
                    BTreeSet::from([event_type.clone()]),
                    subscriptions,
                ) && is_stable;
            }

            // Find all, if any, roles that subscribe to event types emitted later in the protocol.
            let involved_roles = proto_info::roles_on_path(event_type.clone(), &proto_info, &subscriptions);

            // Determinacy 1: roles subscribe to branching events.
            // Events that are branching with event_type.
            let branching_with_event_type: BTreeSet<_> = proto_info
                .branching_events
                .iter()
                .filter(|set| set.contains(&event_type))
                .flatten()
                .cloned()
                .collect();

            // The event types emitted at this node in the set of event types branching together with event_type.
            let branching_this_node: BTreeSet<_> = graph
                .edges_directed(node, Outgoing)
                .map(|e| e.weight().get_event_type())
                .filter(|t| branching_with_event_type.contains(t))
                .collect();

            // If only one event labeled as branching at this node, do not add it to subscriptions.
            // This could happen due to concurrency and loss of behavior on composition.
            if branching_this_node.len() > 1 {
                for r in involved_roles.iter() {
                    is_stable = add_to_sub(r.clone(), branching_this_node.clone(), subscriptions)
                        && is_stable;
                }
            }

            // Determinacy 2. joining events.
            // All joining event types are interfacing event types, but not the other way around.
            // So check if there are two or more incoming concurrent not concurrent with event type
            if proto_info.interfacing_events.contains(&event_type) {
                let incoming_pairs_concurrent: Vec<UnordEventPair> =
                    proto_graph::event_pairs_from_node(node, &graph, Incoming)
                        .into_iter()
                        .filter(|pair| proto_info.concurrent_events.contains(pair))
                        .filter(|pair| {
                            pair.iter().all(|e| {
                                !proto_info
                                    .concurrent_events
                                    .contains(&proto_info::unord_event_pair(e.clone(), event_type.clone()))
                            })
                        })
                        .collect();
                let events_to_add: BTreeSet<EventType> = incoming_pairs_concurrent
                    .into_iter()
                    .flat_map(|pair| pair.into_iter().chain([event_type.clone()]))
                    .collect();
                for r in involved_roles.iter() {
                    is_stable =
                        add_to_sub(r.clone(), events_to_add.clone(), subscriptions) && is_stable;
                }
            }
        }
    }

    is_stable
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils;

    #[test]
    fn test_well_formed_sub() {
        test_utils::setup_logger();

        // Test interfacing_swarms_1
        let result_1 = exact_well_formed_sub(test_utils::get_interfacing_swarms_1(), &BTreeMap::new());
        assert!(result_1.is_ok());
        let subs_1 = result_1.unwrap();
        let expected_subs_1: Subscriptions = BTreeMap::from([
            (Role::from("T"), BTreeSet::from([EventType::from("partID"), EventType::from("pos"), EventType::from("part"), EventType::from("time")])),
            (Role::from("FL"), BTreeSet::from([EventType::from("partID"), EventType::from("pos"), EventType::from("time")])),
            (Role::from("D"), BTreeSet::from([EventType::from("partID"), EventType::from("part"), EventType::from("time")])),
            (Role::from("F"), BTreeSet::from([EventType::from("partID"), EventType::from("part"), EventType::from("time"), EventType::from("car")])),
        ]);
        assert_eq!(subs_1, expected_subs_1);

        // Test interfacing_swarms_2
        let result_2 = exact_well_formed_sub(test_utils::get_interfacing_swarms_2(), &BTreeMap::new());
        assert!(result_2.is_ok());
        let subs_2 = result_2.unwrap();
        let expected_subs_2: Subscriptions = BTreeMap::from([
            (Role::from("T"), BTreeSet::from([EventType::from("partID"), EventType::from("pos"), EventType::from("part"), EventType::from("time")])),
            (Role::from("FL"), BTreeSet::from([EventType::from("partID"), EventType::from("pos"), EventType::from("time")])),
            (Role::from("D"), BTreeSet::from([EventType::from("partID"), EventType::from("part"), EventType::from("time")])),
            (Role::from("F"), BTreeSet::from([EventType::from("partID"), EventType::from("part"), EventType::from("time"), EventType::from("car"), EventType::from("report1")])),
            (Role::from("TR"), BTreeSet::from([EventType::from("partID"), EventType::from("part"), EventType::from("time"), EventType::from("car"), EventType::from("report1"), EventType::from("report2")])),
            (Role::from("QCR"), BTreeSet::from([EventType::from("partID"), EventType::from("part"), EventType::from("time"), EventType::from("car"), EventType::from("report1"), EventType::from("report2"), EventType::from("ok"), EventType::from("notOk")])),
        ]);
        assert_eq!(subs_2, expected_subs_2);

        // Test interfacing_swarms_3
        let result_3 = exact_well_formed_sub(test_utils::get_interfacing_swarms_3(), &BTreeMap::new());
        assert!(result_3.is_ok());
        let subs_3 = result_3.unwrap();
        let expected_subs_3: Subscriptions = BTreeMap::from([
            (Role::from("T"), BTreeSet::from([EventType::from("partID"), EventType::from("pos"), EventType::from("part"), EventType::from("time")])),
            (Role::from("FL"), BTreeSet::from([EventType::from("partID"), EventType::from("pos"), EventType::from("time")])),
            (Role::from("D"), BTreeSet::from([EventType::from("partID"), EventType::from("part"), EventType::from("time")])),
            (Role::from("F"), BTreeSet::from([EventType::from("partID"), EventType::from("part"), EventType::from("time"), EventType::from("car"), EventType::from("report2")])),
            (Role::from("QCR"), BTreeSet::from([EventType::from("partID"), EventType::from("part"), EventType::from("time"), EventType::from("car"), EventType::from("report1"), EventType::from("report2"), EventType::from("report3")]))
        ]);
        assert_eq!(subs_3, expected_subs_3);
    }

    #[test]
    fn test_well_formed_sub_1() {
        test_utils::setup_logger();

        // Test empty set of input protocols
        let empty = exact_well_formed_sub(InterfacingProtocols(vec![]), &BTreeMap::new());
        assert!(empty.is_ok());
        assert_eq!(empty.unwrap(), BTreeMap::new());

        // Test interfacing_swarms_4
        let result_4 = exact_well_formed_sub(test_utils::get_interfacing_swarms_4(), &BTreeMap::new());
        assert!(result_4.is_ok());
        let subs_4 = result_4.unwrap();
        let expected_subs_4: Subscriptions = BTreeMap::from([
            (Role::from("T"), BTreeSet::from([EventType::from("partID"), EventType::from("pos"), EventType::from("part"), EventType::from("time")])),
            (Role::from("FL"), BTreeSet::from([EventType::from("partID"), EventType::from("pos"), EventType::from("time")])),
            (Role::from("D"), BTreeSet::from([EventType::from("partID"), EventType::from("part"), EventType::from("time")])),
            (Role::from("F"), BTreeSet::from([EventType::from("partID"), EventType::from("part"), EventType::from("time"), EventType::from("car"), EventType::from("observing")])),
            (Role::from("QCR"), BTreeSet::from([EventType::from("partID"), EventType::from("part"), EventType::from("time"), EventType::from("car"), EventType::from("observing"), EventType::from("report")]))
        ]);
        assert_eq!(subs_4, expected_subs_4);

        // Test interfacing_swarms_5
        let result_5 = exact_well_formed_sub(test_utils::get_interfacing_swarms_5(), &BTreeMap::new());
        assert!(result_5.is_ok());
        let subs_5 = result_5.unwrap();
        let expected_subs_5: Subscriptions = BTreeMap::from([
            (Role::from("IR"), BTreeSet::from([EventType::from("e_ir_0"), EventType::from("e_ir_1"), EventType::from("e_r0_1"), EventType::from("e_r1_0")])),
            (Role::from("R0"), BTreeSet::from([EventType::from("e_ir_0"), EventType::from("e_ir_1"), EventType::from("e_r0_0"), EventType::from("e_r0_1")])),
            (Role::from("R1"), BTreeSet::from([EventType::from("e_ir_0"), EventType::from("e_r1_0")])),
        ]);
        assert_eq!(subs_5, expected_subs_5);
    }

    #[test]
    fn test_refinement_pattern() {
        test_utils::setup_logger();
        let result = exact_well_formed_sub(test_utils::get_ref_pat_protos(), &BTreeMap::new());
        assert!(result.is_ok());
        let subs = result.unwrap();
        let expected_subs: Subscriptions = BTreeMap::from([
            (Role::from("IR0"), BTreeSet::from([EventType::from("e_ir0_0"), EventType::from("e_ir0_1"), EventType::from("e_ir1_0"), EventType::from("e_ra"), EventType::from("e_rb")])),
            (Role::from("IR1"), BTreeSet::from([EventType::from("e_ir0_0"), EventType::from("e_ir1_0"), EventType::from("e_ir1_1"), EventType::from("e_ra"), EventType::from("e_rc")])),
            (Role::from("RA"), BTreeSet::from([EventType::from("e_ir0_0"), EventType::from("e_ir1_0"), EventType::from("e_ra")])),
            (Role::from("RB"), BTreeSet::from([EventType::from("e_ir1_0"), EventType::from("e_ir1_1"), EventType::from("e_ra"), EventType::from("e_rb")])),
            (Role::from("RC"), BTreeSet::from([EventType::from("e_ir1_0"), EventType::from("e_ra"), EventType::from("e_rc")])),
        ]);
        assert_eq!(subs, expected_subs);
    }

    #[test]
    fn test_extend_subs() {
        test_utils::setup_logger();
        let sub_to_extend = BTreeMap::from([
            (Role::new("D"), BTreeSet::from([EventType::new("pos")])),
            (Role::new("TR"), BTreeSet::from([EventType::new("ok")])),
        ]);
        let result = exact_well_formed_sub(test_utils::get_interfacing_swarms_2(), &sub_to_extend);
        assert!(result.is_ok());
        let subs = result.unwrap();
        assert!(subs[&Role::new("D")].contains(&EventType::new("pos")));
        assert!(subs[&Role::new("TR")].contains(&EventType::new("ok")));
    }
}