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
        unord_event_pair, CompositionInputVec, EventLabel, EventTypeInfo, ProtoInfo, RoleEventMap,
        SwarmInterface, UnordEventPair,
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
    RoleNotSubscribedToJoin(EventType, EventType, EdgeId, Role),
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
            Error::RoleNotSubscribedToJoin(event_type1, event_type2, edge, role) => {
                format!(
                    "role {role} does not subscribe to concurrent event types {event_type1} and {event_type2} leading to joining event in transition {}",
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

pub fn check(
    proto: SwarmProtocol,
    subs: &Subscriptions,
) -> (Graph, Option<NodeId>, Vec<Error>) {
    let proto_info = prepare_graph::<Role>(proto, subs, None);
    let (graph, initial, mut errors) = match proto_info.get_ith_proto(0) {
        Some((g, Some(i), e)) => (g, i, e),
        Some((g, None, e)) => return (g, None, e),
        _ => return (Graph::new(), None, vec![]),
    };
    errors.extend(all_nodes_reachable(&graph, initial));
    errors.extend(weak_well_formed(proto_info, 0));
    (graph, Some(initial), errors)
}

// Should propagate errors?? COME BACK!
pub fn weak_well_formed_sub(proto: SwarmProtocol) -> Subscriptions {
    let proto_info = prepare_graph::<Role>(proto, &BTreeMap::new(), None);
    wwf_sub(proto_info, 0)
}

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
    if protos_ifs.iter().any(|(proto_info, _)| !proto_info.no_errors()) {
        let result = swarms_to_error_report(protos_ifs.into_iter().flat_map(|(proto_info, _)| proto_info.protocols).collect());
        return Err(result);
    }
    // construct this to check whether the protocols interface
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

/*
 * A graph that was constructed with prepare_graph with no errors will have one event type per command.
 * Similarly, such a graph will be weakly confusion free, which means we do not have to check for
 * command and log determinism like we do in swarm::well_formed.
 *
 */
fn weak_well_formed(proto_info: ProtoInfo, proto_pointer: usize) -> Vec<Error> {
    // copied from swarm::well_formed
    let mut errors = Vec::new();
    let empty = BTreeSet::new(); // just for `sub` but needs its own lifetime
    let sub = |r: &Role| proto_info.subscription.get(r).unwrap_or(&empty);
    let (graph, initial, _) = match proto_info.get_ith_proto(proto_pointer) {
        Some((g, Some(i), e)) => (g, i, e),
        Some((_, None, e)) => return e,
        None => return vec![Error::InvalidArg],
    };

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
            // subscribe to event immediately preceeding
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
            if proto_info.joining_events.contains(&event_type) {
                for incoming_pair in event_pairs_from_node(node, &graph, Incoming) {
                    if proto_info.concurrent_events.contains(&incoming_pair) {
                        let join_set: BTreeSet<EventType> = incoming_pair
                            .union(&BTreeSet::from([event_type.clone()]))
                            .cloned()
                            .collect();
                        let involved_not_subbed = involved_roles
                            .iter()
                            .filter(|r| !join_set.is_subset(sub(r)));
                        let pair_vec: Vec<_> = incoming_pair.into_iter().collect();
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
                }
            }
        }
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

            // weak causal consistency 2: a role subscribes to events that immediately preceedes its own commands
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
            if proto_info.joining_events.contains(&event_type) {
                let events_to_add: BTreeSet<_> = event_pairs_from_node(node, &graph, Incoming)
                    .into_iter()
                    .filter(|pair| proto_info.concurrent_events.contains(pair))
                    .flat_map(|pair| pair)
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
        return ProtoInfo::new(
            protocols,
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
        );
    }

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
    let joining_events: BTreeSet<EventType> = proto_info1
        .joining_events
        .union(&proto_info2.joining_events)
        .cloned()
        .collect();
    ProtoInfo::new(
        protocols,
        role_event_map,
        subscription,
        concurrent_events,
        branching_events,
        joining_events,
    )
}

// The result<error, proto> thing here...
fn implicit_composition_fold<T: SwarmInterface>(protos: Vec<(ProtoInfo, Option<T>)>) -> ProtoInfo {
    if protos.is_empty()
        || protos[0].1.is_some()
        || protos[1..].iter().any(|(_, interface)| interface.is_none())
    {
        return ProtoInfo::new(
            vec![(
                (Graph::new(), None, vec![Error::InvalidArg]),
                BTreeSet::new(),
            )],
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
        );
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

// consider changing to (graph, vec error, btreemap... ) instead of swarm protocol. then call with swarm_to_graph()
fn prepare_graph<T: SwarmInterface>(
    proto: SwarmProtocol,
    subs: &Subscriptions,
    interface: Option<T>,
) -> ProtoInfo {
    let mut event_to_command_map = BTreeMap::new();
    let mut role_event_map: RoleEventMap = BTreeMap::new();
    let mut branching_events = BTreeSet::new();
    let mut joining_events: BTreeSet<EventType> = BTreeSet::new();
    let (graph, mut errors, nodes) = swarm_to_graph(&proto);

    let initial = if let Some(idx) = nodes.get(&proto.initial) {
        *idx
    } else {
        errors.push(Error::SwarmError(
            crate::swarm::Error::InitialStateDisconnected,
        ));

        return ProtoInfo::new(
            vec![((graph, None, errors), BTreeSet::new())],
            BTreeMap::new(),
            subs.clone(),
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
        );
    };

    let concurrent_events = all_concurrent_pairs(&graph);
    // if graph contains no concurrency, make old confusion freeness check. requires us to call swarm::prepare_graph through swarm::from_json
    let (graph, initial, mut errors) = if concurrent_events.is_empty() {
        let (graph, initial, e) = match crate::swarm::from_json(proto, subs) {
            (g, Some(i), e) => (g, i, e),
            (g, None, e) => {
                let errors = e.into_iter().map(|s| Error::SwarmErrorString(s)).collect();
                return ProtoInfo::new(
                    vec![((g, None, errors), BTreeSet::new())],
                    BTreeMap::new(),
                    subs.clone(),
                    concurrent_events,
                    BTreeSet::new(),
                    BTreeSet::new(),
                );
            }
        };
        (
            graph,
            initial,
            vec![
                errors,
                e.into_iter().map(|s| Error::SwarmErrorString(s)).collect(),
            ]
            .concat(),
        )
    } else {
        (graph, initial, errors)
    };

    let no_empty_logs = no_empty_log_errors(&errors);

    let mut walk = Dfs::new(&graph, initial);

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

        // add joining events. if there are concurrent incoming edges, add the event types of all outgoing edges to set of joining events.
        if incoming_pairs
            .into_iter()
            .any(|pair| concurrent_events.contains(&pair))
        {
            joining_events.append(
                &mut graph
                    .edges_directed(node_id, Outgoing)
                    .map(|e| e.weight().get_event_type())
                    .collect(),
            );
        }

        // weak confusion freeness checks
        let mut target_map: BTreeMap<SwarmLabel, NodeId> = BTreeMap::new();
        for e in graph.edges_directed(node_id, Outgoing) {
            // rule 1 check. Event types only associated with one command role pair.
            // TEST THIS
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

    for event_type in event_to_command_map.keys() {
        let (role, cmd, _) = event_to_command_map[event_type].clone();
        let e_info = EventTypeInfo::new(cmd, event_type.clone(), role.clone());
        role_event_map
            .entry(role)
            .and_modify(|v| {
                v.insert(e_info.clone());
            })
            .or_insert(BTreeSet::from([e_info]));
    }

    let initial = no_empty_logs.then(|| initial);
    // Set interface field. If interface is some, then we want to interface this protocol
    // with some other protocol on this set of events.
    // We do not know if we can do that yet though.
    let interface = if interface.is_some() {
        interface.unwrap().interfacing_event_types_single(&graph)
    } else {
        BTreeSet::new()
    };
    ProtoInfo::new(
        vec![((graph, initial, errors), interface)],
        role_event_map,
        subs.clone(),
        concurrent_events,
        branching_events,
        joining_events,
    )
}

fn swarm_to_graph(proto: &SwarmProtocol) -> (Graph, Vec<Error>, BTreeMap<State, NodeId>) {
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

    (graph, errors, nodes)
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

fn no_empty_log_errors(errors: &Vec<Error>) -> bool {
    for e in errors {
        match e {
            Error::SwarmError(crate::swarm::Error::LogTypeEmpty(_)) => return false,
            _ => (),
        }
    }
    true
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
    let branching_events = proto_info1
        .branching_events
        .union(&proto_info2.branching_events)
        .cloned()
        .collect();
    let extra = interfacing_events
        .union(&branching_events)
        .cloned()
        .collect();

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

    concurrent_events_union.union(&cartesian_product).cloned().collect()
}


// precondition: the protocols can interface on the given interfaces
fn explicit_composition(proto_info: ProtoInfo) -> (Graph, NodeId) {
    if proto_info.protocols.is_empty() {
        return (Graph::new(), NodeId::end());
    }

    let ((g, i, _, ), _)  = proto_info.protocols[0].clone();
    let folder = |(acc_g, acc_i) : (Graph, NodeId), ((g, i, _), interface): ((Graph, Option<NodeId>, Vec<Error>), BTreeSet<EventType>)| -> (Graph, NodeId) {
        crate::composition::composition_machine::compose(acc_g, acc_i, g, i.unwrap(), interface)
    };
    proto_info.protocols[1..].to_vec().into_iter().fold((g, i.unwrap()), folder)
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
    use crate::{
        composition::{composition_types::CompositionInput, error_report_to_strings},
        types::Command,
        MapVec,
    };

    use super::*;
    use proptest::prelude::*;
    use rand::prelude::*;

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
    fn get_confusionful_proto2() -> SwarmProtocol {
        serde_json::from_str::<SwarmProtocol>(
            r#"{
                "initial": "0 || 0",
                "transitions": [
                    { "source": "0 || 0", "target": "1 || 1", "label": { "cmd": "request", "logType": ["partID"], "role": "T" } },
                    { "source": "0 || 0", "target": "3 || 0", "label": { "cmd": "close", "logType": ["time"], "role": "D" } },
                    { "source": "1 || 1", "target": "2 || 1", "label": { "cmd": "get", "logType": ["pos"], "role": "FL" } },
                    { "source": "2 || 1", "target": "0 || 2", "label": { "cmd": "deliver", "logType": ["partID"], "role": "T" } },
                    { "source": "0 || 2", "target": "0 || 3", "label": { "cmd": "build", "logType": ["car"], "role": "F" } },
                    { "source": "0 || 2", "target": "3 || 2", "label": { "cmd": "close", "logType": ["time"], "role": "D" } },
                    { "source": "0 || 3", "target": "3 || 3", "label": { "cmd": "close", "logType": ["time"], "role": "D" } },
                    { "source": "3 || 2", "target": "3 || 3", "label": { "cmd": "build", "logType": ["car"], "role": "F" } }
                ]
            }"#,
        )
        .unwrap()
    }

    fn get_composition_input_vec1() -> CompositionInputVec {
        vec![
            CompositionInput {
                protocol: get_proto1(),
                subscription: weak_well_formed_sub(get_proto1()),
                interface: None,
            },
            CompositionInput {
                protocol: get_proto2(),
                subscription: weak_well_formed_sub(get_proto2()),
                interface: Some(Role::new("T")),
            },
            CompositionInput {
                protocol: get_proto3(),
                subscription: weak_well_formed_sub(get_proto3()),
                interface: Some(Role::new("F")),
            },
        ]
    }

    prop_compose! {
        fn vec_swarm_label(role: Role, max_events: usize)(vec in prop::collection::vec(("cmd", "e"), 1..max_events)) -> Vec<SwarmLabel> {
            vec
            .into_iter()
            .enumerate()
            .map(|(i, (cmd, event))|
                SwarmLabel { cmd: Command::new(&format!("{role}_{cmd}_{i}")), log_type: vec![EventType::new(&format!("{role}_{event}_{i}"))], role: role.clone()})
            .collect()
        }
    }

    prop_compose! {
        fn vec_role(max_roles: usize)(vec in prop::collection::vec("R", 1..max_roles)) -> Vec<Role> {
            vec
            .into_iter()
            .enumerate()
            .map(|(i, role)| Role::new(&format!("{role}{i}"))).collect()
        }
    }
    prop_compose! {
        fn all_labels(max_roles: usize, max_events: usize)
                    (r in vec_role(max_roles))
                    (labels in r.into_iter().map(|role| vec_swarm_label(role, max_events)).collect::<Vec<_>>()) -> Vec<SwarmLabel> {
            labels.concat()
        }
    }

    // make another one for generating interfacing.
    // this one skewed towards having more branches in the first few nodes??
    prop_compose! {
        // add option (role, set<SwarmLabel>) to first parameter list. interface with these if some.
        // consider changing prepare graph to not have to switch around between json thing and graph all the time in tests.
        fn generate_graph(max_roles: usize, max_events: usize)(mut swarm_labels in all_labels(max_roles, max_events)) -> (Graph, NodeId, BTreeMap<Role, BTreeSet<SwarmLabel>>) {
            let mut graph = Graph::new();
            let mut map: BTreeMap<Role, BTreeSet<SwarmLabel>> = BTreeMap::new();
            let mut nodes = Vec::new();
            let mut rng = rand::thread_rng();
            let gen_state_name = |g: &Graph| -> State { State::new(&g.node_count().to_string()) };
            let add_to_map = |label: &SwarmLabel, m: &mut BTreeMap<Role, BTreeSet<SwarmLabel>>| { m.entry(label.role.clone()).and_modify(|events| { events.insert(label.clone()); } ).or_insert(BTreeSet::from([label.clone()])); };
            swarm_labels.shuffle(&mut rng);
            let initial = graph.add_node(State::new(&graph.node_count().to_string()));
            nodes.push(initial);

            while let Some(label) = swarm_labels.pop() {
                //map.entry(label.role.clone()).and_modify(|events| { events.insert(label.get_event_type()); } ).or_insert(BTreeSet::from([label.get_event_type()]));
                add_to_map(&label, &mut map);
                // consider bernoulli thing. and distrbutions etc. bc documentations says that these once are optimised for cases where only a single sample is needed... if just faster does not matter
                // generate new or select old source? Generate new or select old, generate new target or select old?
                // same because you would have to connect to graph at some point anyway...?
                // exclusive range upper limit
                let source_node = if rng.gen_bool(1.0/10.0) {
                    nodes[rng.gen_range(0..nodes.len())]
                } else {
                    // this whole thing was to have fewer branches... idk. loop will terminate because we always can reach 0?
                    let mut source =  nodes[rng.gen_range(0..nodes.len())];
                    while graph.edges_directed(source, Outgoing).count() > 0 {
                        source = nodes[rng.gen_range(0..nodes.len())];
                    }

                    source
                };



                // if generated bool then select an existing node as target
                // otherwise generate a new node as target
                if rng.gen_bool(1.0/10.0) && !swarm_labels.is_empty() {
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
                        add_to_map(&new_weight, &mut map);
                        graph.add_edge(target_node, new_target_node, new_weight);
                    }
                } else {
                    let target_node = graph.add_node(gen_state_name(&graph));
                    nodes.push(target_node);
                    graph.add_edge(source_node, target_node, label);
                }
            }


            (graph, initial, map)
        }
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
                    EventTypeInfo::new(
                        Command::new("deliver"),
                        EventType::new("part"),
                        Role::new("T"),
                    ),
                    EventTypeInfo::new(
                        Command::new("request"),
                        EventType::new("partID"),
                        Role::new("T"),
                    ),
                ]),
            ),
            (
                Role::from("FL"),
                BTreeSet::from([EventTypeInfo::new(
                    Command::new("get"),
                    EventType::new("pos"),
                    Role::new("FL"),
                )]),
            ),
            (
                Role::from("D"),
                BTreeSet::from([EventTypeInfo::new(
                    Command::new("close"),
                    EventType::new("time"),
                    Role::new("D"),
                )]),
            ),
            (
                Role::from("F"),
                BTreeSet::from([EventTypeInfo::new(
                    Command::new("build"),
                    EventType::new("car"),
                    Role::new("F"),
                )]),
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
        let mut errors = proto_info
            .get_ith_proto(0)
            .unwrap()
            .2
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
        let errors = proto_info
            .get_ith_proto(0)
            .unwrap()
            .2
            .map(Error::convert(&proto_info.get_ith_proto(0).unwrap().0));
        let expected_errors = vec!["event type partID emitted by command in transition (0 || 0)--[request@T<partID>]-->(1 || 1) and command in transition (2 || 1)--[deliver@T<partID>]-->(0 || 2)"];
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
        assert_eq!(weak_well_formed_sub(get_proto1()), get_subs1());
        assert_eq!(weak_well_formed_sub(get_proto2()), get_subs2());
        assert_eq!(weak_well_formed_sub(get_proto3()), get_subs3());
        assert_eq!(
            weak_well_formed_sub(get_proto1_proto2_composed()),
            get_proto1_proto2_composed_subs()
        );
    }

    #[test]
    fn test_compose_subs() {
        let composition_input = vec![
            CompositionInput {
                protocol: get_proto1(),
                subscription: weak_well_formed_sub(get_proto1()),
                interface: None,
            },
            CompositionInput {
                protocol: get_proto2(),
                subscription: weak_well_formed_sub(get_proto2()),
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
                subscription: weak_well_formed_sub(get_proto1()),
                interface: None,
            },
            CompositionInput {
                protocol: get_proto2(),
                subscription: weak_well_formed_sub(get_proto2()),
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

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1))]
        #[test]
        fn test_vec_role(vec in vec_role(10)) {
            for (i, r) in vec.iter().enumerate() {
                println!("ROLE IS: {:?}", r);
                assert_eq!(*r, Role::new(&format!("R{}", i)));
            }
        }
    }

    proptest! {
        //#![proptest_config(ProptestConfig::with_cases(1))]
        #[test]
        fn test_all_labels(labels in all_labels(10, 10)) {
            /* for l in labels {
                println!("label: {:?}", l);
            } */

            let labels2 = labels.clone().into_iter().collect::<BTreeSet<SwarmLabel>>().into_iter().collect::<Vec<_>>();
            //let sl = SwarmLabel{role: Role::new(""), log_type: vec![], cmd: Command::new("")};
            //assert_eq!(labels, vec![lables2, vec![sl]].concat());
            assert_eq!(labels, labels2);
        }
    }

    // For printing:
    /* proptest! {
        #![proptest_config(ProptestConfig::with_cases(15))]
        #[test]
        fn test_generate_graph((graph, initial, _) in generate_graph(10, 10)) {
            let swarm = to_swarm_json(graph, initial);
            println!("{}\n$$$$\n", serde_json::to_string_pretty(&swarm).unwrap());
            let proto_info = prepare_graph::<Role>(swarm, &BTreeMap::new(), None);
            let g = proto_info.get_ith_proto(0).unwrap();
            assert_eq!(g.2, vec![]);

        }
    } */
}
