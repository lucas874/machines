use itertools::Itertools;
use std::{collections::BTreeSet, fmt};

use crate::types::{EdgeId, NodeId, typescript_types::{EventType, Role, StateName, SwarmLabel}};

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

const INVALID_EDGE: &str = "[invalid EdgeId]";

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

/// helper for printing a transition
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

// use pub struct Edge... for this to work. Right now just copied whole edge and fmt implemenation to that ONE place outside of here that needed it it.
// -- tests in machine-check
/* impl<'a, N: StateName> Edge<'a, N> {
    pub fn new(graph: &'a petgraph::Graph<N, SwarmLabel>, edge_id: EdgeId) -> Self {
        Self(graph, edge_id)
    }
} */