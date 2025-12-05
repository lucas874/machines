use std::collections::{BTreeMap, BTreeSet};
use petgraph::{
    visit::{GraphBase, Dfs, Walker},
};
use crate::{errors::composition_errors, errors::swarm_errors, types::typescript_types::{State, SwarmLabel, SwarmProtocolType}};

pub type Graph = petgraph::Graph<State, SwarmLabel>;
pub type NodeId = <petgraph::Graph<(), ()> as GraphBase>::NodeId;
pub type EdgeId = <petgraph::Graph<(), ()> as GraphBase>::EdgeId;

// turn a SwarmProtocol into a petgraph. perform some checks that are not strictly related to wf, but must be successful for any further analysis to take place
pub fn swarm_to_graph(proto: &SwarmProtocolType) -> (Graph, Option<NodeId>, Vec<composition_errors::Error>) {
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
            errors.push(composition_errors::Error::SwarmError(swarm_errors::Error::LogTypeEmpty(edge)));
        } else if t.label.log_type.len() > 1 {
            errors.push(composition_errors::Error::MoreThanOneEventTypeInCommand(edge)) // Come back here and implement splitting command into multiple 'artificial' ones emitting one event type if time instead of reporting it as an error.
        }
    }

    let initial = if let Some(idx) = nodes.get(&proto.initial) {
        errors.append(&mut all_nodes_reachable(&graph, *idx));
        Some(*idx)
    } else {
        // strictly speaking we have all_nodes_reachable errors here too...
        // if there is only an initial state no transitions whatsoever then thats ok? but gives an error here.
        errors.push(composition_errors::Error::SwarmError(
            swarm_errors::Error::InitialStateDisconnected,
        ));
        None
    };
    (graph, initial, errors)
}

// copied from swarm::swarm.rs
fn all_nodes_reachable(graph: &Graph, initial: NodeId) -> Vec<composition_errors::Error> {
    let _span = tracing::info_span!("all_nodes_reachable").entered();
    // Traversal order choice (Bfs vs Dfs vs DfsPostOrder) does not matter
    let visited = Dfs::new(&graph, initial)
        .iter(&graph)
        .collect::<BTreeSet<_>>();

    graph
        .node_indices()
        .filter(|node| !visited.contains(node))
        .map(|node| composition_errors::Error::SwarmError(swarm_errors::Error::StateUnreachable(node)))
        .collect()
}