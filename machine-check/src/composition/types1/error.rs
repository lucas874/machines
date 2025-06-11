use crate::{types::{Command, EventType, Role, StateName, SwarmLabel}, EdgeId, NodeId};
use std::fmt;
use itertools::Itertools;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum WellFormednessError {
    SwarmError(crate::swarm::Error),
    InvalidInterfaceRole(Role),
    InterfaceEventNotInBothProtocols(EventType),
    SpuriousInterface(Command, EventType, Role),
    RoleNotSubscribedToBranch(Vec<EventType>, EdgeId, NodeId, Role),
    RoleNotSubscribedToJoin(Vec<EventType>, EdgeId, Role),
    MoreThanOneEventTypeInCommand(EdgeId),
    EventEmittedMultipleTimes(EventType, Vec<EdgeId>),
    CommandOnMultipleTransitions(Command, Vec<EdgeId>),
    StateCanNotReachTerminal(NodeId),
    InvalidArg, // weird error. not related to shape of protocol, but ok.
}

impl WellFormednessError {
    fn to_string<N: StateName>(&self, graph: &petgraph::Graph<N, SwarmLabel>) -> String {
        match self {
            WellFormednessError::SwarmError(e) => crate::swarm::Error::convert(&graph)(e.clone()),
            WellFormednessError::InvalidInterfaceRole(role) => {
                format!("role {role} can not be used as interface")
            }
            WellFormednessError::InterfaceEventNotInBothProtocols(event_type) => {
                format!("event type {event_type} does not appear in both protocols")
            }
            WellFormednessError::SpuriousInterface(command, event_type, role) => {
                format!("Role {role} is not used as an interface, but the command {command} or the event type {event_type} appear in both protocols")
            }
            WellFormednessError::RoleNotSubscribedToBranch(event_types, edge, node, role) => {
                let events = event_types.join(", ");
                format!(
                    "role {role} does not subscribe to event types {events} in branching transitions at state {}, but is involved after transition {}",
                    &graph[*node].state_name(),
                    Edge(graph, *edge)
                )
            }
            WellFormednessError::RoleNotSubscribedToJoin(preceding_events, edge, role) => {
                let events = preceding_events.join(", ");
                format!(
                    "role {role} does not subscribe to event types {events} leading to or in joining event in transition {}",
                    Edge(graph, *edge),
                )
            }
            WellFormednessError::MoreThanOneEventTypeInCommand(edge) => {
                format!(
                    "transition {} emits more than one event type",
                    Edge(graph, *edge)
                )
            }
            WellFormednessError::EventEmittedMultipleTimes(event_type, edges) => {
                let edges_pretty = edges.iter().map(|edge| Edge(graph, *edge)).join(", ");
                format!(
                    "event type {event_type} emitted in more than one transition: {}",
                    edges_pretty
                )
            }
            WellFormednessError::CommandOnMultipleTransitions(command, edges) => {
                let edges_pretty = edges.iter().map(|edge| Edge(graph, *edge)).join(", ");
                format!(
                    "command {command} enabled in more than one transition: {}",
                    edges_pretty
                )
            }
            WellFormednessError::StateCanNotReachTerminal(node) => {
                format!(
                    "state {} can not reach terminal node",
                    &graph[*node].state_name()
                )
            }
            WellFormednessError::InvalidArg => {
                format!("invalid argument",)
            }
        }
    }

    pub fn convert<N: StateName>(
        graph: &petgraph::Graph<N, SwarmLabel>,
    ) -> impl Fn(WellFormednessError) -> String + '_ {
        |err| err.to_string(graph)
    }
}

const INVALID_EDGE: &str = "[invalid EdgeId]";

/// Copied from swarm.rs helper for printing a transition
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

// Errors related to interfaces
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum InterfaceError1 {
    InvalidInterfaceRole(Role),
    InterfaceEventNotInBothProtocols(EventType),
    SpuriousInterface(Command, EventType, Role),
}

impl InterfaceError1 {
    pub fn to_string(&self) -> String {
        match self {
            InterfaceError1::InvalidInterfaceRole(role) => {
                format!("role {role} can not be used as interface")
            }
            InterfaceError1::InterfaceEventNotInBothProtocols(event_type) => {
                format!("event type {event_type} does not appear in both protocols")
            }
            InterfaceError1::SpuriousInterface(command, event_type, role) => {
                format!("Role {role} is not used as an interface, but the command {command} or the event type {event_type} appear in both protocols")
            }
        }
    }
}