use itertools::Itertools;
use std::collections::BTreeSet;

use crate::types::{proto_graph::{EdgeId, Graph, NodeId}, typescript_types::{Command, EventType, Role, StateName, SwarmLabel}};
use super::{Edge, INVALID_EDGE};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Error {
    ActiveRoleNotSubscribed(EdgeId),
    LaterActiveRoleNotSubscribed(EdgeId, Role),
    LaterInvolvedRoleMoreSubscribed {
        edge: EdgeId,
        later: Role,
        active: Role,
        events: BTreeSet<EventType>,
    },
    LaterInvolvedNotGuarded(EdgeId, Role),
    NonDeterministicGuard(EdgeId),
    NonDeterministicCommand(EdgeId),
    GuardNotInvariant(EventType),
    RoleNotSubscribedToBranch(Vec<EventType>, EdgeId, NodeId, Role),
    RoleNotSubscribedToJoin(Vec<EventType>, EdgeId, Role),
    LoopingError(EdgeId, Vec<Role>),
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
            Error::ActiveRoleNotSubscribed(edge) => {
                format!("active role does not subscribe to any of its emitted event types in transition {}", Edge(graph, *edge))
            }
            Error::LaterActiveRoleNotSubscribed(edge, role) => {
                format!(
                    "subsequently active role {role} does not subscribe to events in transition {}",
                    Edge(graph, *edge)
                )
            }
            Error::LaterInvolvedRoleMoreSubscribed {
                edge,
                later,
                active,
                events,
            } => format!(
                "subsequently involved role {later} subscribes to more events \
                 than active role {active} in transition {}, namely ({})",
                Edge(graph, *edge),
                events.iter().join(", ")
            ),
            Error::LaterInvolvedNotGuarded(edge, role) => format!(
                "subsequently involved role {role} does not subscribe to guard \
                 in transition {}",
                Edge(graph, *edge)
            ),
            Error::NonDeterministicGuard(edge) => {
                let Some((state, _)) = graph.edge_endpoints(*edge) else {
                    return format!("non-deterministic event guard {}", INVALID_EDGE);
                };
                let state = graph[state].state_name();
                let guard = &graph[*edge].log_type[0];
                format!("non-deterministic event guard type {guard} in state {state}")
            }
            Error::NonDeterministicCommand(edge) => {
                let Some((state, _)) = graph.edge_endpoints(*edge) else {
                    return format!("non-deterministic command {}", INVALID_EDGE);
                };
                let state = graph[state].state_name();
                let command = &graph[*edge].cmd;
                let role = &graph[*edge].role;
                format!("non-deterministic command {command} for role {role} in state {state}")
            }
            Error::GuardNotInvariant(ev) => {
                format!("guard event type {ev} appears in transitions from multiple states")
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
pub struct ErrorReport(pub Vec<(Graph, Vec<Error>)>);

impl ErrorReport {
    pub fn is_empty(&self) -> bool {
        self.0.iter().all(|(_, es)| es.is_empty())
    }

    pub fn errors(&self) -> Vec<(Graph, Vec<Error>)> {
        self.0.clone()
    }

    pub fn to_strings(&self) -> Vec<String> {
        self
            .errors()
            .into_iter()
            .flat_map(|(g, e)| e.into_iter().map(Error::convert(&g)).collect::<Vec<_>>())
            .collect()
    }
}