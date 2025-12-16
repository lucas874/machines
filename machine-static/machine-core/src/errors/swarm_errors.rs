use itertools::Itertools;
use std::collections::BTreeSet;

use crate::types::{proto_graph::{EdgeId, NodeId}, typescript_types::{EventType, Role, StateName, SwarmLabel}};
use super::{Edge, INVALID_EDGE};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Error {
    InitialStateDisconnected,
    StateUnreachable(NodeId),
    LogTypeEmpty(EdgeId),
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
}

impl Error {
    fn to_string<N: StateName>(&self, graph: &petgraph::Graph<N, SwarmLabel>) -> String {
        match self {
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
        }
    }

    pub fn convert<N: StateName>(
        graph: &petgraph::Graph<N, SwarmLabel>,
    ) -> impl Fn(Error) -> String + '_ {
        |err| err.to_string(graph)
    }
}