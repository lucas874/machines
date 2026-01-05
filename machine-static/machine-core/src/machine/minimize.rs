use std::collections::{BTreeMap, BTreeSet};

use itertools::Itertools;
use petgraph::{
    Direction::{Incoming, Outgoing},
    visit::EdgeRef,
};

use crate::types::{
    projection::Graph,
    proto_graph::NodeId,
    typescript_types::{MachineLabel, State},
};

pub fn minimal_machine(graph: &Graph, i: NodeId) -> (Graph, NodeId) {
    let _span = tracing::info_span!("minimal_machine").entered();
    let partition = partition_refinement(graph);
    let mut minimal = Graph::new();
    let mut node_to_partition = BTreeMap::new();
    let mut partition_to_minimal_graph_node = BTreeMap::new();
    let mut edges = BTreeSet::new();
    let state_name = |nodes: &BTreeSet<NodeId>| -> State {
        let name = format!(
            "{{ {} }}",
            nodes.iter().map(|n| graph[*n].clone()).join(", ")
        );
        State::new(&name)
    };

    for n in graph.node_indices() {
        node_to_partition.insert(
            n,
            partition.iter().find(|block| block.contains(&n)).unwrap(),
        );
    }

    for block in &partition {
        partition_to_minimal_graph_node.insert(block, minimal.add_node(state_name(block)));
    }
    for node in graph.node_indices() {
        for edge in graph.edges_directed(node, Outgoing) {
            let source = partition_to_minimal_graph_node[node_to_partition[&node]];
            let target = partition_to_minimal_graph_node[node_to_partition[&edge.target()]];
            if !edges.contains(&(source, edge.weight().clone(), target)) {
                minimal.add_edge(source, target, edge.weight().clone());
                edges.insert((source, edge.weight().clone(), target));
            }
        }
    }
    let initial = partition_to_minimal_graph_node[node_to_partition[&i]];
    (minimal, initial)
}

fn partition_refinement(graph: &Graph) -> BTreeSet<BTreeSet<NodeId>> {
    let _span = tracing::info_span!("partition_refinement").entered();
    let mut partition_old = BTreeSet::new();
    let tmp: (BTreeSet<_>, BTreeSet<_>) = graph
        .node_indices()
        .partition(|n| graph.edges_directed(*n, Outgoing).count() == 0);
    let mut partition: BTreeSet<BTreeSet<NodeId>> = BTreeSet::from([tmp.0, tmp.1]);

    let pre_labels = |block: &BTreeSet<NodeId>| -> BTreeSet<MachineLabel> {
        block
            .iter()
            .flat_map(|n| {
                graph
                    .edges_directed(*n, Incoming)
                    .map(|e| e.weight().clone())
            })
            .collect()
    };

    while partition.len() != partition_old.len() {
        partition_old = partition.clone();
        for superblock in &partition_old {
            for label in pre_labels(superblock) {
                partition = refine_partition(graph, partition, superblock, &label);
            }
        }
    }

    partition
}

fn refine_partition(
    graph: &Graph,
    partition: BTreeSet<BTreeSet<NodeId>>,
    superblock: &BTreeSet<NodeId>,
    label: &MachineLabel,
) -> BTreeSet<BTreeSet<NodeId>> {
    partition
        .iter()
        .flat_map(|block| refine_block(graph, block, superblock, label))
        .collect()
}

fn refine_block(
    graph: &Graph,
    block: &BTreeSet<NodeId>,
    superblock: &BTreeSet<NodeId>,
    label: &MachineLabel,
) -> BTreeSet<BTreeSet<NodeId>> {
    let predicate = |node: &NodeId| -> bool {
        graph
            .edges_directed(*node, Outgoing)
            .any(|e| *e.weight() == *label && superblock.contains(&e.target()))
    };

    let tmp: (BTreeSet<_>, BTreeSet<_>) = block.iter().partition(|n| predicate(n));

    BTreeSet::from([tmp.0, tmp.1])
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect()
}

// Nfa to dfa using subset construction. Hopcroft, Motwani and Ullman section 2.3.5.
// Not strictly related to minimizing. But here anyway. Transforms a projection.
pub fn nfa_to_dfa(nfa: Graph, i: NodeId) -> (Graph, NodeId) {
    let _span = tracing::info_span!("nfa_to_dfa").entered();
    let mut dfa = Graph::new();
    // maps vectors of NodeIds from the nfa to a NodeId in the new dfa
    let mut dfa_nodes: BTreeMap<BTreeSet<NodeId>, NodeId> = BTreeMap::new();

    // push to and pop from in loop until empty and NFA has been turned into a dfa
    let mut stack: Vec<BTreeSet<NodeId>> = Vec::new();

    // [0, 1, 2] becomes Some(State("{0, 1, 2}"))
    let state_name = |nodes: &BTreeSet<NodeId>| -> State {
        let name = format!("{{ {} }}", nodes.iter().map(|n| nfa[*n].clone()).join(", "));
        State::new(&name)
    };

    // get all outgoing edges of the sources. turn into a map from machine labels to vectors of target states.
    let outgoing_map = |srcs: &BTreeSet<NodeId>| -> BTreeMap<MachineLabel, BTreeSet<NodeId>> {
        srcs.iter()
            .flat_map(|src| {
                nfa.edges_directed(*src, Outgoing)
                    .map(|e| (e.weight().clone(), e.target()))
            })
            .collect::<BTreeSet<(MachineLabel, NodeId)>>()
            .into_iter()
            .fold(BTreeMap::new(), |mut m, (edge_label, target)| {
                m.entry(edge_label)
                    .and_modify(|v: &mut BTreeSet<NodeId>| {
                        v.insert(target);
                    })
                    .or_insert_with(|| BTreeSet::from([target]));
                m
            })
    };

    // add initial state to dfa
    dfa_nodes.insert(
        BTreeSet::from([i]),
        dfa.add_node(state_name(&BTreeSet::from([i]))),
    );
    // add initial state to stack
    stack.push(BTreeSet::from([i]));

    while let Some(states) = stack.pop() {
        let map = outgoing_map(&states);
        for edge in map.keys() {
            if !dfa_nodes.contains_key(&map[edge]) {
                stack.push(map[edge].clone());
            }
            let target: NodeId = *dfa_nodes
                .entry(map[edge].clone())
                .or_insert_with(|| dfa.add_node(state_name(&map[edge])));
            let src: NodeId = *dfa_nodes.get(&states).unwrap();
            dfa.add_edge(src, target, edge.clone());
        }
    }

    (dfa, dfa_nodes[&BTreeSet::from([i])])
}
