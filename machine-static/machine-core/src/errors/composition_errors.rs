/* use itertools::Itertools;

use crate::types::{proto_graph::{EdgeId, NodeId}, typescript_types::{Command, EventType, Role, State, StateName, SwarmLabel}};
use super::Edge;

pub trait SummarizeError {
    fn to_string<N: StateName>(&self, graph: &petgraph::Graph<N, SwarmLabel>) -> String;
    fn convert<N: StateName>(graph: &petgraph::Graph<N, SwarmLabel>) -> impl Fn(Error) -> String + '_;
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Error {
    EventTypeOnDifferentLabels(EventType, Command, Role, Command, Role),
    CommandOnDifferentLabels(Command, EventType, Role, EventType, Role),
    MoreThanOneEventTypeInCommand(EdgeId),
    EventEmittedMultipleTimes(EventType, Vec<EdgeId>),
    CommandOnMultipleTransitions(Command, Vec<EdgeId>),
    InitialStateDisconnected,
    StateUnreachable(NodeId),
    LogTypeEmpty(EdgeId),
    InvalidArg, // weird error. not related to shape of protocol, but ok.
}

impl Error {
    fn to_string<N: StateName>(&self, graph: &petgraph::Graph<N, SwarmLabel>) -> String {
        match self {
            Error::EventTypeOnDifferentLabels(event_type, command1, role1, command2, role2) => {
                format!("Event type {event_type} appears as {command1}@{role1}<{event_type}> and as {command2}@{role2}<{event_type}>")
            }
            Error::CommandOnDifferentLabels(command, event_type1, role1, event_type2, role2) => {
                format!("Command {command} appears as {command}@{role1}<{event_type1}> and as {command}@{role2}<{event_type2}>")
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
            Error::InitialStateDisconnected => {
                format!("initial swarm protocol state has no transitions")
            }
            Error::StateUnreachable(node) => {
                format!(
                    "state {} is unreachable from initial state",
                    &graph[*node].state_name()
                )
            }
            Error::LogTypeEmpty(edge) => {
                format!("log type must not be empty {}", Edge(graph, *edge))
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

pub fn error_report_to_strings(error_report: ErrorReport) -> Vec<String> {
    error_report
        .errors()
        .into_iter()
        .flat_map(|(g, e)| e.into_iter().map(Error::convert(&g)).collect::<Vec<_>>())
        .collect()
} */