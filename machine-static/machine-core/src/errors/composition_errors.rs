use itertools::Itertools;

use crate::types::{proto_graph::{EdgeId, NodeId}, typescript_types::{Command, EventType, Role, State, StateName, SwarmLabel}};
use super::Edge;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Error {
    SwarmError(super::swarm_errors::Error),
    SwarmErrorString(String), // bit ugly but instead of making prepare graph public or making another from_json returning Error type in swarm.rs
    InvalidInterfaceRole(Role),
    InterfaceEventNotInBothProtocols(EventType),
    SpuriousInterface(Command, EventType, Role),
    EventTypeOnDifferentLabels(EventType, Command, Role, Command, Role),
    CommandOnDifferentLabels(Command, EventType, Role, EventType, Role),
    RoleNotSubscribedToBranch(Vec<EventType>, EdgeId, NodeId, Role),
    RoleNotSubscribedToJoin(Vec<EventType>, EdgeId, Role),
    LoopingError(EdgeId, Vec<Role>),
    MoreThanOneEventTypeInCommand(EdgeId),
    EventEmittedMultipleTimes(EventType, Vec<EdgeId>),
    CommandOnMultipleTransitions(Command, Vec<EdgeId>),
    StateCanNotReachTerminal(NodeId),
    InvalidArg, // weird error. not related to shape of protocol, but ok.
}

impl Error {
    fn to_string<N: StateName>(&self, graph: &petgraph::Graph<N, SwarmLabel>) -> String {
        match self {
            Error::SwarmError(e) => super::swarm_errors::Error::convert(&graph)(e.clone()),
            Error::SwarmErrorString(s) => s.clone(),
            Error::InvalidInterfaceRole(role) => {
                format!("role {role} can not be used as interface")
            }
            Error::InterfaceEventNotInBothProtocols(event_type) => {
                format!("event type {event_type} does not appear in both protocols")
            }
            Error::SpuriousInterface(command, event_type, role) => {
                format!("Role {role} is not used as an interface, but the command {command} or the event type {event_type} appear in both protocols")
            }
            Error::EventTypeOnDifferentLabels(event_type, command1, role1, command2, role2) => {
                format!("Event type {event_type} appears as {command1}@{role1}<{event_type}> and as {command2}@{role2}<{event_type}>")
            }
            Error::CommandOnDifferentLabels(command, event_type1, role1, event_type2, role2) => {
                format!("Command {command} appears as {command}@{role1}<{event_type1}> and as {command}@{role2}<{event_type2}>")
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
            Error::LoopingError(edge, roles) => {
                format!(
                    "transition {} is part of loop that can not reach a terminal state, but no looping event type in the loop is subscribed to by roles {} involved in the loop",
                    Edge(graph, *edge),
                    roles.join(", ")
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
}