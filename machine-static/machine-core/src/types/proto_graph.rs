use itertools::Itertools;
use petgraph::{
    Direction::{self, Outgoing},
    visit::{Dfs, EdgeRef, GraphBase, Reversed, Walker},
};
use std::collections::{BTreeMap, BTreeSet};

use crate::types::{proto_info, unordered_event_pair::UnordEventPair};
use crate::{
    errors::Error,
    types::{
        proto_info::ProtoStruct,
        typescript_types::{EventLabel, EventType, State, SwarmLabel, SwarmProtocolType},
    },
};

pub type Graph = petgraph::Graph<State, SwarmLabel>;
pub type NodeId = <petgraph::Graph<(), ()> as GraphBase>::NodeId;
pub type EdgeId = <petgraph::Graph<(), ()> as GraphBase>::EdgeId;

// turn a SwarmProtocol into a petgraph. perform some checks that are not strictly related to wf, but must be successful for any further analysis to take place
pub fn swarm_to_graph(proto: &SwarmProtocolType) -> (Graph, Option<NodeId>, Vec<Error>) {
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
            errors.push(Error::LogTypeEmpty(edge));
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
        errors.push(Error::InitialStateDisconnected);
        None
    };
    (graph, initial, errors)
}

// Consider this one... is it needed?
pub fn from_json(proto: SwarmProtocolType) -> (Graph, Option<NodeId>, Vec<String>) {
    let _span = tracing::info_span!("from_json").entered();
    let proto_info = proto_info::prepare_proto_info(proto);
    let (g, i, e) = match proto_info.get_ith_proto(0) {
        Some(ProtoStruct {
            graph: g,
            initial: i,
            errors: e,
            roles: _,
        }) => (g, i, e),
        _ => return (Graph::new(), None, vec![]),
    };
    let e = e.into_iter().map(Error::convert(&g)).collect();
    (g, i, e)
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
        .map(|node| Error::StateUnreachable(node))
        .collect()
}

// Given some node, return the swarmlabels going out of that node that are not concurrent with 'event_type'.
pub fn active_transitions_not_conc(
    node: NodeId,
    graph: &Graph,
    event_type: &EventType,
    concurrent_events: &BTreeSet<UnordEventPair>,
) -> Vec<SwarmLabel> {
    graph
        .edges_directed(node, Outgoing)
        .map(|e| e.weight().clone())
        .filter(|e| {
            !concurrent_events.contains(&UnordEventPair::new(event_type.clone(), e.get_event_type()))
        })
        .collect()
}

// Return all event types that are part of an infinte loop in a graph (according to succ_map).
pub fn infinitely_looping_event_types(
    graph: &Graph,
    succ_map: &BTreeMap<EventType, BTreeSet<EventType>>,
) -> BTreeSet<EventType> {
    let _span = tracing::info_span!("infinitely_looping_event_types").entered();
    let nodes = nodes_not_reaching_terminal(graph);
    nodes
        .into_iter()
        .flat_map(|n| {
            graph
                .edges_directed(n, Outgoing)
                .map(|e| e.weight().get_event_type())
                .filter(|t| succ_map.contains_key(t) && succ_map[t].contains(t))
        })
        .collect()
}

fn nodes_not_reaching_terminal(graph: &Graph) -> Vec<NodeId> {
    let _span = tracing::info_span!("nodes_not_reaching_terminal").entered();
    // All terminal nodes
    let terminal_nodes: Vec<_> = graph
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
    let can_reach_terminal_nodes: BTreeSet<_> = terminal_nodes
        .into_iter()
        .map(get_predecessors)
        .flatten()
        .collect();

    // Collect nodes that can not reach a terminal node and transform to a vec of errors.
    graph
        .node_indices()
        .into_iter()
        .filter(|node| !can_reach_terminal_nodes.contains(node))
        .collect()
}

// all pairs of incoming/outgoing events from a node
pub fn event_pairs_from_node(
    node: NodeId,
    graph: &Graph,
    direction: Direction,
) -> Vec<UnordEventPair> {
    graph
        .edges_directed(node, direction)
        .map(|e| e.id())
        .combinations(2)
        .map(|pair| {
            UnordEventPair::new(
                graph[pair[0]].get_event_type(),
                graph[pair[1]].get_event_type(),
            )
        }) //BTreeSet::from([graph[pair[0]].get_event_type(), graph[pair[1]].get_event_type()]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils;
    use crate::types::typescript_types::StateName;

    mod loop_tests {
        use super::*;
        // This module contains tests for relating to looping event types.
        #[test]
        fn looping_1() {
            test_utils::setup_logger();
            // Check states that can not reach terminal state an infinitely looping event types
            let (graph, _, _) = swarm_to_graph(&test_utils::get_looping_proto_1());
            let states_not_reaching_terminal = nodes_not_reaching_terminal(&graph);
            let state_names: Vec<String> = states_not_reaching_terminal
                .into_iter()
                .map(|n| graph[n].state_name().to_string())
                .collect::<Vec<_>>();
            assert_eq!(state_names, ["2", "3", "4"]);
        }

        #[test]
        fn looping_2() {
            test_utils::setup_logger();
            // Check states that can not reach terminal state an infinitely looping event types
            let (graph, _, _) = swarm_to_graph(&test_utils::get_looping_proto_2());
            let states_not_reaching_terminal = nodes_not_reaching_terminal(&graph);
            let state_names: Vec<String> = states_not_reaching_terminal
                .into_iter()
                .map(|n| graph[n].state_name().to_string())
                .collect::<Vec<_>>();
            assert_eq!(state_names, ["2", "3", "4"]);
        }

        #[test]
        fn looping_3() {
            test_utils::setup_logger();
            // Check states that can not reach terminal state an infinitely looping event types
            let (graph, _, _) = swarm_to_graph(&test_utils::get_looping_proto_3());
            let states_not_reaching_terminal = nodes_not_reaching_terminal(&graph);
            let state_names: Vec<String> = states_not_reaching_terminal
                .into_iter()
                .map(|n| graph[n].state_name().to_string())
                .collect::<Vec<_>>();
            assert_eq!(state_names, ["0", "1", "2", "3", "4", "5", "6", "7"]);
        }

        #[test]
        fn looping_4() {
            test_utils::setup_logger();
            // Check states that can not reach terminal state an infinitely looping event types
            let (graph, _, _) = swarm_to_graph(&test_utils::get_looping_proto_4());
            let states_not_reaching_terminal = nodes_not_reaching_terminal(&graph);
            let state_names: Vec<String> = states_not_reaching_terminal
                .into_iter()
                .map(|n| graph[n].state_name().to_string())
                .collect::<Vec<_>>();
            assert_eq!(state_names, ["0", "1", "2", "3", "4", "5", "6", "7"]);
        }

        #[test]
        fn looping_5() {
            test_utils::setup_logger();
            // Check states that can not reach terminal state an infinitely looping event types
            let (graph, _, _) = swarm_to_graph(&test_utils::get_looping_proto_5());
            let states_not_reaching_terminal = nodes_not_reaching_terminal(&graph);
            let state_names: Vec<String> = states_not_reaching_terminal
                .into_iter()
                .map(|n| graph[n].state_name().to_string())
                .collect::<Vec<_>>();
            assert_eq!(state_names, ["0", "1", "2", "3", "4", "5", "6"]);
        }

        #[test]
        fn looping_6() {
            test_utils::setup_logger();
            // Check states that can not reach terminal state an infinitely looping event types
            let (graph, _, _) = swarm_to_graph(&test_utils::get_looping_proto_6());
            let states_not_reaching_terminal = nodes_not_reaching_terminal(&graph);
            let state_names: Vec<String> = states_not_reaching_terminal
                .into_iter()
                .map(|n| graph[n].state_name().to_string())
                .collect::<Vec<_>>();
            assert_eq!(state_names, ["0", "1", "2", "3", "4"]);
        }
    }
}
