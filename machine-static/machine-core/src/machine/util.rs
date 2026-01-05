use std::collections::BTreeMap;

use crate::types::{
    projection::{Graph, OptionGraph},
    proto_graph::NodeId,
    typescript_types::{MachineLabel, MachineType, State, Transition},
};
use petgraph::{graph::EdgeReference, visit::EdgeRef};

pub fn from_json(proto: MachineType) -> (OptionGraph, Option<NodeId>, Vec<String>) {
    let _span = tracing::debug_span!("from_json").entered();
    let mut errors = Vec::new();
    let mut machine = OptionGraph::new();
    let mut nodes = BTreeMap::new();
    for t in proto.transitions {
        tracing::debug!("adding {} --({:?})--> {}", t.source, t.label, t.target);
        let source = *nodes
            .entry(t.source.clone())
            .or_insert_with(|| machine.add_node(Some(t.source.clone())));
        let target = *nodes
            .entry(t.target.clone())
            .or_insert_with(|| machine.add_node(Some(t.target)));
        if let (MachineLabel::Execute { cmd, .. }, true) = (&t.label, source != target) {
            errors.push(format!(
                "command {cmd} is not a self-loop in state {}",
                t.source
            ));
        }
        machine.add_edge(source, target, t.label);
    }
    (machine, nodes.get(&proto.initial).copied(), errors)
}

pub fn to_json_machine(graph: Graph, initial: NodeId) -> MachineType {
    let _span = tracing::info_span!("to_json_machine").entered();
    let machine_label_mapper = |m: &Graph, eref: EdgeReference<'_, MachineLabel>| {
        let label = eref.weight().clone();
        let source = m[eref.source()].clone();
        let target = m[eref.target()].clone();
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

    MachineType {
        initial: graph[initial].clone(),
        transitions,
    }
}

pub fn from_option_to_machine(
    graph: petgraph::Graph<Option<State>, MachineLabel>,
    initial: NodeId,
) -> MachineType {
    let _span = tracing::info_span!("from_option_to_machine").entered();
    let machine_label_mapper =
        |m: &petgraph::Graph<Option<State>, MachineLabel>,
         eref: EdgeReference<'_, MachineLabel>| {
            let label = eref.weight().clone();
            let source = m[eref.source()].clone().unwrap_or(State::from(""));
            let target = m[eref.target()].clone().unwrap_or(State::from(""));
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

    MachineType {
        initial: graph[initial].clone().unwrap_or(State::from("")),
        transitions,
    }
}
