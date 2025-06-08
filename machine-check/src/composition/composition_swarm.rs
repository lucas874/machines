use super::composition_types::{CompositionComponent, Granularity, InterfacingSwarms, ProtoStruct};
use super::MapVec;
use super::{
    composition_types::{
        unord_event_pair, EventLabel, ProtoInfo, RoleEventMap, SwarmInterface, UnordEventPair,
    },
    Graph,
};
use crate::composition::composition_types::ProtoLabel;
use crate::types::Command;
use crate::{
    types::{EventType, Role, State, StateName, SwarmLabel, Transition},
    EdgeId, NodeId, Subscriptions, SwarmProtocolType,
};
use itertools::Itertools;
use petgraph::algo::floyd_warshall;
use petgraph::visit::{DfsPostOrder, Reversed};
use petgraph::Directed;
use petgraph::{
    graph::EdgeReference,
    visit::{Dfs, EdgeRef, Walker},
    Direction::{self, Incoming, Outgoing},
};
use std::collections::HashMap;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Error {
    SwarmError(crate::swarm::Error),
    SwarmErrorString(String), // bit ugly but instead of making prepare graph public or making another from_json returning Error type in swarm.rs
    InvalidInterfaceRole(Role),
    InterfaceEventNotInBothProtocols(EventType),
    RoleNotSubscribedToBranch(Vec<EventType>, EdgeId, NodeId, Role),
    RoleNotSubscribedToJoin(Vec<EventType>, EdgeId, Role),
    MoreThanOneEventTypeInCommand(EdgeId),
    EventEmittedMultipleTimes(EventType, Vec<EdgeId>),
    CommandOnMultipleTransitions(Command, Vec<EdgeId>),
    StateCanNotReachTerminal(NodeId),
    InvalidArg, // weird error. not related to shape of protocol, but ok.
}

impl Error {
    fn to_string<N: StateName>(&self, graph: &petgraph::Graph<N, SwarmLabel>) -> String {
        match self {
            Error::SwarmError(e) => crate::swarm::Error::convert(&graph)(e.clone()),
            Error::SwarmErrorString(s) => s.clone(),
            Error::InvalidInterfaceRole(role) => {
                format!("role {role} can not be used as interface")
            }
            Error::InterfaceEventNotInBothProtocols(event_type) => {
                format!("event type {event_type} does not appear in both protocols")
            }
            Error::RoleNotSubscribedToBranch(event_types, edge, node, role) => {
                let events = event_types.join(", ");
                format!(
                    "role {role} does not subscribe to event types {events} in branching transitions at state {}, but is involved after transition {}",
                    &graph[*node].state_name(),
                    Edge(graph, *edge)
                )
            }
            Error::RoleNotSubscribedToJoin(preceding_events, edge, role) => {
                let events = preceding_events.join(", ");
                format!(
                    "role {role} does not subscribe to event types {events} leading to or in joining event in transition {}",
                    Edge(graph, *edge),
                )
            }
            Error::MoreThanOneEventTypeInCommand(edge) => {
                format!(
                    "transition {} emits more than one event type",
                    Edge(graph, *edge)
                )
            }
            Error::EventEmittedMultipleTimes(event_type, edges) => {
                let edges_pretty = edges.iter().map(|edge| Edge(graph, *edge)).join(", ");
                format!(
                    "event type {event_type} emitted in more than one transition: {}",
                    edges_pretty
                )
            }
            Error::CommandOnMultipleTransitions(command, edges) => {
                let edges_pretty = edges.iter().map(|edge| Edge(graph, *edge)).join(", ");
                format!(
                    "command {command} enabled in more than one transition: {}",
                    edges_pretty
                )
            }
            Error::StateCanNotReachTerminal(node) => {
                format!(
                    "state {} can not reach terminal node",
                    &graph[*node].state_name()
                )
            }
            Error::InvalidArg => {
                format!("invalid argument",)
            }
        }
    }

    pub fn convert<N: StateName>(
        graph: &petgraph::Graph<N, SwarmLabel>,
    ) -> impl Fn(Error) -> String + '_ {
        |err| err.to_string(graph)
    }
}

const INVALID_EDGE: &str = "[invalid EdgeId]";

/// copied from swarm.rs helper for printing a transition
struct Edge<'a, N: StateName>(&'a petgraph::Graph<N, SwarmLabel>, EdgeId);

impl<'a, N: StateName> fmt::Display for Edge<'a, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Some((source, target)) = self.0.edge_endpoints(self.1) else {
            return f.write_str(INVALID_EDGE);
        };
        let source = self.0[source].state_name();
        let target = self.0[target].state_name();
        let label = &self.0[self.1];
        write!(f, "({source})--[{label}]-->({target})")
    }
}

// Container for errors accumulated while processing protocols
#[derive(Debug)]
pub struct ErrorReport(pub Vec<(petgraph::Graph<State, SwarmLabel>, Vec<Error>)>);

impl ErrorReport {
    pub fn is_empty(&self) -> bool {
        self.0.iter().all(|(_, es)| es.is_empty())
    }

    pub fn errors(&self) -> Vec<(petgraph::Graph<State, SwarmLabel>, Vec<Error>)> {
        self.0.clone()
    }
}

macro_rules! get_ith_or_error {
    ($proto_info:expr, $proto_pointer:expr) => {
        match $proto_info.get_ith_proto($proto_pointer) {
            Some(ProtoStruct {
                graph: g,
                initial: Some(i),
                errors: e,
                interface: _,
            }) => (g, i, e),
            Some(ProtoStruct {
                graph: _,
                initial: None,
                errors: e,
                interface: _,
            }) => return e,
            None => return vec![Error::InvalidArg],
        }
    };
}

// Well-formedness check
pub fn check<T: SwarmInterface>(protos: InterfacingSwarms<T>, subs: &Subscriptions) -> ErrorReport {
    let _span = tracing::info_span!("check").entered();
    let combined_proto_info = swarms_to_proto_info(protos);
    if !combined_proto_info.no_errors() {
        return proto_info_to_error_report(combined_proto_info);
    }

    // If we reach this point the protocols can interface and are all confusion free.
    // We construct a ProtoInfo with the composition as the only protocol and all the
    // information about branches etc. from combined_proto_info 
    // and the succeeding_events field updated using the expanded composition.
    let composition = explicit_composition_proto_info(combined_proto_info);
    let composition_checked = weak_well_formed_proto_info(composition, subs);

    proto_info_to_error_report(composition_checked)
}

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

// Construct wf-subscription compositionally.
// Overapproximates the subscription one would obtain from exact_weak_well_formed_sub().
pub fn overapprox_weak_well_formed_sub<T: SwarmInterface>(
    protos: InterfacingSwarms<T>,
    subs: &Subscriptions,
    granularity: Granularity,
) -> Result<Subscriptions, ErrorReport> {
    let _span = tracing::info_span!("overapprox_weak_well_formed_sub").entered();
    let combined_proto_info = swarms_to_proto_info(protos);
    if !combined_proto_info.no_errors() {
        return Err(proto_info_to_error_report(combined_proto_info));
    }

    // If we reach this point the protocols can interface and are all confusion free.
    // We construct a ProtoInfo with the composition as the only protocol and all the
    // information about branches etc. from combined_proto_info
    let sub = overapprox_wwf_sub(&mut combined_proto_info.clone(), subs, granularity);
    Ok(sub)
}

// Construct a ProtoInfo containing all protocols, all branching events, joining events etc.
// Then add any errors arising from confusion freeness to the proto info and return it.
// Does not compute transitive closure of combined succeeding_events, simply takes union of component succeeding_events fields.
pub fn swarms_to_proto_info<T: SwarmInterface>(protos: InterfacingSwarms<T>) -> ProtoInfo {
    let _span = tracing::info_span!("swarms_to_proto_info").entered();
    let combined_proto_info = combine_proto_infos_fold(prepare_proto_infos::<T>(protos));
    confusion_free_proto_info(combined_proto_info)
}

// Construct a graph that is the 'expanded' composition of protos.
pub fn compose_protocols<T: SwarmInterface>(
    protos: InterfacingSwarms<T>,
) -> Result<(Graph, NodeId), ErrorReport> {
    let _span = tracing::info_span!("compose_protocols").entered();
    let combined_proto_info = swarms_to_proto_info(protos);
    if !combined_proto_info.no_errors() {
        return Err(proto_info_to_error_report(combined_proto_info));
    }

    let p = explicit_composition_proto_info(combined_proto_info)
        .get_ith_proto(0)
        .unwrap();
    Ok((p.graph, p.initial.unwrap()))
}

// Perform wf checks on every protocol in a ProtoInfo.
// Does not check confusion-freeness.
fn weak_well_formed_proto_info(proto_info: ProtoInfo, subs: &Subscriptions) -> ProtoInfo {
    let _span = tracing::info_span!("weak_well_formed_proto_info").entered();
    let protocols: Vec<_> = proto_info
        .protocols
        .clone()
        .into_iter()
        .enumerate()
        .map(|(i, p)| {
            let errors = vec![p.errors, weak_well_formed(&proto_info, i, subs)].concat();
            ProtoStruct { errors, ..p }
        })
        .collect();

    ProtoInfo {
        protocols,
        ..proto_info
    }
}

// Perform confusion freeness check on every protocol in a ProtoInfo.
fn confusion_free_proto_info(proto_info: ProtoInfo) -> ProtoInfo {
    let _span = tracing::info_span!("confusion_free_proto_info").entered();
    let protocols: Vec<_> = proto_info
        .protocols
        .clone()
        .into_iter()
        .enumerate()
        .map(|(i, p)| {
            let errors = vec![p.errors, confusion_free(&proto_info, i)].concat();
            ProtoStruct { errors, ..p }
        })
        .collect();

    ProtoInfo {
        protocols,
        ..proto_info
    }
}

/*
 * Check well-formedness of protocol at index proto_pointer in proto_info w.r.t. subs.
 * A graph that was constructed with prepare_graph with no errors will have one event type per command.
 * Similarly, such a graph will be weakly confusion free, which means we do not have to check for
 * command and log determinism like we do in swarm::well_formed.
 *
 * Does not check confusion freeness.
 */
fn weak_well_formed(
    proto_info: &ProtoInfo,
    proto_pointer: usize,
    subs: &Subscriptions,
) -> Vec<Error> {
    let _span = tracing::info_span!("weak_well_formed").entered();
    let mut errors = Vec::new();
    let empty = BTreeSet::new();
    let sub = |r: &Role| subs.get(r).unwrap_or(&empty);
    let (graph, initial, _) = get_ith_or_error!(proto_info, proto_pointer);

    // Visit all transitions in protocol and perform causal consistency and determinacy checks.
    for node in Dfs::new(&graph, initial).iter(&graph) {
        for edge in graph.edges_directed(node, Outgoing) {
            let event_type = edge.weight().get_event_type();

            // Causal consistency
            // Check if role subscribes to own emitted event.
            if !sub(&edge.weight().role).contains(&event_type) {
                errors.push(Error::SwarmError(
                    crate::swarm::Error::ActiveRoleNotSubscribed(edge.id()),
                ));
            }

            // Causal consistency
            // Check if role subscribes to all events immediately preceding this command.
            // Active transitions_not_conc gets the transitions going out of edge.target()
            // and filters out the ones emitting events concurrent with event type of 'edge'.
            for successor in active_transitions_not_conc(
                edge.target(),
                &graph,
                &event_type,
                &proto_info.concurrent_events,
            ) {
                if !sub(&successor.role).contains(&event_type) {
                    errors.push(Error::SwarmError(
                        crate::swarm::Error::LaterActiveRoleNotSubscribed(
                            edge.id(),
                            successor.role,
                        ),
                    ));
                }
            }

            // Roles subscribing to event types emitted later in the protocol.
            let involved_roles = roles_on_path(event_type.clone(), &proto_info, subs);

            // Determinacy.
            // Corresponds to branching rule of weak determinacy.
            if proto_info
                .branching_events
                .iter()
                .any(|branch_set| branch_set.contains(&event_type))
            {
                // If event is branching get all branching events related to 'original' branch.
                // We could have multiple branching events from different protocols at node.
                // These would be concurrent, we only worry about the original event types branching together with event_type.
                let branching_with_this_event = proto_info
                    .branching_events
                    .iter()
                    .find(|set| set.contains(&event_type))
                    .cloned()
                    .unwrap();
                let branching_this_node: BTreeSet<EventType> = graph
                    .edges_directed(node, Outgoing)
                    .map(|e| e.weight().get_event_type())
                    .filter(|e| branching_with_this_event.contains(e))
                    .collect();

                // If only one event labeled as branching at this node, do not count it as an error if not subbed.
                // Could happen due loss of behavior.
                let branches = if branching_this_node.len() > 1 {
                    branching_this_node
                } else {
                    BTreeSet::new()
                };

                // Find all, if any, roles that subscribe to event types emitted later in the protocol that do not subscribe to branches and accumulate errors.
                let involved_not_subbed = involved_roles
                    .iter()
                    .filter(|r| !branches.is_subset(&sub(&r)));
                let mut branching_errors: Vec<_> = involved_not_subbed
                    .map(|r| {
                        (
                            r,
                            branches
                                .difference(&sub(&r))
                                .cloned()
                                .collect::<Vec<EventType>>(),
                        )
                    })
                    .map(|(r, event_types)| {
                        Error::RoleNotSubscribedToBranch(event_types, edge.id(), node, r.clone())
                    })
                    .collect();
                errors.append(&mut branching_errors);
            }

            // Determinacy.
            // Corresponds to joining rule of weak determinacy.
            if proto_info.joining_events.contains(&event_type) {
                // Find pairs of concurrent event types that are both emitted immediately before event_type (i.e. not concurrent with event_type).
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

                // Flatten events identified above and add event type. If no pairs join_set will be empty. Event type chained multiple times, but ok.
                let join_set: BTreeSet<EventType> = incoming_pairs_concurrent
                    .into_iter()
                    .flat_map(|pair| pair.into_iter().chain([event_type.clone()]))
                    .collect();

                // Find all, if any, roles that subscribe to event types emitted later in the protocol that do not subscribe to joins and prejoins and accumulate errors.
                let involved_not_subbed = involved_roles
                    .iter()
                    .filter(|r| !join_set.is_subset(sub(r)));
                let mut joining_errors: Vec<_> = involved_not_subbed
                    .map(|r| {
                        (
                            r,
                            join_set
                                .difference(&sub(r))
                                .cloned()
                                .collect::<Vec<EventType>>(),
                        )
                    })
                    .map(|(r, event_types)| {
                        Error::RoleNotSubscribedToJoin(event_types.clone(), edge.id(), r.clone())
                    })
                    .collect();
                errors.append(&mut joining_errors);
            }
        }
    }
    errors
}

// Check confusion-freeness of a concurrency-free protocol at index proto_pointer in proto_info.
fn confusion_free(proto_info: &ProtoInfo, proto_pointer: usize) -> Vec<Error> {
    let _span = tracing::info_span!("confusion_free").entered();
    let (graph, _, _) = get_ith_or_error!(proto_info, proto_pointer);

    // Map from event types to vec of edge id
    // Map from commands to vec of edge id
    // Error accumulator
    let mut event_types: BTreeMap<EventType, Vec<EdgeId>> = BTreeMap::new();
    let mut commands: BTreeMap<Command, Vec<EdgeId>> = BTreeMap::new();
    let mut errors = vec![];

    // Populate maps and check that each event type/command is only emitted/enabled in one transition.
    for edge in graph.edge_references() {
        let weight = edge.weight();
        event_types
            .entry(weight.get_event_type())
            .and_modify(|edge_ids| edge_ids.push(edge.id()))
            .or_insert_with(|| vec![edge.id()]);
        commands
            .entry(weight.cmd.clone())
            .and_modify(|edge_ids| edge_ids.push(edge.id()))
            .or_insert_with(|| vec![edge.id()]);
    }

    for (event_type, edge_indices) in event_types.iter() {
        if edge_indices.len() > 1 {
            errors.push(Error::EventEmittedMultipleTimes(
                event_type.clone(),
                edge_indices.clone(),
            ));
        }
    }
    for (command, edge_indices) in commands.iter() {
        if edge_indices.len() > 1 {
            errors.push(Error::CommandOnMultipleTransitions(
                command.clone(),
                edge_indices.clone(),
            ));
        }
    }
    
    // This requirement is not part of confusion-freeness. 
    // Our check then is too strict. Prohibits the set of non-terminating swarm protocols. 
    // We do this to not check for the looping condition in determinacy checks/subscription generation. 
    errors.append(&mut all_nodes_reach_zero(&graph));
    errors
}

/*
 * Given a swarm protocol return smallest WF-subscription. WF according to new compositional definition.
 * Expand composition and apply rules from definition of WF until subscription stabilizes.
 */
fn exact_wwf_sub(
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
    let mut is_stable = exact_wwf_sub_step(&proto_info, &graph, initial, &mut subscriptions);
    while !is_stable {
        is_stable = exact_wwf_sub_step(&proto_info, &graph, initial, &mut subscriptions);
    }

    subscriptions
}

// Apply rules from WF defintion to add event types to subscription.
fn exact_wwf_sub_step(
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
            // weak causal consistency 1: a role subscribes to the events it emits
            is_stable = add_to_sub(
                edge.weight().role.clone(),
                BTreeSet::from([event_type.clone()]),
                subscriptions,
            ) && is_stable;

            // weak causal consistency 2: a role subscribes to events that immediately precedes its own commands
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

            let involved_roles = roles_on_path(event_type.clone(), &proto_info, &subscriptions);
            // weak determinacy 1: roles subscribe to branching events.
            if proto_info
                .branching_events
                .iter()
                .any(|branch_set| branch_set.contains(&event_type))
            {
                let branching_with_this_event = proto_info
                    .branching_events
                    .iter()
                    .find(|set| set.contains(&event_type))
                    .cloned()
                    .unwrap();
                let branching_this_node: BTreeSet<EventType> = graph
                    .edges_directed(node, Outgoing)
                    .map(|e| e.weight().get_event_type())
                    .filter(|e| branching_with_this_event.contains(e))
                    .collect();

                // if only one event labeled as branching at this node, do not count it as an error if not subbed.
                // could happen due to concurrency and loss of behavior. In such case we will encounter the 'original'
                // branch and it will be checked there. nope do not do this... slight overapprox without if maybe?
                let branches = if branching_this_node.len() > 1 {
                    branching_this_node
                } else {
                    BTreeSet::new()
                };

                for r in involved_roles.iter() {
                    is_stable = add_to_sub(r.clone(), branches.clone(), subscriptions) && is_stable;
                }
            }

            // weak determinacy 2. joining events.
            // With new strategy: the joining events are an overapproximation.
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
}

fn overapprox_wwf_sub(
    proto_info: &mut ProtoInfo,
    subscription: &Subscriptions,
    granularity: Granularity,
) -> Subscriptions {
    let _span = tracing::info_span!("overapprox_wwf_sub").entered();
    match granularity {
        Granularity::Fine => finer_overapprox_wwf_sub(proto_info, subscription, false),
        Granularity::Medium => finer_overapprox_wwf_sub(proto_info, subscription, true),
        Granularity::Coarse => coarse_overapprox_wwf_sub(proto_info, subscription),
        Granularity::TwoStep => two_step_overapprox_wwf_sub(proto_info, &mut subscription.clone()),
    }
}

fn coarse_overapprox_wwf_sub(
    proto_info: &ProtoInfo,
    subscription: &Subscriptions,
) -> Subscriptions {
    let _span = tracing::info_span!("coarse_overapprox_wwf_sub").entered();
    // for each role add all branching.
    // for each role add all joining and immediately pre joining that are concurrent
    // for each role, add own events and the events immediately preceding these
    let get_pre_joins = |e: &EventType| -> BTreeSet<EventType> {
        let pre = proto_info
            .immediately_pre
            .get(e)
            .cloned()
            .unwrap_or_default();
        let product = pre.clone().into_iter().cartesian_product(&pre);
        product
            .filter(|(e1, e2)| {
                *e1 != **e2
                    && proto_info
                        .concurrent_events
                        .contains(&unord_event_pair(e1.clone(), (*e2).clone()))
            })
            .map(|(e1, e2)| [e1, e2.clone()])
            .flatten()
            .collect()
    };
    let events_to_add_to_all: BTreeSet<EventType> = proto_info
        .branching_events
        .clone()
        .into_iter()
        .flatten()
        .chain(proto_info.joining_events.clone().into_iter())
        .chain(
            proto_info
                .joining_events
                .iter()
                .flat_map(|e| get_pre_joins(e)),
        )
        .collect();

    let sub: BTreeMap<Role, BTreeSet<EventType>> = proto_info
        .role_event_map
        .iter()
        .map(|(role, labels)| {
            (
                role.clone(),
                labels
                    .iter()
                    .flat_map(|label| {
                        proto_info
                            .immediately_pre
                            .get(&label.get_event_type())
                            .cloned()
                            .unwrap_or_default()
                            .clone()
                            .into_iter()
                            .chain([label.get_event_type()])
                    })
                    .chain(events_to_add_to_all.clone().into_iter())
                    .collect::<BTreeSet<EventType>>(),
            )
        })
        .collect();

    combine_maps(subscription.clone(), sub, None)
}

fn finer_overapprox_wwf_sub(
    proto_info: &mut ProtoInfo,
    subscription: &Subscriptions,
    with_all_interfacing: bool,
) -> Subscriptions {
    let _span = tracing::info_span!("finer_overapprox_wwf_sub").entered();
    let mut subscription = subscription.clone();
    proto_info.succeeding_events =
        transitive_closure_succeeding(proto_info.succeeding_events.clone());
    // causal consistency
    for (role, labels) in &proto_info.role_event_map {
        let event_types: BTreeSet<_> = labels.iter().map(|label| label.get_event_type()).collect();
        let preceding_event_types: BTreeSet<_> = event_types
            .iter()
            .flat_map(|e| {
                proto_info
                    .immediately_pre
                    .get(e)
                    .cloned()
                    .unwrap_or_default()
            })
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

    // determinacy
    finer_approx_add_branches_and_joins(proto_info, &mut subscription, with_all_interfacing);

    subscription
}

fn finer_approx_add_branches_and_joins(
    proto_info: &ProtoInfo,
    subscription: &mut Subscriptions,
    with_all_interfacing: bool,
) {
    let _span = tracing::info_span!("finer_approx_add_branches_and_joins").entered();
    let mut is_stable = false;
    let get_pre_joins = |e: &EventType| -> BTreeSet<EventType> {
        let pre = proto_info
            .immediately_pre
            .get(e)
            .cloned()
            .unwrap_or_default();
        let product = pre.clone().into_iter().cartesian_product(&pre);
        product
            .filter(|(e1, e2)| {
                *e1 != **e2
                    && proto_info
                        .concurrent_events
                        .contains(&unord_event_pair(e1.clone(), (*e2).clone()))
            })
            .map(|(e1, e2)| [e1, e2.clone()])
            .flatten()
            .collect()
    };

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

    if with_all_interfacing {
        let interested_roles: Vec<Role> = subscription.keys().cloned().collect();
        for joining_event in &proto_info.joining_events {
            let join_and_prejoin: BTreeSet<_> = [joining_event.clone()]
                .into_iter()
                .chain(get_pre_joins(&joining_event).into_iter())
                .collect();
            for role in &interested_roles {
                add_to_sub(role.clone(), join_and_prejoin.clone(), subscription);
            }
        }
    }

    while !is_stable {
        is_stable = true;
        // determinacy: joins
        if !with_all_interfacing {
            for joining_event in &proto_info.joining_events {
                let interested_roles =
                    roles_on_path(joining_event.clone(), proto_info, &subscription);
                let pre_join_events = get_pre_joins(&joining_event);
                let join_and_prejoin = if !pre_join_events.is_empty() {
                    [joining_event.clone()]
                        .into_iter()
                        .chain(pre_join_events.into_iter())
                        .collect()
                } else {
                    BTreeSet::new()
                };
                for role in interested_roles {
                    is_stable =
                        add_to_sub(role, join_and_prejoin.clone(), subscription) && is_stable;
                }
            }
        }

        // determinacy: branches
        for branching_events in &proto_info.branching_events {
            let interested_roles = branching_events
                .iter()
                .flat_map(|e| roles_on_path(e.clone(), proto_info, &subscription))
                .collect::<BTreeSet<_>>();
            for role in interested_roles {
                is_stable = add_to_sub(role, branching_events.clone(), subscription) && is_stable;
            }
        }
    }
}

// Safe, overapproximating subscription generation as described in article.
fn two_step_overapprox_wwf_sub(
    proto_info: &mut ProtoInfo,
    subscription: &mut Subscriptions,
) -> Subscriptions {
    let _span = tracing::info_span!("two_step_overapprox_wwf_sub").entered();

    // get concurrent event types preceding a join
    let get_pre_joins = |e: &EventType| -> BTreeSet<EventType> {
        let pre = proto_info
            .immediately_pre
            .get(e)
            .cloned()
            .unwrap_or_default();
        let product = pre.clone().into_iter().cartesian_product(&pre);
        product
            .filter(|(e1, e2)| {
                *e1 != **e2
                    && proto_info
                        .concurrent_events
                        .contains(&unord_event_pair(e1.clone(), (*e2).clone()))
            })
            .map(|(e1, e2)| [e1, e2.clone()])
            .flatten()
            .collect()
    };

    // add events to a subscription, return true of they were already in the subscription and false otherwise
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

    // causal consistency
    for (role, labels) in &proto_info.role_event_map {
        let event_types: BTreeSet<_> = labels.iter().map(|label| label.get_event_type()).collect();
        let preceding_event_types: BTreeSet<_> = event_types
            .iter()
            .flat_map(|e| {
                proto_info
                    .immediately_pre
                    .get(e)
                    .cloned()
                    .unwrap_or_default()
            })
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
        // determinacy: branches
        for branching_events in &proto_info.branching_events {
            let interested_roles = branching_events
                .iter()
                .flat_map(|e| roles_on_path(e.clone(), proto_info, &subscription))
                .collect::<BTreeSet<_>>();
            for role in interested_roles {
                is_stable = add_to_sub(role, branching_events.clone(), subscription) && is_stable;
            }
        }

        // determinacy: joins. the joining_events field of proto_info really holds all interfacing events so filter them to get joins
        for joining_event in &proto_info.joining_events {
            let interested_roles = roles_on_path(joining_event.clone(), proto_info, &subscription);
            let pre_join_events = get_pre_joins(&joining_event);
            let join_and_prejoin = if !pre_join_events.is_empty() {
                [joining_event.clone()]
                    .into_iter()
                    .chain(pre_join_events.into_iter())
                    .collect()
            } else {
                BTreeSet::new()
            };
            for role in interested_roles {
                is_stable = add_to_sub(role, join_and_prejoin.clone(), subscription) && is_stable;
            }
        }

        // intefacing rule from algorithm in article
        for joining_event in &proto_info.joining_events {
            let interested_roles = roles_on_path(joining_event.clone(), proto_info, &subscription);
            for role in interested_roles {
                is_stable = add_to_sub(role, BTreeSet::from([joining_event.clone()]), subscription)
                    && is_stable;
            }
        }
    }

    subscription.clone()
}

fn combine_proto_infos<T: SwarmInterface>(
    proto_info1: ProtoInfo,
    proto_info2: ProtoInfo,
    interface: T,
) -> ProtoInfo {
    let _span = tracing::info_span!("combine_proto_infos").entered();
    let errors = interface.check_interface(&proto_info1, &proto_info2);
    if !errors.is_empty() {
        let protocols = vec![
            proto_info1.protocols.clone(),
            proto_info2.protocols.clone(),
            vec![ProtoStruct::new(
                Graph::new(),
                None,
                errors,
                BTreeSet::new(),
            )],
        ]
        .concat();
        // Would work to construct it just like normally. but..
        return ProtoInfo::new_only_proto(protocols);
    }
    let interfacing_event_types = interface.interfacing_event_types(&proto_info1, &proto_info2);
    let protocols = vec![proto_info1.protocols.clone(), proto_info2.protocols.clone()].concat();
    let role_event_map = combine_maps(
        proto_info1.role_event_map.clone(),
        proto_info2.role_event_map.clone(),
        None,
    );
    let concurrent_events = get_concurrent_events(&proto_info1, &proto_info2, &interface);
    let branching_events: Vec<BTreeSet<EventType>> = proto_info1
        .branching_events
        .into_iter()
        .chain(proto_info2.branching_events.into_iter())
        .collect();
    // add the interfacing events to the set of joining events
    let joining_events: BTreeSet<EventType> = proto_info1
        .joining_events
        .into_iter()
        .chain(proto_info2.joining_events.into_iter())
        .chain(interfacing_event_types.into_iter())
        .collect();
    let immediately_pre = combine_maps(
        proto_info1.immediately_pre.clone(),
        proto_info2.immediately_pre.clone(),
        None,
    );
    let happens_after = combine_maps(
        proto_info1.succeeding_events,
        proto_info2.succeeding_events,
        None,
    );
    let happens_after = happens_after;

    ProtoInfo::new(
        protocols,
        role_event_map,
        concurrent_events,
        branching_events,
        joining_events,
        immediately_pre,
        happens_after,
    )
}

fn combine_proto_infos_fold<T: SwarmInterface>(protos: Vec<(ProtoInfo, Option<T>)>) -> ProtoInfo {
    let _span = tracing::info_span!("combine_proto_infos_fold").entered();
    if protos.is_empty()
        || protos[0].1.is_some()
        || protos[1..].iter().any(|(_, interface)| interface.is_none())
    {
        return ProtoInfo::new_only_proto(vec![ProtoStruct::new(
            Graph::new(),
            None,
            vec![Error::InvalidArg],
            BTreeSet::new(),
        )]);
    }

    let (proto, _) = protos[0].clone();

    protos[1..]
        .to_vec()
        .into_iter()
        .fold(proto, |acc, (p, interface)| {
            combine_proto_infos(acc, p, interface.unwrap())
        })
}

// Given some node, return the swarmlabels going out of that node that are not concurrent with 'event_type'.
fn active_transitions_not_conc(
    node: NodeId,
    graph: &Graph,
    event_type: &EventType,
    concurrent_events: &BTreeSet<BTreeSet<EventType>>,
) -> Vec<SwarmLabel> {
    graph
        .edges_directed(node, Outgoing)
        .map(|e| e.weight().clone())
        .filter(|e| {
            !concurrent_events.contains(&BTreeSet::from([event_type.clone(), e.get_event_type()]))
        })
        .collect()
}

// The involved roles on a path are those roles that subscribe to one or
// more of the event types emitted in a transition reachable from the transition
// represented by its emitted event 'event_type'.
fn roles_on_path(
    event_type: EventType,
    proto_info: &ProtoInfo,
    subs: &Subscriptions,
) -> BTreeSet<Role> {
    let succeeding_events: BTreeSet<EventType> = proto_info
        .succeeding_events
        .get(&event_type)
        .cloned()
        .unwrap_or_default();
    subs.iter()
        .filter(|(_, events)| events.intersection(&succeeding_events).count() != 0)
        .map(|(r, _)| r.clone())
        .collect()
}

// Compute a map mapping event types to the set of event types that follow it.
// I.e. for each event type t all those event types t' that can be emitted after t.
fn after_not_concurrent(
    graph: &Graph,
    initial: NodeId,
    concurrent_events: &BTreeSet<BTreeSet<EventType>>,
) -> BTreeMap<EventType, BTreeSet<EventType>> {
    let _span = tracing::info_span!("after_not_concurrent").entered();
    let mut succ_map: BTreeMap<EventType, BTreeSet<EventType>> = BTreeMap::new();
    let mut is_stable = after_not_concurrent_step(graph, initial, concurrent_events, &mut succ_map);

    while !is_stable {
        is_stable = after_not_concurrent_step(graph, initial, concurrent_events, &mut succ_map);
    }

    succ_map
}

// For each event type t we get a set ('active_in_successor') of event types
// that only contains event types that are immediately after t and not concurrent with t.
// We then add each event type t' in active_in_successor and all the event types t''
// that we already know are after t' to the set of event types succeeding t.
fn after_not_concurrent_step(
    graph: &Graph,
    initial: NodeId,
    concurrent_events: &BTreeSet<BTreeSet<EventType>>,
    succ_map: &mut BTreeMap<EventType, BTreeSet<EventType>>,
) -> bool {
    let mut is_stable = true;
    let mut walk = DfsPostOrder::new(&graph, initial);
    while let Some(node) = walk.next(&graph) {
        for edge in graph.edges_directed(node, Outgoing) {
            let event_type = edge.weight().get_event_type();
            let active_in_successor =
                active_transitions_not_conc(edge.target(), graph, &event_type, concurrent_events)
                    .map(|label| label.get_event_type());

            let mut succ_events: BTreeSet<EventType> = active_in_successor
                .clone()
                .into_iter()
                .flat_map(|e| {
                    let events = succ_map.get(&e).cloned().unwrap_or_default();
                    events.clone()
                })
                .chain(active_in_successor.into_iter())
                .collect();

            if !succ_map.contains_key(&event_type)
                || !succ_events
                    .iter()
                    .all(|e| succ_map[&event_type].contains(e))
            {
                succ_map
                    .entry(event_type)
                    .and_modify(|events| {
                        events.append(&mut succ_events);
                    })
                    .or_insert(succ_events);
                is_stable = false;
            }
        }
    }
    is_stable
}

pub fn transitive_closure_succeeding(
    succ_map: BTreeMap<EventType, BTreeSet<EventType>>,
) -> BTreeMap<EventType, BTreeSet<EventType>> {
    let _span = tracing::info_span!("transitive_closure_succeeding").entered();
    let mut graph: petgraph::Graph<EventType, (), Directed> = petgraph::Graph::new();
    let mut node_map = BTreeMap::new();
    for (event, succeeding) in &succ_map {
        if !node_map.contains_key(event) {
            node_map.insert(event.clone(), graph.add_node(event.clone()));
        }
        for succ in succeeding {
            if !node_map.contains_key(succ) {
                node_map.insert(succ.clone(), graph.add_node(succ.clone()));
            }
            graph.add_edge(node_map[event], node_map[succ], ());
        }
    }

    let reflexive_transitive_closure = floyd_warshall(&graph, |_| 1);
    let transitive_closure: Vec<_> = reflexive_transitive_closure
        .unwrap_or_else(|_| HashMap::new())
        .into_iter()
        .filter(|(_, v)| *v != i32::MAX && *v != 0)
        .map(|(related_pair, _)| related_pair)
        .collect();

    let mut succ_map_new: BTreeMap<EventType, BTreeSet<EventType>> = BTreeMap::new();
    for (i1, i2) in transitive_closure {
        succ_map_new
            .entry(graph[i1].clone())
            .and_modify(|succeeding_events| {
                succeeding_events.insert(graph[i2].clone());
            })
            .or_insert_with(|| BTreeSet::from([graph[i2].clone()]));
    }

    // do this because of loops. everything reachable from itself in result from floyd_warshall(), but we filter these out. add them again if loops.
    combine_maps(succ_map, succ_map_new, None)
}

fn prepare_proto_infos<T: SwarmInterface>(
    protos: InterfacingSwarms<T>,
) -> Vec<(ProtoInfo, Option<T>)> {
    let _span = tracing::info_span!("prepare_proto_infos").entered();
    protos
        .0
        .iter()
        .map(|p| (prepare_proto_info::<T>(p.clone()), p.interface.clone()))
        .collect()
}

// Precondition: proto does not contain concurrency.
fn prepare_proto_info<T: SwarmInterface>(proto: CompositionComponent<T>) -> ProtoInfo {
    let _span = tracing::info_span!("prepare_proto_info").entered();
    let mut role_event_map: RoleEventMap = BTreeMap::new();
    let mut branching_events = Vec::new();
    let mut immediately_pre_map: BTreeMap<EventType, BTreeSet<EventType>> = BTreeMap::new();
    let (graph, initial, errors) = swarm_to_graph(&proto.protocol);
    if initial.is_none() || !errors.is_empty() {
        return ProtoInfo::new_only_proto(vec![ProtoStruct::new(
            graph,
            initial,
            errors,
            BTreeSet::new(),
        )]);
    }
    // If interface is some, then we want to interface this protocol
    // with some other protocol on this set of events.
    // We do not know if we can do that yet though, but we prepare as if we can.
    let interface = if proto.interface.is_some() {
        proto
            .interface
            .unwrap()
            .interfacing_event_types_single(&graph)
    } else {
        BTreeSet::new()
    };

    let mut walk = Dfs::new(&graph, initial.unwrap());

    // Add to set of branching and joining.
    // Graph contains no concurrency, so: 
    //      Branching event types are all outgoing event types if more than one and if more than one distinct target.
    //      Immediately preceding to each edge are all incoming event types 
    while let Some(node_id) = walk.next(&graph) { 
        let outgoing_labels: Vec<_> = graph.edges_directed(node_id, Outgoing).map(|edge| edge.weight()).collect();
        let incoming_event_types: BTreeSet<EventType> = graph.edges_directed(node_id, Incoming).map(|edge| edge.weight().get_event_type()).collect();
        
        if outgoing_labels.len() > 1 && direct_successors(&graph, node_id).len() > 1 {
            branching_events.push(outgoing_labels.iter().map(|edge| edge.get_event_type()).collect());
        }

        for label in outgoing_labels {
            role_event_map
                .entry(label.role.clone())
                .and_modify(|role_info| {
                    role_info.insert(label.clone());
                })
                .or_insert_with(|| BTreeSet::from([label.clone()]));
            
            immediately_pre_map
                .entry(label.get_event_type())
                .and_modify(|events| {
                    events.append(&mut incoming_event_types.clone());
                })
                .or_insert_with(|| incoming_event_types.clone());
        }
    }

    // consider changing after_not_concurrent to not take concurrent events as argument. now that we do not consider swarms with concurrency here.
    let happens_after = after_not_concurrent(&graph, initial.unwrap(), &BTreeSet::new());
    ProtoInfo::new(
        vec![ProtoStruct::new(graph, initial, errors, interface)],
        role_event_map,
        BTreeSet::new(),
        branching_events,
        BTreeSet::new(),
        immediately_pre_map,
        happens_after,
    )
}

// Set of direct successor nodes from node (those reachable in one step). 
fn direct_successors(graph: &Graph, node: NodeId) -> BTreeSet<NodeId> {
    graph.edges_directed(node, Outgoing).map(|e| e.target()).collect()
}

// turn a SwarmProtocol into a petgraph. perform some checks that are not strictly related to wwf, but must be successful for any further analysis to take place
fn swarm_to_graph(proto: &SwarmProtocolType) -> (Graph, Option<NodeId>, Vec<Error>) {
    let _span = tracing::info_span!("swarm_to_graph").entered();
    let mut graph = Graph::new();
    let mut errors = vec![];
    let mut nodes = BTreeMap::new();

    for t in &proto.transitions {
        let source = *nodes
            .entry(t.source.clone())
            .or_insert_with(|| graph.add_node(t.source.clone()));
        let target = *nodes
            .entry(t.target.clone())
            .or_insert_with(|| graph.add_node(t.target.clone()));
        let edge = graph.add_edge(source, target, t.label.clone());
        if t.label.log_type.len() == 0 {
            errors.push(Error::SwarmError(crate::swarm::Error::LogTypeEmpty(edge)));
        } else if t.label.log_type.len() > 1 {
            errors.push(Error::MoreThanOneEventTypeInCommand(edge)) // Come back here and implement splitting command into multiple 'artificial' ones emitting one event type if time instead of reporting it as an error.
        }
    }

    let initial = if let Some(idx) = nodes.get(&proto.initial) {
        errors.append(&mut all_nodes_reachable(&graph, *idx));
        Some(*idx)
    } else {
        // strictly speaking we have all_nodes_reachable errors here too...
        // if there is only an initial state no transitions whatsoever then thats ok? but gives an error here.
        errors.push(Error::SwarmError(
            crate::swarm::Error::InitialStateDisconnected,
        ));
        None
    };
    (graph, initial, errors)
}

pub fn from_json(proto: SwarmProtocolType) -> (Graph, Option<NodeId>, Vec<String>) {
    let _span = tracing::info_span!("from_json").entered();
    let proto_info = prepare_proto_info::<Role>(CompositionComponent {
        protocol: proto,
        interface: None,
    });
    let (g, i, e) = match proto_info.get_ith_proto(0) {
        Some(ProtoStruct {
            graph: g,
            initial: i,
            errors: e,
            interface: _,
        }) => (g, i, e),
        _ => return (Graph::new(), None, vec![]),
    };
    let e = e.map(Error::convert(&g));
    (g, i, e)
}

pub fn proto_info_to_error_report(proto_info: ProtoInfo) -> ErrorReport {
    let _span = tracing::info_span!("proto_info_to_error_report").entered();
    ErrorReport(
        proto_info
            .protocols
            .into_iter()
            .map(|p| (p.graph, p.errors))
            .collect(),
    )
}

// copied from swarm::swarm.rs
fn all_nodes_reachable(graph: &Graph, initial: NodeId) -> Vec<Error> {
    let _span = tracing::info_span!("all_nodes_reachable").entered();
    // Traversal order choice (Bfs vs Dfs vs DfsPostOrder) does not matter
    let visited = Dfs::new(&graph, initial)
        .iter(&graph)
        .collect::<BTreeSet<_>>();

    graph
        .node_indices()
        .filter(|node| !visited.contains(node))
        .map(|node| Error::SwarmError(crate::swarm::Error::StateUnreachable(node)))
        .collect()
}

// Check that every node of a graph can reach a node with no outgoing transitions.
fn all_nodes_reach_zero(graph: &Graph) -> Vec<Error> {
    let _span = tracing::info_span!("all_nodes_reach_zero").entered();
    // All terminal nodes
    let zero_nodes: Vec<_> = graph
        .node_indices()
        .filter(|node| graph.edges_directed(*node, Outgoing).count() == 0)
        .collect();
    // Reversed adaptor -- all edges have the opposite direction.
    let reversed = Reversed(&graph);
    
    // Collect all predecessors of from node using reversed adaptor.
    let get_predecessors = |node: NodeId| -> BTreeSet<NodeId> {
        let mut predecessors = BTreeSet::new();
        let mut dfs = Dfs::new(&reversed, node);
        while let Some(predecessor) = dfs.next(&reversed) {
            predecessors.insert(predecessor);
        }
        predecessors
    };

    // Collect all nodes that can reach a terminal node. 
    let can_reach_zero_nodes: BTreeSet<_> = zero_nodes
        .into_iter()
        .map(get_predecessors)
        .flatten()
        .collect();

    // Collect nodes that can not reach a terminal node and transform to a vec of errors.
    graph
        .node_indices()
        .into_iter()
        .filter(|node| !can_reach_zero_nodes.contains(node))
        .map(|node| Error::StateCanNotReachTerminal(node))
        .collect()
}

// all pairs of incoming/outgoing events from a node
fn event_pairs_from_node(
    node: NodeId,
    graph: &crate::Graph,
    direction: Direction,
) -> Vec<BTreeSet<EventType>> {
    graph
        .edges_directed(node, direction)
        .map(|e| e.id())
        .combinations(2)
        .map(|pair| {
            unord_event_pair(
                graph[pair[0]].get_event_type(),
                graph[pair[1]].get_event_type(),
            )
        }) //BTreeSet::from([graph[pair[0]].get_event_type(), graph[pair[1]].get_event_type()]))
        .collect()
}

// combine maps with sets as values
fn combine_maps<K: Ord + Clone, V: Ord + Clone>(
    map1: BTreeMap<K, BTreeSet<V>>,
    map2: BTreeMap<K, BTreeSet<V>>,
    extra: Option<BTreeSet<V>>,
) -> BTreeMap<K, BTreeSet<V>> {
    let all_keys: BTreeSet<K> = map1.keys().chain(map2.keys()).cloned().collect();
    let extra = extra.unwrap_or(BTreeSet::new());
    let extend_for_key = |k: &K| -> (K, BTreeSet<V>) {
        (
            k.clone(),
            map1.get(k)
                .unwrap_or(&BTreeSet::new())
                .union(map2.get(k).unwrap_or(&BTreeSet::new()))
                .chain(&extra)
                .cloned()
                .collect(),
        )
    };

    all_keys.iter().map(extend_for_key).collect()
}

// overapproximate concurrent events. anything from different protocols that are not interfacing events is considered concurrent.
fn get_concurrent_events<T: SwarmInterface>(
    proto_info1: &ProtoInfo,
    proto_info2: &ProtoInfo,
    interface: &T,
) -> BTreeSet<UnordEventPair> {
    let _span = tracing::info_span!("get_concurrent_events").entered();
    let concurrent_events_union: BTreeSet<UnordEventPair> = proto_info1
        .concurrent_events
        .union(&proto_info2.concurrent_events)
        .cloned()
        .collect();
    let interfacing_events = interface.interfacing_event_types(proto_info1, &proto_info2);
    let events_proto1: BTreeSet<EventType> = proto_info1
        .get_event_types()
        .difference(&interfacing_events)
        .cloned()
        .collect();
    let events_proto2: BTreeSet<EventType> = proto_info2
        .get_event_types()
        .difference(&interfacing_events)
        .cloned()
        .collect();
    let cartesian_product = events_proto1
        .into_iter()
        .cartesian_product(&events_proto2)
        .map(|(a, b)| unord_event_pair(a, b.clone()))
        .collect();

    concurrent_events_union
        .union(&cartesian_product)
        .cloned()
        .collect()
}

fn explicit_composition_proto_info(proto_info: ProtoInfo) -> ProtoInfo {
    let _span = tracing::info_span!("explicit_composition_proto_info").entered();
    let (composed, composed_initial) = explicit_composition(&proto_info);
    let succeeding_events =
        after_not_concurrent(&composed, composed_initial, &proto_info.concurrent_events);
    ProtoInfo {
        protocols: vec![ProtoStruct::new(
            composed,
            Some(composed_initial),
            vec![],
            BTreeSet::new(),
        )],
        succeeding_events,
        ..proto_info
    }
}

// precondition: the protocols can interface on the given interfaces
fn explicit_composition(proto_info: &ProtoInfo) -> (Graph, NodeId) {
    let _span = tracing::info_span!("explicit_composition").entered();
    if proto_info.protocols.is_empty() {
        return (Graph::new(), NodeId::end());
    }

    let (g, i, _) = proto_info.protocols[0].get_triple();
    let folder = |(acc_g, acc_i): (Graph, NodeId), p: ProtoStruct| -> (Graph, NodeId) {
        crate::composition::composition_machine::compose(
            acc_g,
            acc_i,
            p.graph,
            p.initial.unwrap(),
            p.interface,
            crate::composition::composition_machine::gen_state_name,
        )
    };
    proto_info.protocols[1..]
        .to_vec()
        .into_iter()
        .fold((g, i.unwrap()), folder)
}

pub fn to_swarm_json(graph: crate::Graph, initial: NodeId) -> SwarmProtocolType {
    let _span = tracing::info_span!("to_swarm_json").entered();
    let machine_label_mapper = |g: &crate::Graph, eref: EdgeReference<'_, SwarmLabel>| {
        let label = eref.weight().clone();
        let source = g[eref.source()].state_name().clone();
        let target = g[eref.target()].state_name().clone();
        Transition {
            label,
            source,
            target,
        }
    };

    let transitions: Vec<_> = graph
        .edge_references()
        .map(|e| machine_label_mapper(&graph, e))
        .collect();

    SwarmProtocolType {
        initial: graph[initial].state_name().clone(),
        transitions,
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        composition::{composition_types::CompositionComponent, error_report_to_strings},
        types::Command,
        MapVec,
    };

    use super::*;
    use tracing_subscriber::{fmt, fmt::format::FmtSpan, EnvFilter};
    fn setup_logger() {
        fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
            .try_init()
            .ok();
    }

    // Example from coplaws slides
    fn get_proto1() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "request", "logType": ["partID"], "role": "T" } },
                    { "source": "1", "target": "2", "label": { "cmd": "get", "logType": ["pos"], "role": "FL" } },
                    { "source": "2", "target": "0", "label": { "cmd": "deliver", "logType": ["part"], "role": "T" } },
                    { "source": "0", "target": "3", "label": { "cmd": "close", "logType": ["time"], "role": "D" } }
                ]
            }"#,
        )
        .unwrap()
    }
    fn get_subs1() -> Subscriptions {
        serde_json::from_str::<Subscriptions>(
            r#"{
                "T": ["partID", "part", "pos", "time"],
                "FL": ["partID", "pos", "time"],
                "D": ["partID", "part", "time"]
            }"#,
        )
        .unwrap()
    }
    fn get_proto2() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "request", "logType": ["partID"], "role": "T" } },
                    { "source": "1", "target": "2", "label": { "cmd": "deliver", "logType": ["part"], "role": "T" } },
                    { "source": "2", "target": "3", "label": { "cmd": "build", "logType": ["car"], "role": "F" } }
                ]
            }"#,
        )
        .unwrap()
    }
    fn get_subs2() -> Subscriptions {
        serde_json::from_str::<Subscriptions>(
            r#"{
                "T": ["partID", "part"],
                "F": ["part", "car"]
            }"#,
        )
        .unwrap()
    }
    fn get_proto3() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "observe", "logType": ["report1"], "role": "TR" } },
                    { "source": "1", "target": "2", "label": { "cmd": "build", "logType": ["car"], "role": "F" } },
                    { "source": "2", "target": "3", "label": { "cmd": "test", "logType": ["report2"], "role": "TR" } },
                    { "source": "3", "target": "4", "label": { "cmd": "accept", "logType": ["ok"], "role": "QCR" } },
                    { "source": "3", "target": "4", "label": { "cmd": "reject", "logType": ["notOk"], "role": "QCR" } }
                ]
            }"#,
        )
        .unwrap()
    }
    fn get_subs3() -> Subscriptions {
        serde_json::from_str::<Subscriptions>(
            r#"{
                "F": ["car", "report1"],
                "TR": ["car", "report1", "report2"],
                "QCR": ["report2", "ok", "notOk"]
            }"#,
        )
        .unwrap()
    }
    fn get_proto31() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "observe1", "logType": ["report1"], "role": "QCR" } },
                    { "source": "1", "target": "2", "label": { "cmd": "observe2", "logType": ["report2"], "role": "QCR" } },
                    { "source": "2", "target": "3", "label": { "cmd": "build", "logType": ["car"], "role": "F" } },
                    { "source": "3", "target": "4", "label": { "cmd": "assess", "logType": ["report3"], "role": "QCR" } }
                ]
            }"#,
        )
        .unwrap()
    }
    fn get_proto32() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "observe", "logType": ["observing"], "role": "QCR" } },
                    { "source": "1", "target": "2", "label": { "cmd": "build", "logType": ["car"], "role": "F" } },
                    { "source": "2", "target": "3", "label": { "cmd": "test", "logType": ["report"], "role": "QCR" } }
                ]
            }"#,
        )
        .unwrap()
    }

    fn get_subs_composition_1() -> Subscriptions {
        serde_json::from_str::<Subscriptions>(
            r#"{
                "T": ["partID", "part", "pos", "time"],
                "FL": ["partID", "pos", "time"],
                "D": ["partID", "part", "time"],
                "F": ["partID", "part", "car", "time"]
            }"#,
        )
        .unwrap()
    }

    // two event types in close, request appears multiple times, get emits no events
    fn get_malformed_proto1() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "request", "logType": ["partID"], "role": "T" } },
                    { "source": "1", "target": "2", "label": { "cmd": "get", "logType": [], "role": "FL" } },
                    { "source": "2", "target": "0", "label": { "cmd": "request", "logType": ["part"], "role": "T" } },
                    { "source": "0", "target": "0", "label": { "cmd": "close", "logType": ["time", "time2"], "role": "D" } }
                ]
            }"#,
        )
        .unwrap()
    }

    // initial state state unreachable
    fn get_malformed_proto2() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "1", "target": "2", "label": { "cmd": "get", "logType": ["pos"], "role": "FL" } },
                    { "source": "2", "target": "3", "label": { "cmd": "deliver", "logType": ["partID"], "role": "T" } }
                ]
            }"#,
        )
        .unwrap()
    }

    // all states not reachable
    fn get_malformed_proto3() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "request", "logType": ["partID"], "role": "T" } },
                    { "source": "2", "target": "3", "label": { "cmd": "deliver", "logType": ["part"], "role": "T" } },
                    { "source": "4", "target": "5", "label": { "cmd": "build", "logType": ["car"], "role": "F" } }
                ]
            }"#,
        )
        .unwrap()
    }

    // pos event type associated with multiple commands and nondeterminism at 0. 
    // No terminal state can be reached from any state -- OK according to confusion freeness, but not according to our
    // stricter-than-necessary checks
    fn get_confusionful_proto1() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "request", "logType": ["partID"], "role": "T" } },
                    { "source": "0", "target": "0", "label": { "cmd": "request", "logType": ["partID"], "role": "T" } },
                    { "source": "1", "target": "2", "label": { "cmd": "get", "logType": ["pos"], "role": "FL" } },
                    { "source": "2", "target": "0", "label": { "cmd": "request", "logType": ["pos"], "role": "T" } },
                    { "source": "0", "target": "0", "label": { "cmd": "close", "logType": ["time"], "role": "D" } }
                ]
            }"#,
        )
        .unwrap()
    }
    // No terminal state can be reached from any state -- OK according to confusion freeness, but not according to our
    // stricter-than-necessary checks
    fn get_some_nonterminating_proto() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "a", "logType": ["a"], "role": "a" } },
                    { "source": "0", "target": "2", "label": { "cmd": "c", "logType": ["c"], "role": "c" } },
                    { "source": "2", "target": "3", "label": { "cmd": "b", "logType": ["b"], "role": "b" } },
                    { "source": "1", "target": "4", "label": { "cmd": "d", "logType": ["d"], "role": "d" } },
                    { "source": "4", "target": "5", "label": { "cmd": "e", "logType": ["e"], "role": "e" } },
                    { "source": "5", "target": "1", "label": { "cmd": "f", "logType": ["f"], "role": "f" } }
                ]
            }"#,
        )
        .unwrap()
    }
    fn get_fail_1_component_1() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"
            {
                "initial": "456",
                "transitions": [
                    {
                    "label": {
                        "cmd": "R453_cmd_0",
                        "logType": [
                        "R453_e_0"
                        ],
                        "role": "R453"
                    },
                    "source": "456",
                    "target": "457"
                    },
                    {
                    "label": {
                        "cmd": "R454_cmd_0",
                        "logType": [
                        "R454_e_0"
                        ],
                        "role": "R454"
                    },
                    "source": "457",
                    "target": "458"
                    }
                ]
                }
            "#,
        )
        .unwrap()
    }

    fn get_fail_1_component_2() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"
            {
                "initial": "459",
                "transitions": [
                    {
                    "label": {
                        "cmd": "R455_cmd_0",
                        "logType": [
                        "R455_e_0"
                        ],
                        "role": "R455"
                    },
                    "source": "459",
                    "target": "460"
                    },
                    {
                    "label": {
                        "cmd": "R455_cmd_1",
                        "logType": [
                        "R455_e_1"
                        ],
                        "role": "R455"
                    },
                    "source": "460",
                    "target": "459"
                    },
                    {
                    "label": {
                        "cmd": "R454_cmd_0",
                        "logType": [
                        "R454_e_0"
                        ],
                        "role": "R454"
                    },
                    "source": "459",
                    "target": "461"
                    }
                ]
            }
            "#,
        )
        .unwrap()
    }

    fn pattern_4_proto_0() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "c_r0", "logType": ["e_r0"], "role": "R0" } },
                    { "source": "1", "target": "2", "label": { "cmd": "c_ir", "logType": ["e_ir"], "role": "IR" } }
                ]
            }"#,
        )
        .unwrap()
    }
    fn pattern_4_proto_1() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "c_r1", "logType": ["e_r1"], "role": "R1" } },
                    { "source": "1", "target": "2", "label": { "cmd": "c_ir", "logType": ["e_ir"], "role": "IR" } }
                ]
            }"#,
        )
        .unwrap()
    }
    fn pattern_4_proto_2() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "c_r2", "logType": ["e_r2"], "role": "R2" } },
                    { "source": "1", "target": "2", "label": { "cmd": "c_ir", "logType": ["e_ir"], "role": "IR" } }
                ]
            }"#,
        )
        .unwrap()
    }
    fn pattern_4_proto_3() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "c_r3", "logType": ["e_r3"], "role": "R3" } },
                    { "source": "1", "target": "2", "label": { "cmd": "c_ir", "logType": ["e_ir"], "role": "IR" } }
                ]
            }"#,
        )
        .unwrap()
    }
    fn pattern_4_proto_4() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "c_r4", "logType": ["e_r4"], "role": "R4" } },
                    { "source": "1", "target": "2", "label": { "cmd": "c_ir", "logType": ["e_ir"], "role": "IR" } }
                ]
            }"#,
        )
        .unwrap()
    }

    fn diff_example_proto_0() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "c_ir_0", "logType": ["e_ir_0"], "role": "IR" } },
                    { "source": "1", "target": "2", "label": { "cmd": "c_ir_1", "logType": ["e_ir_1"], "role": "IR" } },
                    { "source": "2", "target": "1", "label": { "cmd": "c_r0_0", "logType": ["e_r0_0"], "role": "R0" } },
                    { "source": "1", "target": "3", "label": { "cmd": "c_r0_1", "logType": ["e_r0_1"], "role": "R0" } }
                ]
            }"#,
        )
        .unwrap()
    }
    fn diff_example_proto_1() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "c_ir_0", "logType": ["e_ir_0"], "role": "IR" } },
                    { "source": "1", "target": "2", "label": { "cmd": "c_r1_0", "logType": ["e_r1_0"], "role": "R1" } },
                    { "source": "2", "target": "3", "label": { "cmd": "c_ir_1", "logType": ["e_ir_1"], "role": "IR" } }
                ]
            }"#,
        )
        .unwrap()
    }
    fn ref_pat_proto_0() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "c_ir0_0", "logType": ["e_ir0_0"], "role": "IR0" } },
                    { "source": "1", "target": "2", "label": { "cmd": "c_ir0_1", "logType": ["e_ir0_1"], "role": "IR0" } }
                ]
            }"#,
        )
        .unwrap()
    }
    fn ref_pat_proto_1() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "c_ir0_0", "logType": ["e_ir0_0"], "role": "IR0" } },
                    { "source": "1", "target": "2", "label": { "cmd": "c_ir1_0", "logType": ["e_ir1_0"], "role": "IR1" } },
                    { "source": "2", "target": "3", "label": { "cmd": "c_ir1_1", "logType": ["e_ir1_1"], "role": "IR1" } },
                    { "source": "3", "target": "4", "label": { "cmd": "c_rb", "logType": ["e_rb"], "role": "RB" } },
                    { "source": "4", "target": "5", "label": { "cmd": "c_ir0_1", "logType": ["e_ir0_1"], "role": "IR0" } },
                    { "source": "1", "target": "6", "label": { "cmd": "c_ra", "logType": ["e_ra"], "role": "RA" } }
                ]
            }"#,
        )
        .unwrap()
    }
    fn ref_pat_proto_2() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "c_ir1_0", "logType": ["e_ir1_0"], "role": "IR1" } },
                    { "source": "1", "target": "2", "label": { "cmd": "c_rc", "logType": ["e_rc"], "role": "RC" } },
                    { "source": "2", "target": "3", "label": { "cmd": "c_ir1_1", "logType": ["e_ir1_1"], "role": "IR1" } }
                ]
            }"#,
        )
        .unwrap()
    }

    fn get_interfacing_swarms_diff_example() -> InterfacingSwarms<Role> {
        InterfacingSwarms(vec![
            CompositionComponent {
                protocol: diff_example_proto_0(),
                interface: None,
            },
            CompositionComponent {
                protocol: diff_example_proto_1(),
                interface: Some(Role::new("IR")),
            },
        ])
    }
    fn get_ref_pat_protos() -> InterfacingSwarms<Role> {
        InterfacingSwarms(vec![
            CompositionComponent {
                protocol: ref_pat_proto_0(),
                interface: None,
            },
            CompositionComponent {
                protocol: ref_pat_proto_1(),
                interface: Some(Role::new("IR0")),
            },
            CompositionComponent {
                protocol: ref_pat_proto_2(),
                interface: Some(Role::new("IR1")),
            },
        ])
    }
    fn get_interfacing_swarms_1() -> InterfacingSwarms<Role> {
        InterfacingSwarms(vec![
            CompositionComponent {
                protocol: get_proto1(),
                interface: None,
            },
            CompositionComponent {
                protocol: get_proto2(),
                interface: Some(Role::new("T")),
            },
        ])
    }

    fn get_interfacing_swarms_2() -> InterfacingSwarms<Role> {
        InterfacingSwarms(vec![
            CompositionComponent {
                protocol: get_proto1(),
                interface: None,
            },
            CompositionComponent {
                protocol: get_proto2(),
                interface: Some(Role::new("T")),
            },
            CompositionComponent {
                protocol: get_proto3(),
                interface: Some(Role::new("F")),
            },
        ])
    }

    fn get_interfacing_swarms_3() -> InterfacingSwarms<Role> {
        InterfacingSwarms(vec![
            CompositionComponent {
                protocol: get_proto1(),
                interface: None,
            },
            CompositionComponent {
                protocol: get_proto2(),
                interface: Some(Role::new("T")),
            },
            CompositionComponent {
                protocol: get_proto31(),
                interface: Some(Role::new("F")),
            },
        ])
    }

    fn get_interfacing_swarms_4() -> InterfacingSwarms<Role> {
        InterfacingSwarms(vec![
            CompositionComponent {
                protocol: get_proto1(),
                interface: None,
            },
            CompositionComponent {
                protocol: get_proto2(),
                interface: Some(Role::new("T")),
            },
            CompositionComponent {
                protocol: get_proto3(),
                interface: Some(Role::new("F")),
            },
        ])
    }

    fn get_interfacing_swarms_5() -> InterfacingSwarms<Role> {
        InterfacingSwarms(vec![
            CompositionComponent {
                protocol: get_proto1(),
                interface: None,
            },
            CompositionComponent {
                protocol: get_proto2(),
                interface: Some(Role::new("T")),
            },
            CompositionComponent {
                protocol: get_proto32(),
                interface: Some(Role::new("F")),
            },
        ])
    }

    fn get_interfacing_swarms_pat_4() -> InterfacingSwarms<Role> {
        InterfacingSwarms(vec![
            CompositionComponent {
                protocol: pattern_4_proto_0(),
                interface: None,
            },
            CompositionComponent {
                protocol: pattern_4_proto_1(),
                interface: Some(Role::new("IR")),
            },
            CompositionComponent {
                protocol: pattern_4_proto_2(),
                interface: Some(Role::new("IR")),
            },
            CompositionComponent {
                protocol: pattern_4_proto_3(),
                interface: Some(Role::new("IR")),
            },
            CompositionComponent {
                protocol: pattern_4_proto_4(),
                interface: Some(Role::new("IR")),
            },
        ])
    }
    fn get_fail_1_swarms() -> InterfacingSwarms<Role> {
        InterfacingSwarms(vec![
            CompositionComponent {
                protocol: get_fail_1_component_1(),
                interface: None,
            },
            CompositionComponent {
                protocol: get_fail_1_component_2(),
                interface: Some(Role::new("R454")),
            },
        ])
    }

    // QCR subscribes to car and part because report1 is concurrent with part and they lead to a joining event car/event is joining bc of this.
    fn get_subs_composition_2() -> Subscriptions {
        serde_json::from_str::<Subscriptions>(
            r#"{
                "T": ["partID", "part", "pos", "time"],
                "FL": ["partID", "pos", "time"],
                "D": ["partID", "part", "time"],
                "F": ["partID", "part", "car", "time", "report1"],
                "TR": ["partID", "report1", "report2", "car", "time", "part"],
                "QCR": ["partID", "part", "report1", "report2", "car", "time", "ok", "notOk"]
            }"#,
        )
        .unwrap()
    }

    // true if subs1 is a subset of subs2
    fn is_sub_subscription(subs1: Subscriptions, subs2: Subscriptions) -> bool {
        if !subs1
            .keys()
            .cloned()
            .collect::<BTreeSet<Role>>()
            .is_subset(&subs2.keys().cloned().collect::<BTreeSet<Role>>())
        {
            return false;
        }

        for role in subs1.keys() {
            if !subs1[role].is_subset(&subs2[role]) {
                return false;
            }
        }

        true
    }

    #[test]
    fn test_prepare_graph_confusionfree() {
        setup_logger();
        let composition = get_interfacing_swarms_1();
        let proto_info = combine_proto_infos_fold(prepare_proto_infos::<Role>(composition));
        let proto_info = explicit_composition_proto_info(proto_info);

        assert!(proto_info.get_ith_proto(0).is_some());
        assert!(proto_info.get_ith_proto(0).unwrap().errors.is_empty());
        assert_eq!(
            proto_info.concurrent_events,
            BTreeSet::from([
                unord_event_pair(EventType::new("time"), EventType::new("car")),
                unord_event_pair(EventType::new("pos"), EventType::new("car"))
            ])
        );
        assert_eq!(
            proto_info.branching_events,
            vec![BTreeSet::from([
                EventType::new("time"),
                EventType::new("partID")
            ])]
        );
        assert_eq!(
            proto_info.joining_events,
            BTreeSet::from([EventType::new("part"), EventType::new("partID")])
        );
        let expected_role_event_map = BTreeMap::from([
            (
                Role::from("T"),
                BTreeSet::from([
                    SwarmLabel {
                        cmd: Command::new("deliver"),
                        log_type: vec![EventType::new("part")],
                        role: Role::new("T"),
                    },
                    SwarmLabel {
                        cmd: Command::new("request"),
                        log_type: vec![EventType::new("partID")],
                        role: Role::new("T"),
                    },
                ]),
            ),
            (
                Role::from("FL"),
                BTreeSet::from([SwarmLabel {
                    cmd: Command::new("get"),
                    log_type: vec![EventType::new("pos")],
                    role: Role::new("FL"),
                }]),
            ),
            (
                Role::from("D"),
                BTreeSet::from([SwarmLabel {
                    cmd: Command::new("close"),
                    log_type: vec![EventType::new("time")],
                    role: Role::new("D"),
                }]),
            ),
            (
                Role::from("F"),
                BTreeSet::from([SwarmLabel {
                    cmd: Command::new("build"),
                    log_type: vec![EventType::new("car")],
                    role: Role::new("F"),
                }]),
            ),
        ]);
        assert_eq!(proto_info.role_event_map, expected_role_event_map);
        let proto_info = prepare_proto_info::<Role>(CompositionComponent {
            protocol: get_proto1(),
            interface: None,
        });
        assert!(proto_info.get_ith_proto(0).is_some());
        assert!(proto_info.get_ith_proto(0).unwrap().errors.is_empty());
        assert_eq!(proto_info.concurrent_events, BTreeSet::new());
        assert_eq!(
            proto_info.branching_events,
            vec![BTreeSet::from([
                EventType::new("time"),
                EventType::new("partID")
            ])]
        );
        assert_eq!(proto_info.joining_events, BTreeSet::new());

        let proto_info = prepare_proto_info::<Role>(CompositionComponent {
            protocol: get_proto2(),
            interface: None,
        }); //get_proto2(), None);
        assert!(proto_info.get_ith_proto(0).is_some());
        assert!(proto_info.get_ith_proto(0).unwrap().errors.is_empty());
        assert_eq!(proto_info.concurrent_events, BTreeSet::new());
        assert_eq!(proto_info.branching_events, Vec::new());
        assert_eq!(proto_info.joining_events, BTreeSet::new());

        let proto_info = prepare_proto_info::<Role>(CompositionComponent {
            protocol: get_proto3(),
            interface: None,
        }); //get_proto2(), None);//get_proto3(), None);
        assert!(proto_info.get_ith_proto(0).is_some());
        assert!(proto_info.get_ith_proto(0).unwrap().errors.is_empty());
        assert_eq!(proto_info.concurrent_events, BTreeSet::new());
        
        // Should not contain any branching event types since only state with two outgoing is 3 
        // and both of these outgoing transitions go to state 4:
        // { "source": "3", "target": "4", "label": { "cmd": "accept", "logType": ["ok"], "role": "QCR" } },
        // { "source": "3", "target": "4", "label": { "cmd": "reject", "logType": ["notOk"], "role": "QCR" } }
        assert_eq!(
            proto_info.branching_events,
            vec![]
        );
        assert_eq!(proto_info.joining_events, BTreeSet::new());
    }

    #[test]
    fn test_prepare_graph_malformed() {
        setup_logger();
        let proto1 = get_malformed_proto1();
        let proto_info = prepare_proto_info::<Role>(CompositionComponent {
            protocol: proto1.clone(),
            interface: None,
        }); //proto1.clone(), None);
        let mut errors = vec![proto_info.get_ith_proto(0).unwrap().errors]
            .concat()
            .map(Error::convert(&proto_info.get_ith_proto(0).unwrap().graph));

        let mut expected_erros = vec![
            "transition (0)--[close@D<time,time2>]-->(0) emits more than one event type",
            "log type must not be empty (1)--[get@FL<>]-->(2)",
        ];
        errors.sort();
        expected_erros.sort();
        assert_eq!(errors, expected_erros);

        let proto_info = prepare_proto_info::<Role>(CompositionComponent {
            protocol: get_malformed_proto2(),
            interface: None,
        }); //get_malformed_proto2(), None);
        let errors = vec![
            confusion_free(&proto_info, 0),
            proto_info.get_ith_proto(0).unwrap().errors,
        ]
        .concat()
        .map(Error::convert(&proto_info.get_ith_proto(0).unwrap().graph));

        let expected_errors = vec![
            "initial swarm protocol state has no transitions",
            "initial swarm protocol state has no transitions",
        ];
        assert_eq!(errors, expected_errors);

        let proto_info = prepare_proto_info::<Role>(CompositionComponent {
            protocol: get_malformed_proto3(),
            interface: None,
        }); //get_malformed_proto3(), None);
        let errors = proto_info
            .get_ith_proto(0)
            .unwrap()
            .errors
            .map(Error::convert(&proto_info.get_ith_proto(0).unwrap().graph));

        let expected_errors = vec![
            "state 2 is unreachable from initial state",
            "state 3 is unreachable from initial state",
            "state 4 is unreachable from initial state",
            "state 5 is unreachable from initial state",
        ];
        assert_eq!(errors, expected_errors);
    }

    // pos event type associated with multiple commands and nondeterminism at 0
    #[test]
    fn test_prepare_graph_confusionful() {
        setup_logger();
        let proto = get_confusionful_proto1();

        let proto_info = prepare_proto_info::<Role>(CompositionComponent {
            protocol: proto,
            interface: None,
        }); //proto, None);
        let mut errors = vec![
            confusion_free(&proto_info, 0),
            proto_info.get_ith_proto(0).unwrap().errors,
        ]
        .concat()
        .map(Error::convert(&proto_info.get_ith_proto(0).unwrap().graph));

        let mut expected_errors = vec![
            "command request enabled in more than one transition: (0)--[request@T<partID>]-->(1), (0)--[request@T<partID>]-->(0), (2)--[request@T<pos>]-->(0)",
            "event type partID emitted in more than one transition: (0)--[request@T<partID>]-->(1), (0)--[request@T<partID>]-->(0)",
            "event type pos emitted in more than one transition: (1)--[get@FL<pos>]-->(2), (2)--[request@T<pos>]-->(0)",
            "state 0 can not reach terminal node",
            "state 1 can not reach terminal node",
            "state 2 can not reach terminal node",
        ];
        /* let mut expected_errors = vec![
                "command request enabled in more than one transition: (0)--[request@T<partID>]-->(1), (0)--[request@T<partID>]-->(0), (2)--[request@T<pos>]-->(0)",
                "event type partID emitted in more than one transition: (0)--[request@T<partID>]-->(1), (0)--[request@T<partID>]-->(0)",
                "event type pos emitted in more than one transition: (1)--[get@FL<pos>]-->(2), (2)--[request@T<pos>]-->(0)",
            ]; */
        errors.sort();
        expected_errors.sort();
        assert_eq!(errors, expected_errors);
    
        let proto = get_some_nonterminating_proto();
        let proto_info = prepare_proto_info::<Role>(CompositionComponent {
            protocol: proto,
            interface: None,
        }); //proto, None);
        let mut errors = vec![
            confusion_free(&proto_info, 0),
            proto_info.get_ith_proto(0).unwrap().errors,
        ]
        .concat()
        .map(Error::convert(&proto_info.get_ith_proto(0).unwrap().graph));

        let mut expected_errors = vec![
            "state 1 can not reach terminal node",
            "state 4 can not reach terminal node",
            "state 5 can not reach terminal node",
        ];
        errors.sort();
        expected_errors.sort();
        assert_eq!(errors, expected_errors);
    }

    #[test]
    fn test_wwf_ok() {
        setup_logger();
        let proto1: InterfacingSwarms<Role> = InterfacingSwarms(vec![CompositionComponent {
            protocol: get_proto1(),
            interface: None,
        }]);
        let result1 = exact_weak_well_formed_sub(proto1.clone(), &BTreeMap::new());
        assert!(result1.is_ok());
        let subs1 = result1.unwrap();
        let error_report = check(proto1, &subs1);
        assert!(error_report.is_empty());
        assert_eq!(get_subs1(), subs1);

        let proto2: InterfacingSwarms<Role> = InterfacingSwarms(vec![CompositionComponent {
            protocol: get_proto2(),
            interface: None,
        }]);
        let result2 = exact_weak_well_formed_sub(proto2.clone(), &BTreeMap::new());
        assert!(result2.is_ok());
        let subs2 = result2.unwrap();
        let error_report = check(proto2, &subs2);
        assert!(error_report.is_empty());
        assert_eq!(get_subs2(), subs2);

        let proto3: InterfacingSwarms<Role> = InterfacingSwarms(vec![CompositionComponent {
            protocol: get_proto3(),
            interface: None,
        }]);
        let result3 = exact_weak_well_formed_sub(proto3.clone(), &BTreeMap::new());
        assert!(result3.is_ok());
        let subs3 = result3.unwrap();
        let error_report = check(proto3, &subs3);
        assert!(error_report.is_empty());
        assert_eq!(get_subs3(), subs3);

        let composition1: InterfacingSwarms<Role> = get_interfacing_swarms_1();
        let result_composition1 =
            exact_weak_well_formed_sub(composition1.clone(), &BTreeMap::new());
        assert!(result_composition1.is_ok());
        let subs_composition = result_composition1.unwrap();
        let (g, i) = compose_protocols(composition1.clone()).unwrap();
        let thing = to_swarm_json(g, i);
        println!("{}", serde_json::to_string_pretty(&thing).unwrap());
        println!(
            "{}",
            serde_json::to_string_pretty(&subs_composition).unwrap()
        );
        let error_report = check(composition1, &subs_composition);
        assert!(error_report.is_empty());
        assert_eq!(get_subs_composition_1(), subs_composition);

        let composition2: InterfacingSwarms<Role> = get_interfacing_swarms_2();
        let result_composition2 =
            exact_weak_well_formed_sub(composition2.clone(), &BTreeMap::new());
        assert!(result_composition2.is_ok());
        let subs_composition = result_composition2.unwrap();
        let error_report = check(composition2, &subs_composition);
        assert!(error_report.is_empty());
        assert_eq!(get_subs_composition_2(), subs_composition);
    }

    #[test]
    fn test_wwf_fail() {
        setup_logger();
        let input: InterfacingSwarms<Role> = InterfacingSwarms(vec![CompositionComponent {
            protocol: get_proto1(),
            interface: None,
        }]);
        let subs = BTreeMap::from([
            (Role::new("T"), BTreeSet::from([EventType::new("pos")])),
            (Role::new("D"), BTreeSet::from([EventType::new("pos")])),
            (Role::new("FL"), BTreeSet::from([EventType::new("partID")])),
        ]);
        let error_report = check(input, &subs);
        let mut errors = error_report_to_strings(error_report);
        errors.sort();
        let mut expected_errors = vec![
            "active role does not subscribe to any of its emitted event types in transition (0)--[close@D<time>]-->(3)",
            "active role does not subscribe to any of its emitted event types in transition (0)--[request@T<partID>]-->(1)",
            "active role does not subscribe to any of its emitted event types in transition (2)--[deliver@T<part>]-->(0)",
            "active role does not subscribe to any of its emitted event types in transition (1)--[get@FL<pos>]-->(2)",
            "role T does not subscribe to event types partID, time in branching transitions at state 0, but is involved after transition (0)--[request@T<partID>]-->(1)",
            "role D does not subscribe to event types partID, time in branching transitions at state 0, but is involved after transition (0)--[request@T<partID>]-->(1)",
            "role FL does not subscribe to event types time in branching transitions at state 0, but is involved after transition (0)--[request@T<partID>]-->(1)",
            "subsequently active role D does not subscribe to events in transition (2)--[deliver@T<part>]-->(0)",
            "subsequently active role T does not subscribe to events in transition (2)--[deliver@T<part>]-->(0)",
        ];

        expected_errors.sort();
        assert_eq!(errors, expected_errors);

        let input: InterfacingSwarms<Role> = InterfacingSwarms(vec![CompositionComponent {
            protocol: get_proto2(),
            interface: None,
        }]);
        let error_report = check(input, &get_subs3());
        let mut errors = error_report_to_strings(error_report);
        errors.sort();
        let mut expected_errors = vec![
            "active role does not subscribe to any of its emitted event types in transition (0)--[request@T<partID>]-->(1)",
            "subsequently active role T does not subscribe to events in transition (0)--[request@T<partID>]-->(1)",
            "active role does not subscribe to any of its emitted event types in transition (1)--[deliver@T<part>]-->(2)",
            "subsequently active role F does not subscribe to events in transition (1)--[deliver@T<part>]-->(2)"
        ];

        expected_errors.sort();
        assert_eq!(errors, expected_errors);

        let input: InterfacingSwarms<Role> = InterfacingSwarms(vec![CompositionComponent {
            protocol: get_proto3(),
            interface: None,
        }]);
        let error_report = check(input, &get_subs1());
        let mut errors = error_report_to_strings(error_report);
        errors.sort();
        let mut expected_errors = vec![
            "active role does not subscribe to any of its emitted event types in transition (0)--[observe@TR<report1>]-->(1)",
            "active role does not subscribe to any of its emitted event types in transition (1)--[build@F<car>]-->(2)",
            "active role does not subscribe to any of its emitted event types in transition (2)--[test@TR<report2>]-->(3)",
            "active role does not subscribe to any of its emitted event types in transition (3)--[accept@QCR<ok>]-->(4)",
            "active role does not subscribe to any of its emitted event types in transition (3)--[reject@QCR<notOk>]-->(4)",
            "subsequently active role F does not subscribe to events in transition (0)--[observe@TR<report1>]-->(1)",
            "subsequently active role QCR does not subscribe to events in transition (2)--[test@TR<report2>]-->(3)",
            "subsequently active role QCR does not subscribe to events in transition (2)--[test@TR<report2>]-->(3)",
            "subsequently active role TR does not subscribe to events in transition (1)--[build@F<car>]-->(2)"
        ];

        expected_errors.sort();
        assert_eq!(errors, expected_errors);
    }

    #[test]
    fn test_weak_well_formed_sub() {
        setup_logger();
        let result = exact_weak_well_formed_sub(get_interfacing_swarms_1(), &BTreeMap::new());
        assert!(result.is_ok());
        let subs1 = result.unwrap();
        let error_report = check(get_interfacing_swarms_1(), &subs1);
        assert!(error_report.is_empty());

        let result = overapprox_weak_well_formed_sub(
            get_interfacing_swarms_1(),
            &BTreeMap::new(),
            Granularity::Coarse,
        );
        assert!(result.is_ok());
        let subs2 = result.unwrap();
        let error_report = check(get_interfacing_swarms_1(), &subs2);
        assert!(error_report.is_empty());
        assert!(is_sub_subscription(subs1, subs2));

        let result = exact_weak_well_formed_sub(get_interfacing_swarms_2(), &BTreeMap::new());
        assert!(result.is_ok());
        let subs1 = result.unwrap();
        println!("exact: {}", serde_json::to_string_pretty(&subs1).unwrap());
        let error_report = check(get_interfacing_swarms_2(), &subs1);
        assert!(error_report.is_empty());

        let result = overapprox_weak_well_formed_sub(
            get_interfacing_swarms_2(),
            &BTreeMap::new(),
            Granularity::Coarse,
        );
        assert!(result.is_ok());
        let subs2 = result.unwrap();
        println!("approx: {}", serde_json::to_string_pretty(&subs2).unwrap());
        let error_report = check(get_interfacing_swarms_2(), &subs2);
        assert!(error_report.is_empty());
        assert!(is_sub_subscription(subs1, subs2));

        let result = overapprox_weak_well_formed_sub(
            get_interfacing_swarms_1(),
            &BTreeMap::new(),
            Granularity::Medium,
        );
        assert!(result.is_ok());
        let subs2 = result.unwrap();
        let error_report = check(get_interfacing_swarms_1(), &subs2);
        assert!(error_report.is_empty());

        let result = overapprox_weak_well_formed_sub(
            get_interfacing_swarms_1(),
            &BTreeMap::new(),
            Granularity::Fine,
        );
        assert!(result.is_ok());
        let subs2 = result.unwrap();
        let error_report = check(get_interfacing_swarms_1(), &subs2);
        assert!(error_report.is_empty());

        let result = overapprox_weak_well_formed_sub(
            get_interfacing_swarms_1(),
            &BTreeMap::new(),
            Granularity::TwoStep,
        );
        assert!(result.is_ok());
        let subs2 = result.unwrap();
        let error_report = check(get_interfacing_swarms_1(), &subs2);
        assert!(error_report.is_empty());

        let result = overapprox_weak_well_formed_sub(
            get_interfacing_swarms_2(),
            &BTreeMap::new(),
            Granularity::Medium,
        );
        assert!(result.is_ok());
        let subs2 = result.unwrap();
        let error_report = check(get_interfacing_swarms_2(), &subs2);
        assert!(error_report.is_empty());

        let result = overapprox_weak_well_formed_sub(
            get_interfacing_swarms_2(),
            &BTreeMap::new(),
            Granularity::Fine,
        );
        assert!(result.is_ok());
        let subs2 = result.unwrap();
        let error_report = check(get_interfacing_swarms_2(), &subs2);
        assert!(error_report.is_empty());

        let result = overapprox_weak_well_formed_sub(
            get_interfacing_swarms_2(),
            &BTreeMap::new(),
            Granularity::TwoStep,
        );
        assert!(result.is_ok());
        let subs2 = result.unwrap();
        let error_report = check(get_interfacing_swarms_2(), &subs2);
        assert!(error_report.is_empty());
    }

    #[test]
    fn test_weak_well_formed_sub_1() {
        setup_logger();
        let result = exact_weak_well_formed_sub(get_interfacing_swarms_5(), &BTreeMap::new());
        assert!(result.is_ok());
        let subs1 = result.unwrap();
        let error_report = check(get_interfacing_swarms_5(), &subs1);
        assert!(error_report.is_empty());

        let result = overapprox_weak_well_formed_sub(
            get_interfacing_swarms_5(),
            &BTreeMap::new(),
            Granularity::Coarse,
        );
        assert!(result.is_ok());
        let subs2 = result.unwrap();
        let error_report = check(get_interfacing_swarms_5(), &subs2);
        assert!(error_report.is_empty());
        assert!(is_sub_subscription(subs1, subs2));

        let result = overapprox_weak_well_formed_sub(
            get_interfacing_swarms_5(),
            &BTreeMap::new(),
            Granularity::Medium,
        );
        assert!(result.is_ok());
        let subs2 = result.unwrap();
        let error_report = check(get_interfacing_swarms_5(), &subs2);
        assert!(error_report.is_empty());

        let result = overapprox_weak_well_formed_sub(
            get_interfacing_swarms_5(),
            &BTreeMap::new(),
            Granularity::Fine,
        );
        assert!(result.is_ok());
        let subs2 = result.unwrap();
        let error_report = check(get_interfacing_swarms_5(), &subs2);
        assert!(error_report.is_empty());

        let result = overapprox_weak_well_formed_sub(
            get_interfacing_swarms_5(),
            &BTreeMap::new(),
            Granularity::TwoStep,
        );
        assert!(result.is_ok());
        let subs2 = result.unwrap();
        let error_report = check(get_interfacing_swarms_5(), &subs2);
        assert!(error_report.is_empty());
    }

    #[test]
    fn test_extend_subs() {
        setup_logger();
        let sub_to_extend = BTreeMap::from([
            (Role::new("D"), BTreeSet::from([EventType::new("pos")])),
            (Role::new("TR"), BTreeSet::from([EventType::new("ok")])),
        ]);
        let result1 = exact_weak_well_formed_sub(get_interfacing_swarms_4(), &sub_to_extend);
        let result2 = overapprox_weak_well_formed_sub(
            get_interfacing_swarms_4(),
            &sub_to_extend,
            Granularity::Coarse,
        );
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        let subs1 = result1.unwrap();
        let subs2 = result2.unwrap();
        println!("exact: {}", serde_json::to_string_pretty(&subs1).unwrap());
        println!("approx: {}", serde_json::to_string_pretty(&subs2).unwrap());
        assert!(check(get_interfacing_swarms_4(), &subs1).is_empty());
        assert!(check(get_interfacing_swarms_4(), &subs2).is_empty());
        assert!(subs1[&Role::new("D")].contains(&EventType::new("pos")));
        assert!(subs2[&Role::new("D")].contains(&EventType::new("pos")));
        assert!(subs1[&Role::new("TR")].contains(&EventType::new("ok")));
        assert!(subs2[&Role::new("TR")].contains(&EventType::new("ok")));

        let result2 = overapprox_weak_well_formed_sub(
            get_interfacing_swarms_4(),
            &sub_to_extend,
            Granularity::Medium,
        );
        assert!(result2.is_ok());
        let subs2 = result2.unwrap();
        println!("exact: {}", serde_json::to_string_pretty(&subs1).unwrap());
        println!("approx: {}", serde_json::to_string_pretty(&subs2).unwrap());
        assert!(check(get_interfacing_swarms_4(), &subs2).is_empty());
        assert!(subs2[&Role::new("D")].contains(&EventType::new("pos")));
        assert!(subs2[&Role::new("TR")].contains(&EventType::new("ok")));

        let result2 = overapprox_weak_well_formed_sub(
            get_interfacing_swarms_4(),
            &sub_to_extend,
            Granularity::Fine,
        );
        assert!(result2.is_ok());
        let subs2 = result2.unwrap();
        println!("exact: {}", serde_json::to_string_pretty(&subs1).unwrap());
        println!("approx: {}", serde_json::to_string_pretty(&subs2).unwrap());
        assert!(check(get_interfacing_swarms_4(), &subs2).is_empty());
        assert!(subs2[&Role::new("D")].contains(&EventType::new("pos")));
        assert!(subs2[&Role::new("TR")].contains(&EventType::new("ok")));
    }

    #[test]
    fn test_compose_non_wwf_swarms() {
        setup_logger();
        let input = get_interfacing_swarms_1();
        let subs = BTreeMap::from([
            (Role::new("T"), BTreeSet::from([EventType::new("part")])),
            (Role::new("D"), BTreeSet::from([EventType::new("part")])),
            (Role::new("FL"), BTreeSet::from([EventType::new("part")])),
            (Role::new("F"), BTreeSet::from([EventType::new("part")])),
        ]);
        let error_report = check(input, &subs);
        let mut errors = error_report_to_strings(error_report);
        errors.sort();
        let mut expected_errors = vec![
            "active role does not subscribe to any of its emitted event types in transition (0 || 0)--[request@T<partID>]-->(1 || 1)",
            "active role does not subscribe to any of its emitted event types in transition (0 || 0)--[close@D<time>]-->(3 || 0)",
            "active role does not subscribe to any of its emitted event types in transition (1 || 1)--[get@FL<pos>]-->(2 || 1)",
            "active role does not subscribe to any of its emitted event types in transition (0 || 2)--[build@F<car>]-->(0 || 3)",
            "active role does not subscribe to any of its emitted event types in transition (0 || 3)--[close@D<time>]-->(3 || 3)",
            "active role does not subscribe to any of its emitted event types in transition (0 || 2)--[close@D<time>]-->(3 || 2)",
            "active role does not subscribe to any of its emitted event types in transition (3 || 2)--[build@F<car>]-->(3 || 3)",
            "role D does not subscribe to event types partID, time in branching transitions at state 0 || 0, but is involved after transition (0 || 0)--[request@T<partID>]-->(1 || 1)",
            "role T does not subscribe to event types partID, time in branching transitions at state 0 || 0, but is involved after transition (0 || 0)--[request@T<partID>]-->(1 || 1)",
            "role FL does not subscribe to event types partID, time in branching transitions at state 0 || 0, but is involved after transition (0 || 0)--[request@T<partID>]-->(1 || 1)",
            "role F does not subscribe to event types partID, time in branching transitions at state 0 || 0, but is involved after transition (0 || 0)--[request@T<partID>]-->(1 || 1)",
            "subsequently active role FL does not subscribe to events in transition (0 || 0)--[request@T<partID>]-->(1 || 1)",
            "subsequently active role T does not subscribe to events in transition (1 || 1)--[get@FL<pos>]-->(2 || 1)",
        ];
        expected_errors.sort();
        assert_eq!(errors, expected_errors);
    }

    #[test]
    fn test_fail1() {
        setup_logger();
        let result = exact_weak_well_formed_sub(get_fail_1_swarms(), &BTreeMap::new());
        assert!(result.is_ok());
        let subs1 = result.unwrap();
        let error_report = check(get_fail_1_swarms(), &subs1);
        assert!(error_report.is_empty());

        let error_report = check(get_fail_1_swarms(), &BTreeMap::new());
        let mut errors = error_report_to_strings(error_report);
        errors.sort();
        let mut expected_errors = vec![
            "active role does not subscribe to any of its emitted event types in transition (456 || 459)--[R455_cmd_0@R455<R455_e_0>]-->(456 || 460)",
            "subsequently active role R455 does not subscribe to events in transition (456 || 459)--[R455_cmd_0@R455<R455_e_0>]-->(456 || 460)",
            "active role does not subscribe to any of its emitted event types in transition (456 || 459)--[R453_cmd_0@R453<R453_e_0>]-->(457 || 459)",
            "subsequently active role R454 does not subscribe to events in transition (456 || 459)--[R453_cmd_0@R453<R453_e_0>]-->(457 || 459)",
            "active role does not subscribe to any of its emitted event types in transition (457 || 459)--[R454_cmd_0@R454<R454_e_0>]-->(458 || 461)",
            "active role does not subscribe to any of its emitted event types in transition (457 || 459)--[R455_cmd_0@R455<R455_e_0>]-->(457 || 460)",
            "subsequently active role R455 does not subscribe to events in transition (457 || 459)--[R455_cmd_0@R455<R455_e_0>]-->(457 || 460)",
            "active role does not subscribe to any of its emitted event types in transition (457 || 460)--[R455_cmd_1@R455<R455_e_1>]-->(457 || 459)",
            "subsequently active role R454 does not subscribe to events in transition (457 || 460)--[R455_cmd_1@R455<R455_e_1>]-->(457 || 459)",
            "subsequently active role R455 does not subscribe to events in transition (457 || 460)--[R455_cmd_1@R455<R455_e_1>]-->(457 || 459)",
            "active role does not subscribe to any of its emitted event types in transition (456 || 460)--[R455_cmd_1@R455<R455_e_1>]-->(456 || 459)",
            "subsequently active role R455 does not subscribe to events in transition (456 || 460)--[R455_cmd_1@R455<R455_e_1>]-->(456 || 459)",
            "active role does not subscribe to any of its emitted event types in transition (456 || 460)--[R453_cmd_0@R453<R453_e_0>]-->(457 || 460)"
        ];
        expected_errors.sort();
        assert_eq!(errors, expected_errors);
    }

    #[test]
    fn test_join_errors() {
        setup_logger();
        let composition: InterfacingSwarms<Role> = get_interfacing_swarms_2();
        let result_composition = exact_weak_well_formed_sub(composition.clone(), &BTreeMap::new());
        assert!(result_composition.is_ok());
        let mut subs_composition = result_composition.unwrap();
        subs_composition.entry(Role::new("QCR")).and_modify(|s| {
            *s = BTreeSet::from([
                EventType::new("report2"),
                EventType::new("ok"),
                EventType::new("notOk"),
                EventType::new("partID"),
                EventType::new("time"),
            ])
        });
        subs_composition.entry(Role::new("F")).and_modify(|s| {
            s.remove(&EventType::new("report1"));
        });
        let error_report = check(composition, &subs_composition);
        let mut errors = error_report_to_strings(error_report);

        let mut expected_errors = vec![
            "subsequently active role F does not subscribe to events in transition (0 || 2 || 0)--[observe@TR<report1>]-->(0 || 2 || 1)",
            "subsequently active role F does not subscribe to events in transition (3 || 2 || 0)--[observe@TR<report1>]-->(3 || 2 || 1)",
            "role QCR does not subscribe to event types car, part, report1 leading to or in joining event in transition (0 || 2 || 1)--[build@F<car>]-->(0 || 3 || 2)"];
        errors.sort();
        expected_errors.sort();
        assert_eq!(errors, expected_errors);
    }

    #[test]
    #[ignore]
    fn test_example_from_text() {
        setup_logger();
        let composition = compose_protocols(get_interfacing_swarms_5());
        assert!(composition.is_ok());

        let result_composition =
            exact_weak_well_formed_sub(get_interfacing_swarms_5(), &BTreeMap::new());
        assert!(result_composition.is_ok());
        let subs_composition = result_composition.unwrap();
        let result = check(get_interfacing_swarms_5(), &subs_composition);
        assert!(result.is_empty());
        let result_composition = overapprox_weak_well_formed_sub(
            get_interfacing_swarms_5(),
            &BTreeMap::new(),
            Granularity::Coarse,
        );
        assert!(result_composition.is_ok());
        let subs_composition = result_composition.unwrap();

        let result = check(get_interfacing_swarms_5(), &subs_composition);
        assert!(result.is_empty());
    }

    #[test]
    fn test_example_3() {
        setup_logger();
        let composition = compose_protocols(get_interfacing_swarms_3());
        assert!(composition.is_ok());

        let (g, i) = composition.unwrap();
        let swarm = to_swarm_json(g, i);
        println!("proto:\n {}", serde_json::to_string_pretty(&swarm).unwrap());
        let result_composition =
            exact_weak_well_formed_sub(get_interfacing_swarms_3(), &BTreeMap::new());
        assert!(result_composition.is_ok());
        let subs_composition = result_composition.unwrap();
        println!(
            "subs exact: {}",
            serde_json::to_string_pretty(&subs_composition).unwrap()
        );
        let result_composition = overapprox_weak_well_formed_sub(
            get_interfacing_swarms_3(),
            &BTreeMap::new(),
            Granularity::Fine,
        );
        assert!(result_composition.is_ok());
        let subs_composition = result_composition.unwrap();
        println!(
            "subs approx: {}",
            serde_json::to_string_pretty(&subs_composition).unwrap()
        );
    }

    #[test]
    #[ignore]
    fn test_pattern_4() {
        setup_logger();
        for i in 1..6 {
            let index = i as usize;
            let composition = compose_protocols(InterfacingSwarms(
                get_interfacing_swarms_pat_4().0[..index].to_vec(),
            ));
            assert!(composition.is_ok());

            let (g, i) = composition.unwrap();
            let node_count = g.node_count();
            let edge_count = g.edge_count();
            let swarm = to_swarm_json(g, i);
            println!("{}\n$$$$\n", serde_json::to_string_pretty(&swarm).unwrap());
            println!("num states: {}, num edges: {}", node_count, edge_count);
        }
    }

    #[test]
    #[ignore]
    fn test_diff_example() {
        setup_logger();
        let composition = compose_protocols(get_interfacing_swarms_diff_example());
        assert!(composition.is_ok());
        let protos = get_interfacing_swarms_diff_example();
        let result_composition = exact_weak_well_formed_sub(protos.clone(), &BTreeMap::new());
        assert!(result_composition.is_ok());
        let subs_composition = result_composition.unwrap();
        let result = check(protos.clone(), &subs_composition);
        assert!(result.is_empty());
        let result_composition =
            overapprox_weak_well_formed_sub(protos.clone(), &BTreeMap::new(), Granularity::Fine);
        assert!(result_composition.is_ok());
        let subs_composition = result_composition.unwrap();
        let result = check(protos.clone(), &subs_composition);
        assert!(result.is_empty());
    }

    #[test]
    #[ignore]
    fn test_ref_example() {
        setup_logger();
        let composition = compose_protocols(get_ref_pat_protos());
        assert!(composition.is_ok());
        let protos = get_ref_pat_protos();
        for p in protos.0.iter() {
            println!(
                "{}",
                serde_json::to_string_pretty(&p.protocol.clone()).unwrap()
            );
        }
        let (g, i) = composition.unwrap();
        let swarm = to_swarm_json(g, i);
        println!(
            "composition:\n {}",
            serde_json::to_string_pretty(&swarm).unwrap()
        );
        let result_composition = exact_weak_well_formed_sub(protos.clone(), &BTreeMap::new());
        assert!(result_composition.is_ok());
        let subs_composition = result_composition.unwrap();
        println!(
            "subs exact: {}",
            serde_json::to_string_pretty(&subs_composition).unwrap()
        );
        let result_composition =
            overapprox_weak_well_formed_sub(protos.clone(), &BTreeMap::new(), Granularity::Fine);
        assert!(result_composition.is_ok());
        let subs_composition = result_composition.unwrap();
        println!(
            "subs approx: {}",
            serde_json::to_string_pretty(&subs_composition).unwrap()
        );

        let result = check(protos.clone(), &subs_composition);
        println!("errors is empty: {}", result.is_empty());
    }

    #[test]
    fn test_after_not_concurrent() {
        let proto1: SwarmProtocolType =
            serde_json::from_str::<SwarmProtocolType>(
                r#"{
                    "initial": "0",
                    "transitions": [
                        { "source": "0", "target": "1", "label": { "cmd": "i1", "logType": ["i1"], "role": "IR" } },
                        { "source": "1", "target": "2", "label": { "cmd": "a", "logType": ["a"], "role": "R1" } },
                        { "source": "2", "target": "3", "label": { "cmd": "b", "logType": ["b"], "role": "R1" } },
                        { "source": "3", "target": "4", "label": { "cmd": "i2", "logType": ["i2"], "role": "IR" } }
                    ]
                }"#,
            )
            .unwrap();

        let proto2: SwarmProtocolType =
            serde_json::from_str::<SwarmProtocolType>(
                r#"{
                    "initial": "0",
                    "transitions": [
                        { "source": "0", "target": "1", "label": { "cmd": "i1", "logType": ["i1"], "role": "IR" } },
                        { "source": "1", "target": "2", "label": { "cmd": "c", "logType": ["c"], "role": "R2" } },
                        { "source": "2", "target": "3", "label": { "cmd": "d", "logType": ["d"], "role": "R2" } },
                        { "source": "3", "target": "4", "label": { "cmd": "i2", "logType": ["i2"], "role": "IR" } }
                    ]
                }"#,
            )
            .unwrap();

        let interfacing_swarms = InterfacingSwarms(vec![
            CompositionComponent {
                protocol: proto1,
                interface: None,
            },
            CompositionComponent {
                protocol: proto2,
                interface: Some(Role::new("IR")),
            },
        ]);

        let expected_after = BTreeMap::from([
            (
                EventType::new("i1"),
                BTreeSet::from([
                    EventType::new("a"),
                    EventType::new("b"),
                    EventType::new("c"),
                    EventType::new("d"),
                    EventType::new("i2"),
                ]),
            ),
            (
                EventType::new("a"),
                BTreeSet::from([EventType::new("b"), EventType::new("i2")]),
            ),
            (EventType::new("b"), BTreeSet::from([EventType::new("i2")])),
            (
                EventType::new("c"),
                BTreeSet::from([EventType::new("d"), EventType::new("i2")]),
            ),
            (EventType::new("d"), BTreeSet::from([EventType::new("i2")])),
            (EventType::new("i2"), BTreeSet::from([])),
        ]);

        let expected_concurrent = BTreeSet::from([
            unord_event_pair(EventType::new("a"), EventType::new("c")),
            unord_event_pair(EventType::new("a"), EventType::new("d")),
            unord_event_pair(EventType::new("b"), EventType::new("c")),
            unord_event_pair(EventType::new("b"), EventType::new("d")),
        ]);

        let combined_proto_info =
            combine_proto_infos_fold(prepare_proto_infos::<Role>(interfacing_swarms.clone()));

        assert_eq!(expected_after, combined_proto_info.succeeding_events);
        assert_eq!(expected_concurrent, combined_proto_info.concurrent_events);

        let (composition, composition_initial) =
            compose_protocols(interfacing_swarms.clone()).unwrap();

        let after_map = after_not_concurrent(
            &composition,
            composition_initial,
            &combined_proto_info.concurrent_events,
        );
        assert_eq!(expected_after, after_map);

        println!(
            "{}",
            serde_json::to_string_pretty(&to_swarm_json(composition, composition_initial)).unwrap()
        );
    }
}
