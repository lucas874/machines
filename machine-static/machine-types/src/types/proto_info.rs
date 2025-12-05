use std::collections::{BTreeMap, BTreeSet};

use crate::errors::composition_errors::Error;
use crate::types::typescript_types::{Command, EventLabel};
use crate::types::{
    typescript_types::{EventType, Role, SwarmLabel},
    Graph, NodeId,
};

pub type RoleEventMap = BTreeMap<Role, BTreeSet<SwarmLabel>>;

pub type UnordEventPair = BTreeSet<EventType>;

pub fn unord_event_pair(a: EventType, b: EventType) -> UnordEventPair {
    BTreeSet::from([a, b])
}

#[derive(Debug, Clone)]
pub struct ProtoStruct {
    pub graph: Graph,
    pub initial: Option<NodeId>,
    pub errors: Vec<Error>,
    pub roles: BTreeSet<Role>,
}

impl ProtoStruct {
    pub fn new(
        graph: Graph,
        initial: Option<NodeId>,
        errors: Vec<Error>,
        roles: BTreeSet<Role>,
    ) -> Self {
        Self {
            graph,
            initial,
            errors,
            roles,
        }
    }

    pub fn get_triple(&self) -> (Graph, Option<NodeId>, Vec<Error>) {
        (
            self.graph.clone(),
            self.initial.clone(),
            self.errors.clone(),
        )
    }

    pub fn no_errors(&self) -> bool {
        self.errors.is_empty()
    }
}

// I do not think this is the way to go. Set of event types suffices?
#[derive(Debug, Clone)]
pub struct InterfaceStruct {
    pub interfacing_roles: BTreeSet<Role>,
    pub interfacing_event_types: BTreeSet<EventType>,
}

impl InterfaceStruct {
    // https://doc.rust-lang.org/src/alloc/vec/mod.rs.html#434
    #[inline]
    pub const fn new() -> Self {
        InterfaceStruct {
            interfacing_roles: BTreeSet::new(),
            interfacing_event_types: BTreeSet::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProtoInfo {
    pub protocols: Vec<ProtoStruct>,
    pub role_event_map: RoleEventMap,
    pub concurrent_events: BTreeSet<UnordEventPair>, // Consider to make a more specific type. unordered pair.
    pub branching_events: Vec<BTreeSet<EventType>>,
    pub joining_events: BTreeMap<EventType, BTreeSet<EventType>>,
    pub immediately_pre: BTreeMap<EventType, BTreeSet<EventType>>,
    pub succeeding_events: BTreeMap<EventType, BTreeSet<EventType>>,
    pub interfacing_events: BTreeSet<EventType>,
    pub infinitely_looping_events: BTreeSet<EventType>, // Event types that do not lead to a terminal state.
    pub interface_errors: Vec<Error>,
}

impl ProtoInfo {
    pub fn new(
        protocols: Vec<ProtoStruct>,
        role_event_map: RoleEventMap,
        concurrent_events: BTreeSet<UnordEventPair>,
        branching_events: Vec<BTreeSet<EventType>>,
        joining_events: BTreeMap<EventType, BTreeSet<EventType>>,
        immediately_pre: BTreeMap<EventType, BTreeSet<EventType>>,
        succeeding_events: BTreeMap<EventType, BTreeSet<EventType>>,
        interfacing_events: BTreeSet<EventType>,
        infinitely_looping_events: BTreeSet<EventType>,
        interface_errors: Vec<Error>,
    ) -> Self {
        Self {
            protocols,
            role_event_map,
            concurrent_events,
            branching_events,
            joining_events,
            immediately_pre,
            succeeding_events,
            interfacing_events,
            infinitely_looping_events,
            interface_errors,
        }
    }

    pub fn new_only_proto(protocols: Vec<ProtoStruct>) -> Self {
        Self {
            protocols,
            role_event_map: BTreeMap::new(),
            concurrent_events: BTreeSet::new(),
            branching_events: Vec::new(),
            joining_events: BTreeMap::new(),
            immediately_pre: BTreeMap::new(),
            succeeding_events: BTreeMap::new(),
            interfacing_events: BTreeSet::new(),
            infinitely_looping_events: BTreeSet::new(),
            interface_errors: Vec::new(),
        }
    }

    pub fn get_ith_proto(&self, i: usize) -> Option<ProtoStruct> {
        if i >= self.protocols.len() {
            None
        } else {
            Some(self.protocols[i].clone())
        }
    }

    pub fn no_errors(&self) -> bool {
        self.protocols.iter().all(|p| p.no_errors()) && self.interface_errors.is_empty()
    }

    // Return all values from a ProtoInfo.role_event_map field as a set of triples:
    // (Command, EventType, Role)
    fn get_labels(&self) -> BTreeSet<(Command, EventType, Role)> {
        self
            .role_event_map
            .values()
            .flat_map(|role_info| {
                role_info
                    .iter()
                    .map(|sl| (sl.cmd.clone(), sl.get_event_type(), sl.role.clone()))
            })
            .collect()
    }

    // If we accumulate errors this map should really map to a set of (Command, Role)...
    pub fn event_type_map(&self) -> BTreeMap<EventType, (Command, Role)> {
        self.get_labels()
            .into_iter()
            .map(|(c, t, r)| (t, (c, r)))
            .collect()
    }

    // If we accumulate errors this map should really map to a set of (EvenType, Role)...
    pub fn command_map(&self) -> BTreeMap<Command, (EventType, Role)> {
        self.get_labels()
            .into_iter()
            .map(|(c, t, r)| (c, (t, r)))
            .collect()
    }
}

pub fn get_branching_joining_proto_info(proto_info: &ProtoInfo) -> BTreeSet<EventType> {
    proto_info
        .branching_events
        .clone()
        .into_iter()
        .flatten()
        .chain(
            proto_info
                .joining_events
                .keys()
                .cloned()
                .collect::<BTreeSet<EventType>>(),
        )
        .collect()
}