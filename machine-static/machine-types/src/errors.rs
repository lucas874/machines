use std::fmt;

use crate::types::{EdgeId, typescript_types::{StateName, SwarmLabel}};

pub mod swarm_errors;
pub mod composition_errors;


const INVALID_EDGE: &str = "[invalid EdgeId]";

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