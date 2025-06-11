use crate::{types::{MachineLabel, State, StateName, Transition}, MachineType, NodeId};
use petgraph::{
    graph::EdgeReference,
    visit::EdgeRef,
};

// types more or less copied from machine.rs.
pub type MachineGraph = petgraph::Graph<State, MachineLabel>;
pub type OptionGraph = petgraph::Graph<Option<State>, MachineLabel>;

pub(in crate::composition) fn to_option_machine(graph: &MachineGraph) -> OptionGraph {
    graph.map(|_, n| Some(n.state_name().clone()), |_, x| x.clone())
}

pub(in crate::composition) fn from_option_graph_to_graph(graph: &OptionGraph) -> MachineGraph {
    graph.map(
        |_, n| n.clone().unwrap_or_else(|| State::new("")),
        |_, x| x.clone(),
    )
}

//from_adaption_to_machine
pub fn to_json_machine(graph: MachineGraph, initial: NodeId) -> MachineType {
    let _span = tracing::info_span!("to_json_machine").entered();
    let machine_label_mapper = |m: &MachineGraph, eref: EdgeReference<'_, MachineLabel>| {
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