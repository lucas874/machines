use std::collections::BTreeSet;
use crate::types::{proto_graph::NodeId, typescript_types::{EventType, MachineLabel, State, SwarmLabel}};

pub type Graph = petgraph::Graph<State, MachineLabel>;
pub type OptionGraph = petgraph::Graph<Option<State>, MachineLabel>;

// Vec of triples of the form:
//      (protocol_graph, initial_node, interfacing event types with vec[i-1])
// Protocols linked together in a 'chain' by interfacing event types
pub type ChainedProtos = Vec<(
    petgraph::Graph<State, SwarmLabel>,
    NodeId,
    BTreeSet<EventType>,
)>;

// Same as type above, but for projections
pub type ChainedProjections = Vec<(Graph, NodeId, BTreeSet<EventType>)>;
