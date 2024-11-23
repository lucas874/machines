use crate::composition::composition_types::ProtoLabel;
use crate::{
    types::{EventType, Role, State, StateName, SwarmLabel, Transition},
    EdgeId, NodeId, Subscriptions, SwarmProtocol,
};
use itertools::Itertools;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use super::MapVec;
use super::{
    composition_types::{
        unord_event_pair, CompositionInputVec, EventLabel, ProtoInfo, RoleEventMap, SwarmInterface,
        UnordEventPair,
    },
    Graph,
};
use petgraph::{
    graph::EdgeReference,
    visit::{Dfs, EdgeRef, Walker},
    Direction::{self, Incoming, Outgoing},
};
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Error {
    SwarmError(crate::swarm::Error),
    SwarmErrorString(String), // bit ugly but instead of making prepare graph public or making another from_json returning Error type in swarm.rs
    InvalidInterfaceRole(Role),
    InterfaceEventNotInBothProtocols(EventType),
    RoleNotSubscribedToBranch(EdgeId, Role),
    RoleNotSubscribedToJoin(Vec<EventType>, EdgeId, Role),
    MoreThanOneEventTypeInCommand(EdgeId),
    EventEmittedByDifferentCommands(EventType, EdgeId, EdgeId),
    StateCanNotReachTerminal(NodeId),
    InvalidArg, // weird error. not related to shape of protocol, but ok for now.
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
            Error::RoleNotSubscribedToBranch(edge, role) => {
                format!(
                    "role {role} does not subscribe to events in branching transition {}",
                    Edge(graph, *edge)
                )
            }
            Error::RoleNotSubscribedToJoin(preceding_events, edge, role) => {
                let events = preceding_events.join(",");
                format!(
                    "role {role} does not subscribe to concurrent event types {events} leading to joining event in transition {}",
                    Edge(graph, *edge),
                )
            }
            Error::MoreThanOneEventTypeInCommand(edge) => {
                format!(
                    "transition {} emits more than one event type",
                    Edge(graph, *edge)
                )
            }
            Error::EventEmittedByDifferentCommands(event_type, edge1, edge2) => {
                format!(
                    "event type {event_type} emitted by command in transition {} and command in transition {}",
                    Edge(graph, *edge1),
                    Edge(graph, *edge2)
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

pub fn check(proto: SwarmProtocol, subs: &Subscriptions) -> (Graph, Option<NodeId>, Vec<Error>) {
    let proto_info = prepare_graph::<Role>(proto, subs, None);
    let (graph, initial, mut errors) = match proto_info.get_ith_proto(0) {
        Some((g, Some(i), e)) => (g, i, e),
        Some((g, None, e)) => return (g, None, e),
        _ => return (Graph::new(), None, vec![]),
    };

    errors.extend(weak_well_formed(&proto_info, 0));
    (graph, Some(initial), errors)
}

pub fn weak_well_formed_sub(proto: SwarmProtocol) -> (Subscriptions, ErrorReport) {
    let proto_info = prepare_graph::<Role>(proto, &BTreeMap::new(), None);
    let (graph, _, mut errors) = match proto_info.get_ith_proto(0) {
        Some((g, Some(i), e)) => (g, i, e),
        Some((g, None, e)) => return (BTreeMap::new(), ErrorReport(vec![(g, e)])),
        _ => return (BTreeMap::new(), ErrorReport(vec![(Graph::new(), vec![])])),
    };
    // Check confusion freeness now that it is not done in prepare
    errors.append(&mut confusion_free(&proto_info, 0));
    (wwf_sub(proto_info, 0), ErrorReport(vec![(graph, errors)]))
}

// why have this function and the one below????
pub fn compose_subscriptions(protos: CompositionInputVec) -> (Subscriptions, ErrorReport) {
    let protos_ifs = prepare_graphs(protos);
    let result = implicit_composition_fold(protos_ifs);
    (
        result.subscription.clone(),
        proto_info_to_error_report(result),
    )
}

pub fn implicit_composition_swarms(
    protos: CompositionInputVec,
) -> (
    Vec<((Graph, Option<NodeId>, Vec<Error>), BTreeSet<EventType>)>,
    Subscriptions,
) {
    let protos_ifs = prepare_graphs(protos);
    let result = implicit_composition_fold(protos_ifs);
    (result.protocols, result.subscription)
}

// find return type
pub fn compose_protocols(protos: CompositionInputVec) -> Result<(Graph, NodeId), ErrorReport> {
    let protos_ifs = prepare_graphs(protos);
    if protos_ifs
        .iter()
        .any(|(proto_info, _)| !proto_info.no_errors())
    {
        let result = swarms_to_error_report(
            protos_ifs
                .into_iter()
                .flat_map(|(proto_info, _)| proto_info.protocols)
                .collect(),
        );
        return Err(result);
    }
    // construct this to check whether the protocols interface. also checks wwf for each proto. not sure if good idea to check wwf.
    let implicit_composition = implicit_composition_fold(protos_ifs);
    if !implicit_composition.no_errors() {
        return Err(proto_info_to_error_report(implicit_composition));
    }

    let (explicit_composition, i) = explicit_composition(implicit_composition);
    Ok((explicit_composition, i))
}

fn prepare_graphs(protos: CompositionInputVec) -> Vec<(ProtoInfo, Option<Role>)> {
    protos
        .iter()
        .map(|p| {
            (
                prepare_graph::<Role>(p.protocol.clone(), &p.subscription, p.interface.clone()),
                p.interface.clone(),
            )
        })
        .collect()
}

// perform wwf check on every protocol in a ProtoInfo
fn weak_well_formed_proto_info(proto_info: ProtoInfo) -> ProtoInfo {
    let protocols: Vec<_> = proto_info
        .protocols
        .clone()
        .into_iter()
        .enumerate()
        .map(|(i, ((graph, initial, errors), interface))| {
            let errors = vec![errors, weak_well_formed(&proto_info, i)].concat();
            ((graph, initial, errors), interface)
        })
        .collect();

    ProtoInfo {
        protocols,
        ..proto_info
    }
}

/*
 * A graph that was constructed with prepare_graph with no errors will have one event type per command.
 * Similarly, such a graph will be weakly confusion free, which means we do not have to check for
 * command and log determinism like we do in swarm::well_formed.
 *
 */
fn weak_well_formed(proto_info: &ProtoInfo, proto_pointer: usize) -> Vec<Error> {
    // copied from swarm::well_formed
    let mut errors = Vec::new();
    let empty = BTreeSet::new(); // just for `sub` but needs its own lifetime
    let sub = |r: &Role| proto_info.subscription.get(r).unwrap_or(&empty);
    let (graph, initial, _) = match proto_info.get_ith_proto(proto_pointer) {
        Some((g, Some(i), e)) => (g, i, e),
        Some((_, None, e)) => return e,
        None => return vec![Error::InvalidArg],
    };

    errors.append(&mut confusion_free(proto_info, proto_pointer));

    // inital statements of loop copied from swarm::well_formed
    // comment from there:
    // "visit all reachable nodes of the graph to check their prescribed conditions; order doesn’t matter"
    for node in Dfs::new(&graph, initial).iter(&graph) {
        for edge in graph.edges_directed(node, Outgoing) {
            let event_type = edge.weight().get_event_type();

            // weak causal consistency
            // corresponds to condition 1
            // check if role subscribes to own emitted event.
            if !sub(&edge.weight().role).contains(&event_type) {
                errors.push(Error::SwarmError(
                    crate::swarm::Error::ActiveRoleNotSubscribed(edge.id()),
                ));
            }

            // weak causal consistency
            // corresponds to condition 2
            // subscribe to event immediately preceding
            // unlike well-formed we do not need to check that later involved roles
            // subscribes to fewer event in log than active -- only one event pr. log
            // active transitions not conc gets the transitions going out of edge.target()
            // and filters out the ones emitting events concurrent with event type of 'edge'
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

            // weak determinacy. branching events and joining subscribed to by all roles in roles(graph[node]). too strict though. does not use new notion of roles()
            // corresponds to branching rule of weak determinacy.
            let involved_roles = involved(node, &graph);
            if proto_info.branching_events.contains(&event_type) {
                let involved_not_subbed = involved_roles
                    .iter()
                    .filter(|r| !sub(&r).contains(&event_type));
                let mut branching_errors: Vec<_> = involved_not_subbed
                    .map(|r| Error::RoleNotSubscribedToBranch(edge.id(), r.clone()))
                    .collect();
                errors.append(&mut branching_errors);
            }

            // corresponds to joining rule of weak determinacy.
            // very ugly srsly need to redo this
            if proto_info.joining_events.contains(&event_type) {
                // not sure if this is to coarse?
                let join_set: BTreeSet<EventType> = proto_info.immediately_pre[&event_type]
                    .clone()
                    .into_iter()
                    .chain([event_type.clone()])
                    .collect();
                let involved_not_subbed = involved_roles
                    .iter()
                    .filter(|r| !join_set.is_subset(sub(r)));
                let pre: Vec<_> = proto_info.immediately_pre[&event_type]
                    .clone()
                    .into_iter()
                    .collect();
                let mut joining_errors: Vec<_> = involved_not_subbed
                    .map(|r| Error::RoleNotSubscribedToJoin(pre.clone(), edge.id(), r.clone()))
                    .collect();
                errors.append(&mut joining_errors);
                /* for incoming_pair in event_pairs_from_node(node, &graph, Incoming) {
                    if proto_info.concurrent_events.contains(&incoming_pair) {
                        let pair_vec: Vec<_> = incoming_pair.into_iter().collect();
                        let join_set = if
                            !proto_info.concurrent_events.contains(&unord_event_pair(pair_vec[0].clone(), event_type.clone()))
                            && !proto_info.concurrent_events.contains(&unord_event_pair(pair_vec[1].clone(), event_type.clone())) {
                                BTreeSet::from([pair_vec[0].clone(), pair_vec[1].clone(), event_type.clone()])
                            } else {
                                BTreeSet::new()
                            };
                        let involved_not_subbed = involved_roles
                            .iter()
                            .filter(|r| !join_set.is_subset(sub(r)));
                        let mut joining_errors: Vec<_> = involved_not_subbed
                            .map(|r| {
                                Error::RoleNotSubscribedToJoin(
                                    pair_vec[0].clone(),
                                    pair_vec[1].clone(),
                                    edge.id(),
                                    r.clone(),
                                )
                            })
                            .collect();
                        errors.append(&mut joining_errors);
                    }
                } */
            }
        }
    }
    errors
}

fn confusion_free(proto_info: &ProtoInfo, proto_pointer: usize) -> Vec<Error> {
    let mut event_to_command_map = BTreeMap::new();
    let (graph, initial, _) = match proto_info.get_ith_proto(proto_pointer) {
        Some((g, Some(i), e)) => (g, i, e),
        Some((_, None, e)) => return e, // this error would be returned twice in this case?
        None => return vec![Error::InvalidArg],
    };
    // compute concurrent events for this graph instead of using the field in proto_info bc. we want to now concurrency in this graph
    let concurrent_events = all_concurrent_pairs(&graph);
    // if graph contains no concurrency, make old confusion freeness check. requires us to call swarm::prepare_graph through swarm::from_json
    // corresponds to rule 3 of concurrency freeness in Composing Swarm Protocols
    let (graph, initial, mut errors) = if concurrent_events.is_empty() {
        let (graph, initial, e) = match crate::swarm::from_json(
            to_swarm_json(graph, initial),
            &proto_info.subscription,
        ) {
            (g, Some(i), e) => (g, i, e),
            (_, None, e) => {
                return e.into_iter().map(|s| Error::SwarmErrorString(s)).collect();
            }
        };
        (
            graph,
            initial,
            e.into_iter().map(|s| Error::SwarmErrorString(s)).collect(),
        )
    } else {
        (graph, initial, Vec::new())
    };

    let mut walk = Dfs::new(&graph, initial);

    // add to set of branching and joining
    // check for weak confusion freeness. confusion freeness does not depend on subscription.
    // nice to whether graph is weakly confusion freeness before for instance generating wwf-subscription.
    // for each node we pass three times over its outgoing edges... awkward find better way.
    while let Some(node_id) = walk.next(&graph) {
        // weak confusion freeness checks
        let mut target_map: BTreeMap<SwarmLabel, NodeId> = BTreeMap::new();
        for e in graph.edges_directed(node_id, Outgoing) {
            // rule 1 check. Event types only associated with one command role pair.
            let (role, command, e_id) = event_to_command_map
                .entry(e.weight().get_event_type())
                .or_insert((e.weight().role.clone(), e.weight().cmd.clone(), e.id()));
            if (role.clone(), command.clone()) != (e.weight().role.clone(), e.weight().cmd.clone())
            {
                errors.push(Error::EventEmittedByDifferentCommands(
                    e.weight().get_event_type(),
                    *e_id,
                    e.id(),
                ));
            }

            // rule 2 check. Determinism.
            let dst = target_map.entry(e.weight().clone()).or_insert(e.target());
            if *dst != e.target() {
                errors.push(Error::SwarmError(
                    crate::swarm::Error::NonDeterministicGuard(e.id()),
                ));
                errors.push(Error::SwarmError(
                    crate::swarm::Error::NonDeterministicCommand(e.id()),
                ));
            }
        }

        // we do not check for weak confusion freeness rule 3 if graph contains concurrency...? can we?

        // weak confusion free rule 4 check.
        errors.append(&mut node_can_reach_zero(&graph, node_id));
    }

    errors
}

/*
 * given a swarm protocol return smallest wwf-subscription
 * assume that the protocol does not contain concurrency etc
 * assume graph was constructed from a prepare_graph call with an empty subscription -- empty roles and active fields
 * log and command determinism?
 */
fn wwf_sub(proto_info: ProtoInfo, proto_pointer: usize) -> Subscriptions {
    let mut subscriptions: BTreeMap<Role, BTreeSet<EventType>> = BTreeMap::new();
    let (graph, initial, _) = match proto_info.get_ith_proto(proto_pointer) {
        Some((g, Some(i), e)) => (g, i, e),
        _ => return BTreeMap::new(),
    };

    for node in Dfs::new(&graph, initial).iter(&graph) {
        // for each edge going out of node:
        // extend subscriptions to satisfy conditions for weak causal consistency
        // make role performing the command subscribe to the emitted event type
        // make roles active in continuations subscribe to the event type
        // make an overapproximation of the roles in roles(e.G) subscribe to branching events.
        for edge in graph.edges_directed(node, Outgoing) {
            let event_type = edge.weight().get_event_type();
            // weak causal consistency 1: a role subscribes to the events it emits
            subscriptions
                .entry(edge.weight().role.clone())
                .and_modify(|curr| {
                    curr.insert(event_type.clone());
                })
                .or_insert(BTreeSet::from([event_type.clone()]));

            // weak causal consistency 2: a role subscribes to events that immediately precedes its own commands
            for active in active_transitions_not_conc(
                edge.target(),
                &graph,
                &event_type,
                &proto_info.concurrent_events,
            ) {
                subscriptions
                    .entry(active.role)
                    .and_modify(|curr| {
                        curr.insert(event_type.clone());
                    })
                    .or_insert(BTreeSet::from([event_type.clone()]));
            }

            // weak determinacy 1: roles subscribe to branching events. be more precise with what roles. overapproximation now.
            let involved_roles = involved(node, &graph);
            if proto_info.branching_events.contains(&event_type) {
                for r in involved_roles.iter() {
                    subscriptions
                        .entry(r.clone())
                        .and_modify(|curr| {
                            curr.insert(event_type.clone());
                        })
                        .or_insert(BTreeSet::from([event_type.clone()]));
                }
            }

            // weak determinacy 2. joining events. Add test for this...
            // go over this again. But right now if joining add joining and all
            // events immediately preceding the joining event
            if proto_info.joining_events.contains(&event_type) {
                let events_to_add: BTreeSet<EventType> = proto_info.immediately_pre[&event_type]
                    .clone()
                    .into_iter()
                    .chain([event_type.clone()])
                    .collect();
                for r in involved_roles.iter() {
                    subscriptions
                        .entry(r.clone())
                        .and_modify(|curr| {
                            curr.append(&mut events_to_add.clone());
                        })
                        .or_insert(events_to_add.clone());
                }
            }
        }
    }

    subscriptions
}

fn implicit_composition<T: SwarmInterface>(
    proto_info1: ProtoInfo,
    proto_info2: ProtoInfo,
    interface: T,
) -> ProtoInfo {
    let errors = interface.check_interface(&proto_info1, &proto_info2);
    if !errors.is_empty() {
        let protocols = vec![
            proto_info1.protocols.clone(),
            proto_info2.protocols.clone(),
            vec![((Graph::new(), None, errors), BTreeSet::new())],
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
    let subscription = combine_subscriptions(&proto_info1, &proto_info2, &interface);
    let concurrent_events = get_concurrent_events(&proto_info1, &proto_info2, &interface);
    let branching_events: BTreeSet<EventType> = proto_info1
        .branching_events
        .union(&proto_info2.branching_events)
        .cloned()
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
    ProtoInfo::new(
        protocols,
        role_event_map,
        subscription,
        concurrent_events,
        branching_events,
        joining_events,
        immediately_pre,
    )
}

// The result<error, proto> thing here...
fn implicit_composition_fold<T: SwarmInterface>(protos: Vec<(ProtoInfo, Option<T>)>) -> ProtoInfo {
    if protos.is_empty()
        || protos[0].1.is_some()
        || protos[1..].iter().any(|(_, interface)| interface.is_none())
    {
        return ProtoInfo::new_only_proto(vec![(
            (Graph::new(), None, vec![Error::InvalidArg]),
            BTreeSet::new(),
        )]);
    }

    let protos: Vec<_> = protos
        .into_iter()
        .map(|(proto_info, interface)| (weak_well_formed_proto_info(proto_info), interface))
        .collect();

    // check that every proto is wwf before composing. Consider doing this elsewhere?
    if protos.iter().any(|(proto_info, _)| !proto_info.no_errors()) {
        let protocols: Vec<_> = protos
            .into_iter()
            .flat_map(|(proto_info, _)| proto_info.protocols)
            .collect();
        return ProtoInfo::new_only_proto(protocols);
    }

    let (proto, _) = protos[0].clone();

    protos[1..]
        .to_vec()
        .into_iter()
        .fold(proto, |acc, (p, interface)| {
            implicit_composition(acc, p, interface.unwrap())
        })
}

// given some node, return the swarmlabels going out of that node that are not concurrent with 'event'
fn active_transitions_not_conc(
    node: NodeId,
    graph: &crate::Graph,
    event: &EventType,
    concurrent_events: &BTreeSet<BTreeSet<EventType>>,
) -> Vec<SwarmLabel> {
    graph
        .edges_directed(node, Outgoing)
        .map(|e| e.weight().clone())
        .filter(|e| {
            !concurrent_events.contains(&BTreeSet::from([event.clone(), e.get_event_type()]))
        })
        .collect()
}

fn involved(node: NodeId, graph: &super::Graph) -> BTreeSet<Role> {
    let mut roles: BTreeSet<Role> = BTreeSet::new();
    for n in Dfs::new(graph, node).iter(graph) {
        roles.append(
            &mut graph
                .edges_directed(n, Outgoing)
                .map(|e| e.weight().role.clone())
                .collect(),
        )
    }

    roles
}

// turn a SwarmProtocol into a ProtoInfo. Check for errors such as initial not reachable and empty logs.
fn prepare_graph<T: SwarmInterface>(
    proto: SwarmProtocol,
    subs: &Subscriptions,
    interface: Option<T>,
) -> ProtoInfo {
    let mut role_event_map: RoleEventMap = BTreeMap::new();
    let mut branching_events = BTreeSet::new();
    let mut joining_events: BTreeSet<EventType> = BTreeSet::new();
    let mut immediately_pre_map: BTreeMap<EventType, BTreeSet<EventType>> = BTreeMap::new();
    let (graph, initial, errors) = swarm_to_graph(&proto);
    if initial.is_none() || !errors.is_empty() {
        return ProtoInfo::new_only_proto(vec![((graph, initial, errors), BTreeSet::new())]);
    }

    // If interface is some, then we want to interface this protocol
    // with some other protocol on this set of events.
    // We do not know if we can do that yet though, but we prepare as if we can.
    let interface = if interface.is_some() {
        interface.unwrap().interfacing_event_types_single(&graph)
    } else {
        BTreeSet::new()
    };

    let concurrent_events = all_concurrent_pairs(&graph);
    let mut walk = Dfs::new(&graph, initial.unwrap());

    // add to set of branching and joining
    // check for weak confusion freeness. confusion freeness does not depend on subscription.
    // nice to whether graph is weakly confusion freeness before for instance generating wwf-subscription.
    // for each node we pass three times over its outgoing edges... awkward find better way.
    while let Some(node_id) = walk.next(&graph) {
        let outgoing_pairs = event_pairs_from_node(node_id, &graph, Outgoing);

        // should work even if two branches with same outgoing event type right?
        branching_events.append(
            &mut outgoing_pairs
                .into_iter()
                .filter(|pair| !concurrent_events.contains(pair))
                .concat(),
        );

        let incoming_pairs = event_pairs_from_node(node_id, &graph, Incoming);

        // add joining events. if there are concurrent incoming edges, add the event types of all outgoing edges not concurrent with identified incoming to set of joining events.
        let incoming_concurrent = incoming_pairs
            .into_iter()
            .filter(|pair| concurrent_events.contains(pair))
            .map(|set| set.into_iter().collect::<Vec<_>>());
        let outgoing = graph
            .edges_directed(node_id, Outgoing)
            .map(|e| e.weight().get_event_type())
            .collect::<BTreeSet<_>>();
        let product: Vec<_> = incoming_concurrent.cartesian_product(&outgoing).collect();
        // if we have Ga-ea->Gb-eb->Gc, Gd-ec->Gb, with ea, ec concurrent, but not concurrent with eb then eb is joining
        let mut joining: BTreeSet<_> = product
            .into_iter()
            .filter(|(pair, event)| {
                !concurrent_events.contains(&unord_event_pair(pair[0].clone(), (*event).clone()))
                    && !concurrent_events
                        .contains(&unord_event_pair(pair[1].clone(), (*event).clone()))
            })
            .map(|(_, event)| event.clone())
            .collect();

        joining_events.append(&mut joining);

        for edge in graph.edges_directed(node_id, Outgoing) {
            role_event_map
                .entry(edge.weight().role.clone())
                .and_modify(|role_info| {
                    role_info.insert(edge.weight().clone());
                })
                .or_insert(BTreeSet::from([edge.weight().clone()]));

            let mut pre = get_immediately_pre(&graph, edge, &concurrent_events);
            immediately_pre_map
                .entry(edge.weight().get_event_type())
                .and_modify(|events| {
                    events.append(&mut pre);
                })
                .or_insert(pre);
        }
    }

    ProtoInfo::new(
        vec![((graph, initial, errors), interface)],
        role_event_map,
        subs.clone(),
        concurrent_events,
        branching_events,
        joining_events,
        immediately_pre_map,
    )
}

// turn a SwarmProtocol into a petgraph. perform some checks that are not strictly related to wwf, but must be successful for any further analysis to take place
fn swarm_to_graph(proto: &SwarmProtocol) -> (Graph, Option<NodeId>, Vec<Error>) {
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

pub fn from_json(
    proto: SwarmProtocol,
    subs: &Subscriptions,
) -> (Graph, Option<NodeId>, Vec<String>) {
    let proto_info = prepare_graph::<Role>(proto, subs, None);
    let (g, i, e) = match proto_info.get_ith_proto(0) {
        Some((g, i, e)) => (g, i, e),
        _ => return (Graph::new(), None, vec![]),
    };
    let e = e.map(Error::convert(&g));
    (g, i, e)
}

fn proto_info_to_error_report(proto_info: ProtoInfo) -> ErrorReport {
    ErrorReport(
        proto_info
            .protocols
            .into_iter()
            .map(|((graph, _, errors), _)| (graph, errors))
            .collect(),
    )
}

pub fn swarms_to_error_report(
    swarms: Vec<((Graph, Option<NodeId>, Vec<Error>), BTreeSet<EventType>)>,
) -> ErrorReport {
    ErrorReport(
        swarms
            .into_iter()
            .map(|((graph, _, errors), _)| (graph, errors))
            .collect(),
    )
}

// copied from swarm::swarm.rs
fn all_nodes_reachable(graph: &Graph, initial: NodeId) -> Vec<Error> {
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

fn node_can_reach_zero<N, E>(graph: &petgraph::Graph<N, E>, node: NodeId) -> Vec<Error> {
    for n in Dfs::new(&graph, node).iter(&graph) {
        if graph.edges_directed(n, Outgoing).count() == 0 {
            return vec![];
        }
    }
    vec![Error::StateCanNotReachTerminal(node)]
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

fn all_concurrent_pairs(graph: &Graph) -> BTreeSet<UnordEventPair> {
    graph
        .node_indices()
        .flat_map(|node| diamond_shape(graph, node))
        .collect()
}

// check for 'diamond shape' starting at a node.
// we have a pair of concurrent events if there is a path G-a->Ga-b->G', G-b->Gb-a->G', diamond shape.
fn diamond_shape(graph: &Graph, node: NodeId) -> BTreeSet<BTreeSet<EventType>> {
    let mut paths: BTreeSet<(EventType, EventType, NodeId)> = BTreeSet::new();
    let mut concurrent_events: BTreeSet<BTreeSet<EventType>> = BTreeSet::new();

    for edge1 in graph.edges_directed(node, Outgoing) {
        for edge2 in graph.edges_directed(edge1.target(), Outgoing) {
            let tup = (
                edge1.weight().get_event_type(),
                edge2.weight().get_event_type(),
                edge2.target(),
            );
            paths.insert(tup.clone());
            if paths.contains(&(tup.1.clone(), tup.0.clone(), tup.2.clone())) && tup.0 != tup.1 {
                concurrent_events.insert(unord_event_pair(tup.0, tup.1)); //BTreeSet::from([tup.0, tup.1]));
            }
        }
    }

    concurrent_events
}

// get events that are immediately before some event and not concurrent.
// backtrack if immediately preceding is concurrent. not sure if this is needed or ok though
fn get_immediately_pre(
    graph: &Graph,
    edge: EdgeReference<'_, SwarmLabel>,
    concurrent_events: &BTreeSet<BTreeSet<EventType>>,
) -> BTreeSet<EventType> {
    let node = edge.source();
    let event_type = edge.weight().get_event_type();
    let mut visited = BTreeSet::from([node]);
    let mut to_visit = Vec::from([node]);
    let mut immediately_pre = BTreeSet::new();

    while let Some(node) = to_visit.pop() {
        for e in graph.edges_directed(node, Incoming) {
            if !concurrent_events.contains(&unord_event_pair(
                event_type.clone(),
                e.weight().get_event_type(),
            )) {
                immediately_pre.insert(e.weight().get_event_type());
            } else {
                // not sure this else branch is actually needed. when concurrency, one of the incoming will be noncurrent with event?
                let source = e.source();
                if !visited.contains(&source) {
                    visited.insert(source);
                    to_visit.push(source);
                }
            }
        }
    }

    immediately_pre
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

// add all branching and all interfacing events to the subscription of each role
fn combine_subscriptions<T: SwarmInterface>(
    proto_info1: &ProtoInfo,
    proto_info2: &ProtoInfo,
    interface: &T,
) -> Subscriptions {
    let interfacing_events = interface.interfacing_event_types(proto_info1, &proto_info2);

    let extra = interfacing_events
        .clone()
        .into_iter()
        .chain(proto_info1.branching_events.clone())
        .chain(proto_info2.branching_events.clone())
        .chain(proto_info1.joining_events.clone()) // think joins could be handled in a better way...
        .chain(
            proto_info1
                .joining_events
                .iter()
                .flat_map(|e| proto_info1.immediately_pre[e].clone()),
        )
        .chain(proto_info2.joining_events.clone())
        .chain(
            proto_info2
                .joining_events
                .iter()
                .flat_map(|e| proto_info2.immediately_pre[e].clone()),
        )
        .chain(
            interfacing_events
                .iter()
                .flat_map(|e| proto_info1.immediately_pre[e].clone()),
        )
        .chain(
            interfacing_events
                .iter()
                .flat_map(|e| proto_info2.immediately_pre[e].clone()),
        )
        .collect::<BTreeSet<_>>();

    combine_maps(
        proto_info1.subscription.clone(),
        proto_info2.subscription.clone(),
        Some(extra),
    )
}

// overapproximate concurrent events. anything from different protocols that are not interfacing events is considered concurrent.
fn get_concurrent_events<T: SwarmInterface>(
    proto_info1: &ProtoInfo,
    proto_info2: &ProtoInfo,
    interface: &T,
) -> BTreeSet<UnordEventPair> {
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

// precondition: the protocols can interface on the given interfaces
fn explicit_composition(proto_info: ProtoInfo) -> (Graph, NodeId) {
    if proto_info.protocols.is_empty() {
        return (Graph::new(), NodeId::end());
    }

    let ((g, i, _), _) = proto_info.protocols[0].clone();
    let folder =
        |(acc_g, acc_i): (Graph, NodeId),
         ((g, i, _), interface): ((Graph, Option<NodeId>, Vec<Error>), BTreeSet<EventType>)|
         -> (Graph, NodeId) {
            crate::composition::composition_machine::compose(acc_g, acc_i, g, i.unwrap(), interface)
        };
    proto_info.protocols[1..]
        .to_vec()
        .into_iter()
        .fold((g, i.unwrap()), folder)
}

pub fn to_swarm_json(graph: crate::Graph, initial: NodeId) -> SwarmProtocol {
    let machine_label_mapper = |g: &crate::Graph, eref: EdgeReference<'_, SwarmLabel>| {
        let label = eref.weight().clone();
        let source = g[eref.source()].state_name().clone();
        let target = g[eref.target()].state_name().clone();
        Transition {
            label: label,
            source: source,
            target: target,
        }
    };

    let transitions: Vec<_> = graph
        .edge_references()
        .map(|e| machine_label_mapper(&graph, e))
        .collect();

    SwarmProtocol {
        initial: graph[initial].state_name().clone(),
        transitions,
    }
}

#[cfg(test)]
mod tests {
    use std::{cmp, iter::zip, sync::Mutex};

    use crate::{
        composition::{
            composition_machine::{project, project_combine, to_option_machine},
            composition_types::CompositionInput,
            error_report_to_strings,
        },
        types::Command,
        MapVec,
    };

    use super::*;
    use itertools::enumerate;
    use petgraph::visit::{IntoNodeReferences, Reversed};
    use proptest::prelude::*;
    use rand::{distributions::Bernoulli, prelude::*};

    // Example from coplaws slides
    fn get_proto1() -> SwarmProtocol {
        serde_json::from_str::<SwarmProtocol>(
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
    fn get_proto2() -> SwarmProtocol {
        serde_json::from_str::<SwarmProtocol>(
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
    fn get_proto3() -> SwarmProtocol {
        serde_json::from_str::<SwarmProtocol>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "build", "logType": ["car"], "role": "F" } },
                    { "source": "1", "target": "2", "label": { "cmd": "test", "logType": ["report"], "role": "TR" } },
                    { "source": "2", "target": "3", "label": { "cmd": "accept", "logType": ["ok"], "role": "QCR" } },
                    { "source": "2", "target": "3", "label": { "cmd": "reject", "logType": ["notOk"], "role": "QCR" } }
                ]
            }"#,
        )
        .unwrap()
    }
    fn get_subs3() -> Subscriptions {
        serde_json::from_str::<Subscriptions>(
            r#"{
                "F": ["car"],
                "TR": ["car", "report"],
                "QCR": ["report", "ok", "notOk"]
            }"#,
        )
        .unwrap()
    }
    fn get_proto1_proto2_composed() -> SwarmProtocol {
        serde_json::from_str::<SwarmProtocol>(
            r#"{
                "initial": "0 || 0",
                "transitions": [
                    { "source": "0 || 0", "target": "1 || 1", "label": { "cmd": "request", "logType": ["partID"], "role": "T" } },
                    { "source": "0 || 0", "target": "3 || 0", "label": { "cmd": "close", "logType": ["time"], "role": "D" } },
                    { "source": "1 || 1", "target": "2 || 1", "label": { "cmd": "get", "logType": ["pos"], "role": "FL" } },
                    { "source": "2 || 1", "target": "0 || 2", "label": { "cmd": "deliver", "logType": ["part"], "role": "T" } },
                    { "source": "0 || 2", "target": "0 || 3", "label": { "cmd": "build", "logType": ["car"], "role": "F" } },
                    { "source": "0 || 2", "target": "3 || 2", "label": { "cmd": "close", "logType": ["time"], "role": "D" } },
                    { "source": "0 || 3", "target": "3 || 3", "label": { "cmd": "close", "logType": ["time"], "role": "D" } },
                    { "source": "3 || 2", "target": "3 || 3", "label": { "cmd": "build", "logType": ["car"], "role": "F" } }
                ]
            }"#,
        )
        .unwrap()
    }

    fn get_proto1_proto2_composed_subs() -> Subscriptions {
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

    fn get_confusionful_proto1() -> SwarmProtocol {
        serde_json::from_str::<SwarmProtocol>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "request", "logType": ["partID"], "role": "T" } },
                    { "source": "1", "target": "2", "label": { "cmd": "get", "logType": ["pos"], "role": "FL" } },
                    { "source": "2", "target": "0", "label": { "cmd": "request", "logType": ["partID"], "role": "T" } },
                    { "source": "0", "target": "0", "label": { "cmd": "close", "logType": ["time", "time2"], "role": "D" } }
                ]
            }"#,
        )
        .unwrap()
    }

    // pos event type associated with multiple commands and nondeterminism at 0 || 0
    fn get_confusionful_proto2() -> SwarmProtocol {
        serde_json::from_str::<SwarmProtocol>(
            r#"{
                "initial": "0 || 0",
                "transitions": [
                    { "source": "0 || 0", "target": "1 || 1", "label": { "cmd": "request", "logType": ["partID"], "role": "T" } },
                    { "source": "0 || 0", "target": "3 || 0", "label": { "cmd": "request", "logType": ["partID"], "role": "T" } },
                    { "source": "1 || 1", "target": "2 || 1", "label": { "cmd": "get", "logType": ["pos"], "role": "FL" } },
                    { "source": "2 || 1", "target": "0 || 2", "label": { "cmd": "deliver", "logType": ["pos"], "role": "T" } },
                    { "source": "0 || 2", "target": "0 || 3", "label": { "cmd": "build", "logType": ["car"], "role": "F" } },
                    { "source": "0 || 2", "target": "3 || 2", "label": { "cmd": "close", "logType": ["time"], "role": "D" } },
                    { "source": "0 || 3", "target": "3 || 3", "label": { "cmd": "close", "logType": ["time"], "role": "D" } },
                    { "source": "3 || 2", "target": "3 || 3", "label": { "cmd": "build", "logType": ["car"], "role": "F" } }
                ]
            }"#,
        )
        .unwrap()
    }

    // initial state state unreachable
    fn get_confusionful_proto3() -> SwarmProtocol {
        serde_json::from_str::<SwarmProtocol>(
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
    fn get_confusionful_proto4() -> SwarmProtocol {
        serde_json::from_str::<SwarmProtocol>(
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

    fn get_composition_input_vec1() -> CompositionInputVec {
        vec![
            CompositionInput {
                protocol: get_proto1(),
                subscription: weak_well_formed_sub(get_proto1()).0,
                interface: None,
            },
            CompositionInput {
                protocol: get_proto2(),
                subscription: weak_well_formed_sub(get_proto2()).0,
                interface: Some(Role::new("T")),
            },
            CompositionInput {
                protocol: get_proto3(),
                subscription: weak_well_formed_sub(get_proto3()).0,
                interface: Some(Role::new("F")),
            },
        ]
    }

    // for uniquely named roles. not strictly necessary? but nice. little ugly idk
    static ROLE_COUNTER_MUTEX: Mutex<u32> = Mutex::new(0);

    static R_BASE: &str = "R";
    static IR_BASE: &str = "IR";
    static CMD_BASE: &str = "cmd";
    static E_BASE: &str = "e";

    prop_compose! {
        fn vec_swarm_label(role: Role, max_events: usize)(vec in prop::collection::vec((CMD_BASE, E_BASE), 1..max_events)) -> Vec<SwarmLabel> {
            vec.into_iter()
            .enumerate()
            .map(|(i, (cmd, event))|
                SwarmLabel { cmd: Command::new(&format!("{role}_{cmd}_{i}")), log_type: vec![EventType::new(&format!("{role}_{event}_{i}"))], role: role.clone()})
            .collect()
        }
    }

    prop_compose! {
        fn vec_role(max_roles: usize)(vec in prop::collection::vec(R_BASE, 1..max_roles)) -> Vec<Role> {
            vec
            .into_iter()
            .map(|role| {
                let mut mut_guard = ROLE_COUNTER_MUTEX.lock().unwrap();
                let i: u32 = *mut_guard;
                *mut_guard += 1;
                Role::new(&format!("{role}{i}"))
            }).collect()
        }
    }

    prop_compose! {
        fn all_labels(max_roles: usize, max_events: usize)
                    (roles in vec_role(max_roles))
                    (labels in roles.into_iter().map(|role| vec_swarm_label(role, max_events)).collect::<Vec<_>>()) -> Vec<SwarmLabel> {
            labels.concat()
        }
    }

    prop_compose! {
        fn all_labels_1(roles: Vec<Role>, max_events: usize)
                    (labels in roles.into_iter().map(|role| vec_swarm_label(role, max_events)).collect::<Vec<_>>()) -> Vec<Vec<SwarmLabel>> {
            labels
        }
    }

    prop_compose! {
        fn all_labels_and_if(max_roles: usize, max_events: usize)
                (roles in vec_role(max_roles))
                (index in 0..roles.len(), labels in roles.into_iter().map(|role| vec_swarm_label(role, max_events)).collect::<Vec<_>>())
                -> (Vec<SwarmLabel>, Vec<SwarmLabel>) {
            let interfacing = labels[index].clone();
            (labels.concat(), interfacing)
        }
    }

    prop_compose! {
        fn all_labels_composition(max_roles: usize, max_events: usize, max_protos: usize, exactly_max: bool)
                (tuples in prop::collection::vec(all_labels_and_if(max_roles, max_events), if exactly_max {max_protos..=max_protos} else {1..=max_protos}))
                -> Vec<(Option<Role>, Vec<SwarmLabel>)> {
            let (labels, interfaces): (Vec<_>, Vec<_>) = tuples.into_iter().unzip();
            let tmp: Vec<(Option<Role>, Vec<SwarmLabel>)>  = interfaces[..interfaces.len()].to_vec().into_iter().map(|interface| (Some(interface[0].role.clone()), interface)).collect();
            let interfaces: Vec<(Option<Role>, Vec<SwarmLabel>)> = vec![vec![(None, vec![])], tmp].concat();
            labels.into_iter().zip(interfaces.into_iter()).map(|(labels, (interface, interfacing_cmds))| (interface, vec![labels, interfacing_cmds].concat())).collect()
        }
    }

    // shuffle labels before calling, then call random graph
    fn random_graph_shuffle_labels(mut swarm_labels: Vec<SwarmLabel>) -> (Graph, NodeId) {
        let mut rng = rand::thread_rng();
        swarm_labels.shuffle(&mut rng);
        random_graph(swarm_labels)
    }

    // add option (graph, nodeid) argument and build on top of this graph if some
    fn random_graph(mut swarm_labels: Vec<SwarmLabel>) -> (Graph, NodeId) {
        let mut graph = Graph::new();
        let mut nodes = Vec::new();
        let mut rng = rand::thread_rng();
        let b_dist = Bernoulli::new(0.1).unwrap(); // bernoulli distribution with propability 0.1 of success
        let gen_state_name = |g: &Graph| -> State { State::new(&g.node_count().to_string()) };

        //swarm_labels.shuffle(&mut rng); // Back to shuffling here again. Do not know if it is better to shuffle here or shuffle the labels before calling.
        let initial = graph.add_node(State::new(&graph.node_count().to_string()));
        nodes.push(initial);

        while let Some(label) = swarm_labels.pop() {
            // consider bernoulli thing. and distrbutions etc. bc documentations says that these once are optimised for cases where only a single sample is needed... if just faster does not matter
            // generate new or select old source? Generate new or select old, generate new target or select old?
            // same because you would have to connect to graph at some point anyway...?
            // exclusive range upper limit
            let source_node = if b_dist.sample(&mut rng) {
                nodes[rng.gen_range(0..nodes.len())]
            } else {
                // this whole thing was to have fewer branches... idk. loop will terminate because we always can reach 0?
                let mut source = nodes[rng.gen_range(0..nodes.len())];
                while graph.edges_directed(source, Outgoing).count() > 0 {
                    source = nodes[rng.gen_range(0..nodes.len())];
                }

                source
            };

            // if generated bool then select an existing node as target
            // otherwise generate a new node as target
            if b_dist.sample(&mut rng) && !swarm_labels.is_empty() {
                let index = rng.gen_range(0..nodes.len());
                let target_node = nodes[index];
                //nodes.push(graph.add_node(State::new(&graph.node_count().to_string())));
                graph.add_edge(source_node, target_node, label);
                // we should be able to reach a terminating node from all nodes.
                // we check that swarm_labels is not empty before entering this branch
                // so we should be able to generate new node and add and edge from
                // target node to this new node
                if !node_can_reach_zero(&graph, target_node).is_empty() {
                    let new_target_node = graph.add_node(gen_state_name(&graph));
                    // consider not pushing?
                    nodes.push(new_target_node);
                    let new_weight = swarm_labels.pop().unwrap();
                    graph.add_edge(target_node, new_target_node, new_weight);
                }
            } else {
                let target_node = graph.add_node(gen_state_name(&graph));
                nodes.push(target_node);
                graph.add_edge(source_node, target_node, label);
            }
        }

        (graph, initial)
    }

    // generate a number of protocols that interface. interfacing events may appear in different orderes in the protocols
    // and may be scattered across different branches: we may 'lose' a lot of behavior.
    prop_compose! {
        fn generate_composition_input_vec(max_roles: usize, max_events: usize, max_protos: usize, exactly_max: bool)
                          (vec in all_labels_composition(max_roles, max_events, max_protos, exactly_max))
                          -> CompositionInputVec {
            vec.into_iter()
                .map(|(interface, swarm_labels)| (random_graph_shuffle_labels(swarm_labels), interface))
                .map(|((graph, initial), interface)| {
                    let protocol = to_swarm_json(graph, initial);
                    CompositionInput { protocol, subscription: BTreeMap::new(), interface }
                    }
                ).collect()

        }
    }

    // generate a number of protocols that interface and where protocol i 'refines' protocol i+1
    prop_compose! {
        fn generate_composition_input_vec_refinement(max_roles: usize, max_events: usize, num_protos: usize)
                          (vec in prop::collection::vec(all_labels(max_roles, max_events), cmp::max(0, num_protos-1)))
                          -> CompositionInputVec {
            let level_0_proto = refinement_initial_proto();
            let mut graphs = vec![CompositionInput {protocol: to_swarm_json(level_0_proto.0, level_0_proto.1), subscription: BTreeMap::new(), interface: None}];
            let mut vec = vec
                .into_iter()
                .map(|swarm_labels| random_graph_shuffle_labels(swarm_labels))
                .enumerate()
                .map(|(level, (proto, initial))| (level, refinement_shape(level, proto, initial)))
                .map(|(level, (proto, initial))|
                        CompositionInput { protocol: to_swarm_json(proto, initial), subscription: BTreeMap::new(), interface: Some(Role::new(&format!("{IR_BASE}_{level}")))}
                    )
                .collect();
            graphs.append(&mut vec);

            graphs
        }
    }

    prop_compose! {
        fn protos_refinement_2(max_events: usize, num_protos: usize)
                    (labels in all_labels_1((0..num_protos).into_iter().map(|i| Role::new(&format!("{IR_BASE}_{i}"))).collect(), max_events))
                    -> Vec<(Graph, NodeId)> {
            labels.into_iter().map(|labels| random_graph(labels)).collect()
        }
    }

    prop_compose! {
        fn generate_composition_input_vec_refinement_2(max_roles: usize, max_events: usize, num_protos: usize)
                    (protos in protos_refinement_2(max_events, num_protos))
                    -> CompositionInputVec {
            let protos_altered: Vec<_> = protos.clone()
                .into_iter()
                .enumerate()
                .map(|(i, (graph, initial))| {
                    let (graph, initial) = if i == 0 {
                        (graph, initial)
                    } else {
                        insert_into(protos[i-1].clone(), (graph, initial))
                    };
                    randomly_expand(graph, initial, max_roles, max_events)
                }).collect();

            protos_altered.into_iter()
                .enumerate()
                .map(|(i, (graph, initial))|
                    CompositionInput { protocol: to_swarm_json(graph, initial), subscription: BTreeMap::new(), interface: if i == 0 { None } else { Some(Role::new(&format!("{IR_BASE}_{i}"))) } })
                .collect()
        }

    }

    fn refinement_initial_proto() -> (Graph, NodeId) {
        let mut graph = Graph::new();
        let initial = graph.add_node(State::new("0"));
        let middle = graph.add_node(State::new("1"));
        let last = graph.add_node(State::new("2"));

        let start_label = SwarmLabel {
            cmd: Command::new(&format!("{IR_BASE}_0_{CMD_BASE}_0")),
            log_type: vec![EventType::new(&format!("{IR_BASE}_0_{E_BASE}_0"))],
            role: Role::new(&format!("{IR_BASE}_0")),
        };
        let end_label = SwarmLabel {
            cmd: Command::new(&format!("{IR_BASE}_0_{CMD_BASE}_1")),
            log_type: vec![EventType::new(&format!("{IR_BASE}_0_{E_BASE}_1"))],
            role: Role::new(&format!("{IR_BASE}_0")),
        };

        graph.add_edge(initial, middle, start_label);
        graph.add_edge(middle, last, end_label);

        (graph, initial)
    }

    // consider a version where we change existing labels instead of adding new edges. still adding new edges for if, but not next if.
    fn refinement_shape(level: usize, mut proto: Graph, initial: NodeId) -> (Graph, NodeId) {
        let terminal_nodes: Vec<_> = proto
            .node_indices()
            .filter(|node| proto.edges_directed(*node, Outgoing).count() == 0)
            .collect();
        let mut rng = rand::thread_rng();
        let index = terminal_nodes[rng.gen_range(0..terminal_nodes.len())];
        let reversed_graph = Reversed(&proto);
        let mut dfs = Dfs::new(&reversed_graph, index);
        let mut nodes_on_path = Vec::new();
        while let Some(node) = dfs.next(&reversed_graph) {
            nodes_on_path.push(node);
            if node == initial {
                break;
            }
        }
        // reverse so that index 0 is the initial node and index len-1 is the terminal node on the path
        nodes_on_path.reverse();

        let next_ir = format!("{IR_BASE}_{next_level}", next_level = level + 1);
        let next_if_label_0 = SwarmLabel {
            cmd: Command::new(&format!("{next_ir}_{CMD_BASE}_0")),
            log_type: vec![EventType::new(&format!("{next_ir}_{E_BASE}_0"))],
            role: Role::new(&next_ir),
        };
        let next_if_label_1 = SwarmLabel {
            cmd: Command::new(&format!("{next_ir}_{CMD_BASE}_1")),
            log_type: vec![EventType::new(&format!("{next_ir}_{E_BASE}_1"))],
            role: Role::new(&next_ir),
        };

        let index = rng.gen_range(0..nodes_on_path.len());
        let source_node = nodes_on_path[index];

        if index == nodes_on_path.len()-1 {
            let next_if_middle = proto.add_node(State::new(&proto.node_count().to_string()));
            let next_if_end = proto.add_node(State::new(&proto.node_count().to_string()));
            proto.add_edge(source_node, next_if_middle, next_if_label_0);
            proto.add_edge(next_if_middle, next_if_end, next_if_label_1);
            nodes_on_path.push(next_if_middle);
            nodes_on_path.push(next_if_end);
        } else {
            let target_node = nodes_on_path[index + 1];
            let edge_to_remove = proto.find_edge(source_node, target_node).unwrap();
            let weight = proto[edge_to_remove].clone();
            let old_size = proto.node_count();
            proto.remove_edge(edge_to_remove);
            let next_if_start = proto.add_node(State::new(&format!("{old_size}")));
            proto.add_edge(source_node, next_if_start, weight);
            let next_if_middle = proto.add_node(State::new(&proto.node_count().to_string()));
            proto.add_edge(next_if_start, next_if_middle, next_if_label_0);
            proto.add_edge(next_if_middle, target_node, next_if_label_1);
            nodes_on_path = vec![nodes_on_path[..index+1].to_vec(), vec![next_if_start, next_if_middle], nodes_on_path[index+1..].to_vec()].concat();
        };

        let ir = format!("{IR_BASE}_{level}");
        let if_label_0 = SwarmLabel {
            cmd: Command::new(&format!("{ir}_{CMD_BASE}_0")),
            log_type: vec![EventType::new(&format!("{ir}_{E_BASE}_0"))],
            role: Role::new(&ir),
        };
        let if_label_1 = SwarmLabel {
            cmd: Command::new(&format!("{ir}_{CMD_BASE}_1")),
            log_type: vec![EventType::new(&format!("{ir}_{E_BASE}_1"))],
            role: Role::new(&ir),
        };

        let new_initial = proto.add_node(State::new(&proto.node_count().to_string()));
        let new_end = proto.add_node(State::new(&proto.node_count().to_string()));
        proto.add_edge(new_initial, initial, if_label_0);
        proto.add_edge(nodes_on_path[nodes_on_path.len()-1], new_end, if_label_1);

        (proto, new_initial)
    }

    // insert graph2 into graph1. that is, find some edge e in graph1.
    // make e terminate at the initial node of graph2.
    // insert all the edges outgoing from the node where e was incoming in the old graph
    // as outgoing edges of some node in graph2.
    // assume graph1 and graph2 have terminal nodes. Assume they both have at least one edge.
    fn insert_into(graph1: (Graph, NodeId), graph2: (Graph, NodeId)) -> (Graph, NodeId) {
        let mut rng = rand::thread_rng();
        let (mut graph1, initial1) = graph1;
        let (mut graph2, initial2) = graph2;
        // map nodes in graph2 to nodes in graph1
        let mut node_map: BTreeMap<NodeId, NodeId> = BTreeMap::new();
        let mut graph2_terminals: Vec<NodeId> = vec![];

        // edge that we attach to initial of graph2 instead of its old target
        let connecting_edge = graph1.edge_references().choose(&mut rng).unwrap();
        let connecting_source = connecting_edge.source();
        let connecting_old_target = connecting_edge.target();
        let connecting_weight = connecting_edge.weight().clone();
        graph1.remove_edge(connecting_edge.id());

        // create a node in graph1 corresponding to initial of graph2
        let inserted_initial = node_map.entry(initial2).or_insert(graph1.add_node(State::new(&graph1.node_count().to_string())));
        graph1.add_edge(connecting_source, *inserted_initial, connecting_weight);

        let mut dfs = Dfs::new(&graph2, initial2);
        while let Some(node) = dfs.next(&graph2) {
            let node_in_graph1 = *node_map.entry(node).or_insert(graph1.add_node(State::new(&graph1.node_count().to_string())));
            for e in graph2.edges_directed(node, Outgoing) {
                let target_in_graph1 = *node_map.entry(e.target()).or_insert(graph1.add_node(State::new(&graph1.node_count().to_string())));
                graph1.add_edge(node_in_graph1, target_in_graph1, e.weight().clone());
            }

            if graph2.edges_directed(node, Outgoing).count() == 0 {
                graph2_terminals.push(node);
            }
        }
        // make all edges starting at connecting_old_target start at this node and remove connecting_old_target
        let connecting_node = *graph2_terminals.choose(&mut rng).unwrap();
        // fill the vectors below, then remove, then add.
        let mut edges_to_remove: Vec<EdgeId> = vec![];
        let mut edges_to_add: Vec<(NodeId, NodeId, SwarmLabel)> = vec![];
        for e in graph1.edges_directed(connecting_old_target, Outgoing) {
            let target = e.target();
            let weight = e.weight();
            edges_to_remove.push(e.id());
            edges_to_add.push((connecting_node, target, weight.clone()));
        }
        for e_id in edges_to_remove {
            graph1.remove_edge(e_id);
        }
        for (source, target, weight) in edges_to_add {
            graph1.add_edge(source, target, weight);
        }


        (graph1, initial1)
    }

    // randomly add edges and nodes to a graph
    fn randomly_expand(graph: Graph, initial: NodeId, max_roles: usize, max_events: usize) -> (Graph, NodeId) {
        unimplemented!()
    }

    #[test]
    fn test_prepare_graph_confusionfree() {
        let composition = get_proto1_proto2_composed();
        let sub = get_proto1_proto2_composed_subs();

        let proto_info = prepare_graph::<Role>(composition, &sub, None);
        assert!(proto_info.get_ith_proto(0).is_some());
        assert!(proto_info.get_ith_proto(0).unwrap().2.is_empty());
        assert_eq!(
            proto_info.concurrent_events,
            BTreeSet::from([unord_event_pair(
                EventType::new("time"),
                EventType::new("car")
            )])
        );
        assert_eq!(
            proto_info.branching_events,
            BTreeSet::from([EventType::new("time"), EventType::new("partID")])
        );
        assert_eq!(proto_info.joining_events, BTreeSet::new());
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
        let proto_info = prepare_graph::<Role>(get_proto1(), &get_subs1(), None);
        assert!(proto_info.get_ith_proto(0).is_some());
        assert!(proto_info.get_ith_proto(0).unwrap().2.is_empty());
        assert_eq!(proto_info.concurrent_events, BTreeSet::new());
        assert_eq!(
            proto_info.branching_events,
            BTreeSet::from([EventType::new("time"), EventType::new("partID")])
        );
        assert_eq!(proto_info.joining_events, BTreeSet::new());

        let proto_info = prepare_graph::<Role>(get_proto2(), &get_subs2(), None);
        assert!(proto_info.get_ith_proto(0).is_some());
        assert!(proto_info.get_ith_proto(0).unwrap().2.is_empty());
        assert_eq!(proto_info.concurrent_events, BTreeSet::new());
        assert_eq!(proto_info.branching_events, BTreeSet::new());
        assert_eq!(proto_info.joining_events, BTreeSet::new());

        let proto_info = prepare_graph::<Role>(get_proto3(), &get_subs3(), None);
        assert!(proto_info.get_ith_proto(0).is_some());
        assert!(proto_info.get_ith_proto(0).unwrap().2.is_empty());
        assert_eq!(proto_info.concurrent_events, BTreeSet::new());
        assert_eq!(
            proto_info.branching_events,
            BTreeSet::from([EventType::new("notOk"), EventType::new("ok")])
        );
        assert_eq!(proto_info.joining_events, BTreeSet::new());
    }

    #[test]
    fn test_prepare_graph_confusionful() {
        let proto1 = get_confusionful_proto1();
        let sub = get_subs1();
        let proto_info = prepare_graph::<Role>(proto1.clone(), &sub, None);
        let mut errors = vec![
            confusion_free(&proto_info, 0),
            proto_info.get_ith_proto(0).unwrap().2,
        ]
        .concat()
        .map(Error::convert(&proto_info.get_ith_proto(0).unwrap().0));

        let mut expected_erros = vec![
            "transition (0)--[close@D<time,time2>]-->(0) emits more than one event type",
            "guard event type partID appears in transitions from multiple states",
            "state 0 can not reach terminal node",
            "state 1 can not reach terminal node",
            "state 2 can not reach terminal node",
        ];
        errors.sort();
        expected_erros.sort();
        assert_eq!(errors, expected_erros);

        let proto2 = get_confusionful_proto2();
        let sub = get_proto1_proto2_composed_subs();
        let proto_info = prepare_graph::<Role>(proto2, &sub, None);
        let errors = vec![
            confusion_free(&proto_info, 0),
            proto_info.get_ith_proto(0).unwrap().2,
        ]
        .concat()
        .map(Error::convert(&proto_info.get_ith_proto(0).unwrap().0));
        let expected_errors = vec![
                "non-deterministic event guard type partID in state 0 || 0",
                "non-deterministic command request for role T in state 0 || 0",
                "event type pos emitted by command in transition (1 || 1)--[get@FL<pos>]-->(2 || 1) and command in transition (2 || 1)--[deliver@T<pos>]-->(0 || 2)"
            ];

        assert_eq!(errors, expected_errors);

        let proto_info = prepare_graph::<Role>(get_confusionful_proto3(), &sub, None);
        let errors = vec![
            confusion_free(&proto_info, 0),
            proto_info.get_ith_proto(0).unwrap().2,
        ]
        .concat()
        .map(Error::convert(&proto_info.get_ith_proto(0).unwrap().0));
        // recorded twice fix this!
        let expected_errors = vec![
            "initial swarm protocol state has no transitions",
            "initial swarm protocol state has no transitions",
        ];
        assert_eq!(errors, expected_errors);

        let proto_info = prepare_graph::<Role>(get_confusionful_proto4(), &sub, None);
        let errors = //vec![
                //confusion_free(&proto_info, 0)
                proto_info.get_ith_proto(0).unwrap().2
            //].concat()
            .map(Error::convert(&proto_info.get_ith_proto(0).unwrap().0));
        // recorded twice fix this!
        let expected_errors = vec![
            "state 2 is unreachable from initial state",
            "state 3 is unreachable from initial state",
            "state 4 is unreachable from initial state",
            "state 5 is unreachable from initial state",
        ];
        assert_eq!(errors, expected_errors);
    }

    #[test]
    fn test_wwf_ok() {
        let (_, _, e) = check(get_proto1(), &get_subs1());
        assert!(e.is_empty());

        let (_, _, e) = check(get_proto2(), &get_subs2());
        assert!(e.is_empty());

        let (_, _, e) = check(get_proto3(), &get_subs3());
        assert!(e.is_empty());

        let (_, _, e) = check(
            get_proto1_proto2_composed(),
            &get_proto1_proto2_composed_subs(),
        );
        assert!(e.is_empty());
    }

    #[test]
    fn test_wwf_fail() {
        let (g, _, e) = check(get_proto1(), &get_subs2());
        let mut errors = e.map(Error::convert(&g));
        errors.sort();
        let mut expected_errors = vec![
            "active role does not subscribe to any of its emitted event types in transition (0)--[close@D<time>]-->(3)",
            "active role does not subscribe to any of its emitted event types in transition (1)--[get@FL<pos>]-->(2)",
            "role D does not subscribe to events in branching transition (0)--[close@D<time>]-->(3)",
            "role D does not subscribe to events in branching transition (0)--[request@T<partID>]-->(1)",
            "role FL does not subscribe to events in branching transition (0)--[close@D<time>]-->(3)",
            "role FL does not subscribe to events in branching transition (0)--[request@T<partID>]-->(1)",
            "role T does not subscribe to events in branching transition (0)--[close@D<time>]-->(3)",
            "subsequently active role D does not subscribe to events in transition (2)--[deliver@T<part>]-->(0)",
            "subsequently active role FL does not subscribe to events in transition (0)--[request@T<partID>]-->(1)",
            "subsequently active role T does not subscribe to events in transition (1)--[get@FL<pos>]-->(2)"
        ];

        expected_errors.sort();
        assert_eq!(errors, expected_errors);

        let (g, _, e) = check(get_proto2(), &get_subs3());
        let mut errors = e.map(Error::convert(&g));
        errors.sort();
        let mut expected_errors = vec![
            "active role does not subscribe to any of its emitted event types in transition (0)--[request@T<partID>]-->(1)",
            "subsequently active role T does not subscribe to events in transition (0)--[request@T<partID>]-->(1)",
            "active role does not subscribe to any of its emitted event types in transition (1)--[deliver@T<part>]-->(2)",
            "subsequently active role F does not subscribe to events in transition (1)--[deliver@T<part>]-->(2)"
        ];

        expected_errors.sort();
        assert_eq!(errors, expected_errors);

        let (g, _, e) = check(get_proto3(), &get_subs1());
        let mut errors = e.map(Error::convert(&g));
        errors.sort();
        let mut expected_errors = vec![
            "active role does not subscribe to any of its emitted event types in transition (0)--[build@F<car>]-->(1)",
            "active role does not subscribe to any of its emitted event types in transition (1)--[test@TR<report>]-->(2)",
            "active role does not subscribe to any of its emitted event types in transition (2)--[accept@QCR<ok>]-->(3)",
            "active role does not subscribe to any of its emitted event types in transition (2)--[reject@QCR<notOk>]-->(3)",
            "role QCR does not subscribe to events in branching transition (2)--[accept@QCR<ok>]-->(3)",
            "role QCR does not subscribe to events in branching transition (2)--[reject@QCR<notOk>]-->(3)",
            "subsequently active role QCR does not subscribe to events in transition (1)--[test@TR<report>]-->(2)",
            "subsequently active role QCR does not subscribe to events in transition (1)--[test@TR<report>]-->(2)",
            "subsequently active role TR does not subscribe to events in transition (0)--[build@F<car>]-->(1)"
        ];

        // fix the duplicate situation. because of active not conc returning labels not set of roles.

        expected_errors.sort();
        assert_eq!(errors, expected_errors);
    }

    #[test]
    fn test_weak_well_formed_sub() {
        let (subs1, errors1) = weak_well_formed_sub(get_proto1());
        let (subs2, errors2) = weak_well_formed_sub(get_proto2());
        let (subs3, errors3) = weak_well_formed_sub(get_proto3());
        let (subs4, errors4) = weak_well_formed_sub(get_proto1_proto2_composed());
        assert_eq!(subs1, get_subs1());
        assert!(errors1.is_empty());
        assert_eq!(subs2, get_subs2());
        assert!(errors2.is_empty());
        assert_eq!(subs3, get_subs3());
        assert!(errors3.is_empty());
        assert_eq!(subs4, get_proto1_proto2_composed_subs());
        assert!(errors4.is_empty());
    }

    #[test]
    fn test_compose_subs() {
        let composition_input = vec![
            CompositionInput {
                protocol: get_proto1(),
                subscription: weak_well_formed_sub(get_proto1()).0,
                interface: None,
            },
            CompositionInput {
                protocol: get_proto2(),
                subscription: weak_well_formed_sub(get_proto2()).0,
                interface: Some(Role::from("T")),
            },
        ];

        let (sub, errors) = compose_subscriptions(composition_input);
        assert!(errors.is_empty());
        let (_, _, e) = check(get_proto1_proto2_composed(), &sub);
        assert!(e.is_empty());

        let composition_input = vec![
            CompositionInput {
                protocol: get_proto1(),
                subscription: weak_well_formed_sub(get_proto1()).0,
                interface: None,
            },
            CompositionInput {
                protocol: get_proto2(),
                subscription: weak_well_formed_sub(get_proto2()).0,
                interface: Some(Role::from("FL")),
            },
        ];

        let (_, errors) = compose_subscriptions(composition_input);
        assert_eq!(
            error_report_to_strings(errors),
            vec![
                "role FL can not be used as interface",
                "event type pos does not appear in both protocols"
            ]
        );
    }

    #[test]
    fn test_explicit_composition() {
        let composition = compose_protocols(get_composition_input_vec1());
        assert!(composition.is_ok());

        let (g, i) = composition.unwrap();
        let swarm = to_swarm_json(g, i);
        let (wwf_sub, _) = compose_subscriptions(get_composition_input_vec1());
        let (_, _, errors) = check(swarm.clone(), &wwf_sub);

        // check if subscription generated using implicit composition is actually wwf for the explicit composition.
        assert!(errors.is_empty());

        let composition = compose_protocols(get_composition_input_vec1()[..2].to_vec());
        assert!(composition.is_ok());

        let (g, i) = composition.unwrap();
        let swarm = to_swarm_json(g, i);
        let (wwf_sub, _) = compose_subscriptions(get_composition_input_vec1()[..2].to_vec());
        let (_, _, errors) = check(swarm.clone(), &wwf_sub);

        // check if subscription generated using implicit composition is actually wwf for the explicit composition.
        assert!(errors.is_empty());
    }

    #[test]
    fn test_compose_non_wwf_swarms() {
        let proto1 = get_proto1();
        let proto2 = get_proto2();
        let subs1: Subscriptions = BTreeMap::from([(Role::new("T"), BTreeSet::new())]);
        let subs2: Subscriptions = BTreeMap::from([(Role::new("F"), BTreeSet::new())]);

        let composition_input: CompositionInputVec = Vec::from([
            CompositionInput {
                protocol: proto1,
                subscription: subs1,
                interface: None,
            },
            CompositionInput {
                protocol: proto2,
                subscription: subs2,
                interface: Some(Role::new("T")),
            },
        ]);
        let (swarms, subscriptions) = implicit_composition_swarms(composition_input);
        let errors1 = vec![
            "active role does not subscribe to any of its emitted event types in transition (0)--[close@D<time>]-->(3)",
            "role D does not subscribe to events in branching transition (0)--[close@D<time>]-->(3)",
            "role FL does not subscribe to events in branching transition (0)--[close@D<time>]-->(3)",
            "role T does not subscribe to events in branching transition (0)--[close@D<time>]-->(3)",
            "active role does not subscribe to any of its emitted event types in transition (0)--[request@T<partID>]-->(1)",
            "subsequently active role FL does not subscribe to events in transition (0)--[request@T<partID>]-->(1)",
            "role D does not subscribe to events in branching transition (0)--[request@T<partID>]-->(1)",
            "role FL does not subscribe to events in branching transition (0)--[request@T<partID>]-->(1)",
            "role T does not subscribe to events in branching transition (0)--[request@T<partID>]-->(1)",
            "active role does not subscribe to any of its emitted event types in transition (1)--[get@FL<pos>]-->(2)",
            "subsequently active role T does not subscribe to events in transition (1)--[get@FL<pos>]-->(2)",
            "active role does not subscribe to any of its emitted event types in transition (2)--[deliver@T<part>]-->(0)",
            "subsequently active role D does not subscribe to events in transition (2)--[deliver@T<part>]-->(0)",
            "subsequently active role T does not subscribe to events in transition (2)--[deliver@T<part>]-->(0)"
        ];

        let errors2 = vec![
            "active role does not subscribe to any of its emitted event types in transition (0)--[request@T<partID>]-->(1)",
            "subsequently active role T does not subscribe to events in transition (0)--[request@T<partID>]-->(1)",
            "active role does not subscribe to any of its emitted event types in transition (1)--[deliver@T<part>]-->(2)",
            "subsequently active role F does not subscribe to events in transition (1)--[deliver@T<part>]-->(2)",
            "active role does not subscribe to any of its emitted event types in transition (2)--[build@F<car>]-->(3)"
        ];

        let errors = vec![errors1, errors2];

        for (((g, _, e), _), expected) in zip(swarms, errors) {
            assert_eq!(e.map(Error::convert(&g)), expected);
        }

        assert!(subscriptions.is_empty());
    }

    // test that we do not generate duplicate labels
    proptest! {
        #[test]
        fn test_all_labels(mut labels in all_labels(10, 10)) {
            labels.sort();
            let mut labels2 = labels.clone().into_iter().collect::<BTreeSet<SwarmLabel>>().into_iter().collect::<Vec<_>>();
            labels2.sort();
            assert_eq!(labels, labels2);
        }
    }

    // test that we only generate confusion free protocols and that going back and forth between swarm and graph does not change the meaning of the protocol
    // lower with_cases arg or max roles and events to make test run faster
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]
        #[test]
        fn test_generate_graph(labels in all_labels(50, 50)) {
            let (graph, initial) = random_graph_shuffle_labels(labels);
            let swarm = to_swarm_json(graph.clone(), initial);
            let (g, i, e) = crate::swarm::from_json(to_swarm_json(graph.clone(), initial), &BTreeMap::new());
            assert!(e.is_empty());
            //println!("swarm: {}", serde_json::to_string_pretty(&swarm).unwrap());
            let swarm1 = to_swarm_json(g, i.unwrap());
            assert_eq!(swarm, swarm1);
            assert!(confusion_free(&prepare_graph::<Role>(swarm, &BTreeMap::new(), None), 0).is_empty());
        }
    }

    proptest! {
        #[test]
        fn test_labels_and_interface((labels, interfacing) in all_labels_and_if(10, 10)) {
            let interfacing_set = interfacing.clone().into_iter().collect::<BTreeSet<_>>();
            let labels_set = labels.into_iter().collect::<BTreeSet<_>>();
            assert!(interfacing_set.is_subset(&labels_set));
            let first = interfacing[0].clone();
            assert!(interfacing[1..].into_iter().all(|label| first.role == label.role));
        }
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
            //println!("explicit size: {} implicit size: {}", subs1[role].len(), subs2[role].len());
            if !subs1[role].is_subset(&subs2[role]) {
                return false;
            }
        }

        true
    }

    // test whether the approximated subscription for compositions
    // is contained within the 'exact' subscription.
    // i.e. is the approximation safe. max five protocols, max five roles
    // in each, max five commands per role. relatively small.
    proptest! {
        //#![proptest_config(ProptestConfig::with_cases(1))]
        #[test]
        fn test_overapprox_1(vec in generate_composition_input_vec(5, 5, 5, false)) {
            let vec: CompositionInputVec = vec
                .into_iter()
                .map(|composition_input| {
                    let (subscription, _) = weak_well_formed_sub(composition_input.protocol.clone());
                    CompositionInput {subscription, ..composition_input}
                })
                .collect();
            let (subs_implicit, errors) = compose_subscriptions(vec.clone());
            assert!(errors.is_empty());
            let result = compose_protocols(vec.clone());
            assert!(result.is_ok());
            let (composed_graph, composed_initial) = result.unwrap();
            // we want to turn it to swarm and call weak_well_well_formed_sub
            // instead of calling wwf_sub with graph because we want to
            // prepare the graph and obtain concurrent events etc.
            let swarm = to_swarm_json(composed_graph.clone(), composed_initial);
            let (subs_explicit, _) = weak_well_formed_sub(swarm.clone());
            assert!(is_sub_subscription(subs_explicit.clone(), subs_implicit));
        }
    }

    // same test as above but for larger compositions. test fewer cases.
    /* proptest! {
        #![proptest_config(ProptestConfig::with_cases(1))]
        #[test]
        fn test_overapprox_2(vec in generate_composition_input_vec(10, 10, 7)) {
            let (subs_implicit, errors) = compose_subscriptions(vec.clone());
            assert!(errors.is_empty());
            let result = compose_protocols(vec.clone());
            assert!(result.is_ok());
            let (composed_graph, composed_initial) = result.unwrap();
            // we want to turn it to swarm and call weak_well_well_formed_sub
            // instead of calling wwf_sub with graph because we want to
            // prepare the graph and obtain concurrent events etc.
            let swarm = to_swarm_json(composed_graph, composed_initial);
            let (subs_explicit, _) = weak_well_formed_sub(swarm);
            assert!(is_sub_subscription(subs_explicit, subs_implicit));
        }
    } */
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1))]
        #[test]
        fn test_refinement_pattern(vec in generate_composition_input_vec_refinement(10, 10, 3)) {
            for v in &vec {
                println!("protocol: {}", serde_json::to_string_pretty(&v.protocol).unwrap());
                println!("protocol: {:?}", v.interface);
            }
            println!("--------");
            let vec: CompositionInputVec = vec
                .into_iter()
                .map(|composition_input| {
                    let (subscription, _) = weak_well_formed_sub(composition_input.protocol.clone());
                    CompositionInput {subscription, ..composition_input}
                })
                .collect();
            let result = compose_protocols(vec.clone());
            assert!(result.is_ok());
            let (composed_graph, composed_initial) = result.unwrap();
            let swarm = to_swarm_json(composed_graph.clone(), composed_initial);
            println!("composition: {}", serde_json::to_string_pretty(&swarm).unwrap());
        }
    }

    // test whether project(compose(G1, G2, ..., Gn)) = compose(project(G1), project(G2), ... project(Gn))
    // have test here instead of in composition_machine.rs because...
    proptest! {
        #[test]
        fn test_project_combine(vec in generate_composition_input_vec(5, 5, 5, false)) {
            let vec: CompositionInputVec = vec
                .into_iter()
                .map(|composition_input| {
                    let (subscription, _) = weak_well_formed_sub(composition_input.protocol.clone());
                    CompositionInput {subscription, ..composition_input}
                })
                .collect();
            let (protos, subs_implicit) = implicit_composition_swarms(vec.clone());
            let protos = protos.into_iter().map(|((g, i, _), set)| (g, i.unwrap(), set)).collect();
            let result = compose_protocols(vec.clone());
            assert!(result.is_ok());
            let (composed_graph, composed_initial) = result.unwrap();
            for role in subs_implicit.keys() {
                let (proj_combined, proj_combined_initial) =
                    project_combine(&protos, &subs_implicit, role.clone());
                let (proj, proj_initial) = project(&composed_graph, composed_initial, &subs_implicit, role.clone());
                assert!(crate::machine::equivalent(
                    &to_option_machine(&proj),
                    proj_initial,
                    &proj_combined,
                    proj_combined_initial.unwrap()
                )
                .is_empty());
            }
        }
    }

    // Confusion free thing come back to this!
    /*     proptest! {
        #![proptest_config(ProptestConfig::with_cases(1))]
        #[test]
        fn test_confusion_free_comp(vec in generate_composition_input_vec(5, 5, 2)) {
            let vec: CompositionInputVec = vec
                .into_iter()
                .map(|composition_input| {
                    let (subscription, _) = weak_well_formed_sub(composition_input.protocol.clone());
                    CompositionInput {subscription, ..composition_input}
                })
                .collect();
            let result = compose_protocols(vec.clone());
            //println!("{:?}", result);
            assert!(result.is_ok());
            /* match result {
                Ok(_) => (),
                Err(e) => println!("{:?}", error_report_to_strings(e)),
            } */
            let (composed_graph, composed_initial) = result.unwrap();
            let swarm = to_swarm_json(composed_graph.clone(), composed_initial);
            let (subs_explicit, e) = weak_well_formed_sub(swarm.clone());
            //
            if !e.is_empty() {
                //println!("e: {:?}", e);
                println!("e strings: {:?}", error_report_to_strings(e));
                /* println!("g: {}", serde_json::to_string_pretty(&swarm).unwrap());
                for v in vec {
                    println!("g component: {}", serde_json::to_string_pretty(&v.protocol).unwrap());
                    println!("g inteface: {:?}", v.interface);
                } */
            }
            //assert!(is_sub_subscription(subs_explicit.clone(), subs_implicit));
        }
    } */
}
