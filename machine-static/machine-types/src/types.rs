use petgraph::visit::GraphBase;

use crate::types::typescript_types::{State, SwarmLabel};

pub mod typescript_types;

pub type Graph = petgraph::Graph<State, SwarmLabel>;
pub type NodeId = <petgraph::Graph<(), ()> as GraphBase>::NodeId;
pub type EdgeId = <petgraph::Graph<(), ()> as GraphBase>::EdgeId;
