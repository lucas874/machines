/* use std::collections::{BTreeMap, BTreeSet};

use petgraph::{visit::{Dfs, EdgeRef, Walker}, Direction::{Incoming, Outgoing}};

use crate::composition::composition_types::{ProtoInfo, ProtoStruct};
use super::{
    composition_types::{
        unord_event_pair, EventLabel, UnordEventPair,
    },
    Graph,
};
use crate::{
    types::{EventType, Role},
    NodeId, Subscriptions,
};

// Construct a wf-subscription by constructing the composition of all protocols in protos and analyzing the result
pub fn exact_weak_well_formed_sub<T: SwarmInterface>(
    protos: InterfacingSwarms<T>,
    subs: &Subscriptions,
) -> Result<Subscriptions, ErrorReport> {
    let _span = tracing::info_span!("exact_weak_well_formed_sub").entered();
    let combined_proto_info = swarms_to_proto_info(protos);
    if !combined_proto_info.no_errors() {
        return Err(proto_info_to_error_report(combined_proto_info));
    }

    // If we reach this point the protocols can interface and are all confusion free.
    // We construct a ProtoInfo with the composition as the only protocol and all the
    // information about branches etc. from combined_proto_info
    // and the succeeding_events field updated using the expanded composition.
    let composition = explicit_composition_proto_info(combined_proto_info);
    let sub = exact_wwf_sub(composition, 0, subs);

    Ok(sub)
}

/*
 * Given a swarm protocol return smallest WF-subscription. WF according to new compositional definition.
 * Expand composition and apply rules from definition of WF until subscription stabilizes.
 */
fn exact_wwf_sub1(
    proto_info: ProtoInfo,
    proto_pointer: usize,
    subscriptions: &Subscriptions,
) -> Subscriptions {
    let _span = tracing::info_span!("exact_wwf_sub").entered();
    let (graph, initial) = match proto_info.get_ith_proto(proto_pointer) {
        Some(ProtoStruct {
            graph: g,
            initial: Some(i),
            errors: _,
            interface: _,
        }) => (g, i),
        _ => return BTreeMap::new(),
    };
    let mut subscriptions = subscriptions.clone();
    let mut is_stable = exact_wwf_sub_step1(&proto_info, &graph, initial, &mut subscriptions);
    while !is_stable {
        is_stable = exact_wwf_sub_step1(&proto_info, &graph, initial, &mut subscriptions);
    }

    subscriptions
}

// Apply rules from WF defintion to add event types to subscription.
fn exact_wwf_sub_step1(
    proto_info: &ProtoInfo,
    graph: &Graph,
    initial: NodeId,
    subscriptions: &mut Subscriptions,
) -> bool {
    let _span = tracing::info_span!("exact_wwf_sub_step").entered();
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
        //  Extend subscriptions to satisfy conditions for weak causal consistency
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
            for active in active_transitions_not_conc(
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
            let involved_roles = roles_on_path(event_type.clone(), &proto_info, &subscriptions);

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
                    is_stable = add_to_sub(r.clone(), branching_this_node.clone(), subscriptions) && is_stable;
                }
            }

            // Determinacy 2. joining events.
            // The joining events field is an overapproximation.
            // so check if there are two or more incoming concurrent not concurrent with event type
            if proto_info.joining_events.contains(&event_type) {
                let incoming_pairs_concurrent: Vec<UnordEventPair> =
                    event_pairs_from_node(node, &graph, Incoming)
                        .into_iter()
                        .filter(|pair| proto_info.concurrent_events.contains(pair))
                        .filter(|pair| {
                            pair.iter().all(|e| {
                                !proto_info
                                    .concurrent_events
                                    .contains(&unord_event_pair(e.clone(), event_type.clone()))
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
} */