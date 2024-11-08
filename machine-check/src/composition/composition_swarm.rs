use crate::{
    types::{EventType, Role, State, StateName, SwarmLabel, Transition},
    EdgeId, MapVec, NodeId, Subscriptions, SwarmProtocol,
};
use itertools::Itertools;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use super::{
    composition_types::{
        unord_event_pair, EventLabel, EventTypeInfo, ProtoInfo, RoleEventMap, UnordEventPair,
    },
    Graph,
};
use petgraph::{
    graph::EdgeReference,
    visit::{Dfs, EdgeRef, IntoEdgesDirected, Walker},
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

pub fn check(
    proto: SwarmProtocol,
    subs: &Subscriptions,
) -> (super::super::Graph, Option<NodeId>, Vec<Error>) {
    let proto_info = prepare_graph(proto, subs);
    let (graph, initial, mut errors) = match proto_info.get_ith(0) {
        Some((g, Some(i), e)) => (g, i, e),
        Some((g, None, e)) => return (g, None, e),
        _ => return (Graph::new(), None, vec![]),
    };
    //errors.extend(super_error_wrapper(all_nodes_reachable(&graph, initial))); // TODO
    errors.extend(weak_well_formed(proto_info, 0));
    (graph, Some(initial), errors)
}

// Should propagate errors?? COME BACK!
pub fn weak_well_formed_sub(
    proto: SwarmProtocol,
) -> Subscriptions {
    let proto_info = prepare_graph(proto, &BTreeMap::new());
    wwf_sub(proto_info, 0)
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
    let (graph, initial, _) = match proto_info.get_ith(proto_pointer) {
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
            // active transitions gets the transitions going out of edge.target() and filters out the ones emitting events concurrent with event type
            for successor in active_transitions_not_conc(
                node,
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
    let (graph, initial, _) = match proto_info.get_ith(proto_pointer) {
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
            for active in
                active_transitions_not_conc(edge.target(), &graph, &event_type, &proto_info.concurrent_events)
            {
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
                let events_to_add: BTreeSet<_> = event_pairs_from_node(node, &graph, Incoming).into_iter().filter(|pair| proto_info.concurrent_events.contains(pair)).flat_map(|pair| pair).chain([event_type.clone()]).collect();
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

fn prepare_graph(proto: SwarmProtocol, subs: &Subscriptions) -> ProtoInfo {
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
            vec![(graph, None, errors)],
            BTreeMap::new(),
            subs.clone(),
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            vec![],
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
                    vec![(g, None, errors)],
                    BTreeMap::new(),
                    subs.clone(),
                    concurrent_events,
                    BTreeSet::new(),
                    BTreeSet::new(),
                    vec![],
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

    for even_type in event_to_command_map.keys() {
        let (role, cmd, _) = event_to_command_map[even_type].clone();
        let e_info = EventTypeInfo {
            event_type: even_type.clone(),
            role: role.clone(),
            cmd,
        };
        role_event_map
            .entry(role)
            .and_modify(|v| v.push(e_info.clone()))
            .or_insert(vec![e_info]);
    }

    let initial = no_empty_logs.then(|| initial);

    ProtoInfo::new(
        vec![(graph, initial, errors)],
        role_event_map,
        subs.clone(),
        concurrent_events,
        branching_events,
        joining_events,
        vec![],
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

fn no_empty_log_errors(errors: &Vec<Error>) -> bool {
    for e in errors {
        match e {
            Error::SwarmError(crate::swarm::Error::LogTypeEmpty(_)) => return false,
            _ => (),
        }
    }
    true
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

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_prepare_graph_confusionfree() {
        let composition = get_proto1_proto2_composed();
        let sub = BTreeMap::from([
            (
                Role::from("T"),
                BTreeSet::from([
                    EventType::from("partID"),
                    EventType::from("part"),
                    EventType::from("pos"),
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

        let proto_info = prepare_graph(composition, &sub);
        assert!(proto_info.get_ith(0).is_some());
        assert!(proto_info.get_ith(0).unwrap().2.is_empty());
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

        let proto_info = prepare_graph(get_proto1(), &get_subs1());
        assert!(proto_info.get_ith(0).is_some());
        assert!(proto_info.get_ith(0).unwrap().2.is_empty());
        assert_eq!(proto_info.concurrent_events, BTreeSet::new());
        assert_eq!(
            proto_info.branching_events,
            BTreeSet::from([EventType::new("time"), EventType::new("partID")])
        );
        assert_eq!(proto_info.joining_events, BTreeSet::new());

        let proto_info = prepare_graph(get_proto2(), &get_subs2());
        assert!(proto_info.get_ith(0).is_some());
        assert!(proto_info.get_ith(0).unwrap().2.is_empty());
        assert_eq!(proto_info.concurrent_events, BTreeSet::new());
        assert_eq!(proto_info.branching_events, BTreeSet::new());
        assert_eq!(proto_info.joining_events, BTreeSet::new());

        let proto_info = prepare_graph(get_proto3(), &get_subs3());
        assert!(proto_info.get_ith(0).is_some());
        assert!(proto_info.get_ith(0).unwrap().2.is_empty());
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
        let proto_info = prepare_graph(proto1.clone(), &sub);
        let mut errors = proto_info
            .get_ith(0)
            .unwrap()
            .2
            .map(Error::convert(&proto_info.get_ith(0).unwrap().0));
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

        let proto_info = prepare_graph(proto2, &sub);
        let errors = proto_info
            .get_ith(0)
            .unwrap()
            .2
            .map(Error::convert(&proto_info.get_ith(0).unwrap().0));
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

        let (_, _, e) = check(get_proto1_proto2_composed(), &get_proto1_proto2_composed_subs());
        assert!(e.is_empty());
    }

    #[test]
    fn test_wwf_fail() {
        let (g, _, e) = check(get_proto1(), &get_subs2());
        let mut errors = e.map(Error::convert(&g));
        errors.sort();
        let mut expected_errors = vec![
            "active role does not subscribe to any of its emitted event types in transition (0)--[close@D<time>]-->(3)",
            "subsequently active role D does not subscribe to events in transition (0)--[close@D<time>]-->(3)",
            "subsequently active role T does not subscribe to events in transition (0)--[close@D<time>]-->(3)",
            "role D does not subscribe to events in branching transition (0)--[close@D<time>]-->(3)",
            "role FL does not subscribe to events in branching transition (0)--[close@D<time>]-->(3)",
            "role T does not subscribe to events in branching transition (0)--[close@D<time>]-->(3)",
            "subsequently active role D does not subscribe to events in transition (0)--[request@T<partID>]-->(1)",
            "role D does not subscribe to events in branching transition (0)--[request@T<partID>]-->(1)",
            "role FL does not subscribe to events in branching transition (0)--[request@T<partID>]-->(1)",
            "active role does not subscribe to any of its emitted event types in transition (1)--[get@FL<pos>]-->(2)",
            "subsequently active role FL does not subscribe to events in transition (1)--[get@FL<pos>]-->(2)",
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
            "subsequently active role T does not subscribe to events in transition (1)--[deliver@T<part>]-->(2)"
        ];

        expected_errors.sort();
        assert_eq!(errors, expected_errors);

        let (g, _, e) = check(get_proto3(), &get_subs1());
        let mut errors = e.map(Error::convert(&g));
        errors.sort();
        let mut expected_errors = vec![
            "active role does not subscribe to any of its emitted event types in transition (0)--[build@F<car>]-->(1)",
            "subsequently active role F does not subscribe to events in transition (0)--[build@F<car>]-->(1)",
            "active role does not subscribe to any of its emitted event types in transition (1)--[test@TR<report>]-->(2)",
            "subsequently active role TR does not subscribe to events in transition (1)--[test@TR<report>]-->(2)",
            "active role does not subscribe to any of its emitted event types in transition (2)--[reject@QCR<notOk>]-->(3)",
            "subsequently active role QCR does not subscribe to events in transition (2)--[reject@QCR<notOk>]-->(3)",
            "subsequently active role QCR does not subscribe to events in transition (2)--[reject@QCR<notOk>]-->(3)",
            "role QCR does not subscribe to events in branching transition (2)--[reject@QCR<notOk>]-->(3)",
            "active role does not subscribe to any of its emitted event types in transition (2)--[accept@QCR<ok>]-->(3)",
            "subsequently active role QCR does not subscribe to events in transition (2)--[accept@QCR<ok>]-->(3)",
            "subsequently active role QCR does not subscribe to events in transition (2)--[accept@QCR<ok>]-->(3)",
            "role QCR does not subscribe to events in branching transition (2)--[accept@QCR<ok>]-->(3)"
        ];

        expected_errors.sort();
        assert_eq!(errors, expected_errors);
    }

    #[test]
    fn test_weak_well_formed_sub() {
        assert_eq!(weak_well_formed_sub(get_proto1()), get_subs1());
        assert_eq!(weak_well_formed_sub(get_proto2()), get_subs2());
        assert_eq!(weak_well_formed_sub(get_proto3()), get_subs3());
        assert_eq!(weak_well_formed_sub(get_proto1_proto2_composed()), get_proto1_proto2_composed_subs());
    }
}
