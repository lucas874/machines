use std::collections::{BTreeMap, BTreeSet};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{
    composition::composition_swarm::Error,
    types::{Command, EventType, MachineLabel, Role, SwarmLabel},
    Graph, Machine,
};

use super::{NodeId, SwarmProtocol};

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DataResult<T: Serialize> {
    OK { data: T },
    ERROR { errors: Vec<String> },
}

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
    pub interface: BTreeSet<EventType>,
}

impl ProtoStruct {
    pub fn new(
        graph: Graph,
        initial: Option<NodeId>,
        errors: Vec<Error>,
        interface: BTreeSet<EventType>,
    ) -> Self {
        Self {
            graph,
            initial,
            errors,
            interface,
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

#[derive(Debug, Clone)]
pub struct ProtoInfo {
    pub protocols: Vec<ProtoStruct>, // maybe weird to have an interface as if it was related to one protocol. but convenient. "a graph interfaces with rest on if"
    pub role_event_map: RoleEventMap,
    pub concurrent_events: BTreeSet<UnordEventPair>, // consider to make a more specific type. unordered pair.
    pub branching_events: Vec<BTreeSet<EventType>>,
    pub joining_events: BTreeSet<EventType>,
    pub immediately_pre: BTreeMap<EventType, BTreeSet<EventType>>,
    pub succeeding_events: BTreeMap<EventType, BTreeSet<EventType>>,
}

impl ProtoInfo {
    pub fn new(
        protocols: Vec<ProtoStruct>,
        role_event_map: RoleEventMap,
        concurrent_events: BTreeSet<UnordEventPair>,
        branching_events: Vec<BTreeSet<EventType>>,
        joining_events: BTreeSet<EventType>,
        immediately_pre: BTreeMap<EventType, BTreeSet<EventType>>,
        succeeding_events: BTreeMap<EventType, BTreeSet<EventType>>,
    ) -> Self {
        Self {
            protocols,
            role_event_map,
            concurrent_events,
            branching_events,
            joining_events,
            immediately_pre,
            succeeding_events,
        }
    }

    pub fn new_only_proto(protocols: Vec<ProtoStruct>) -> Self {
        Self {
            protocols,
            role_event_map: BTreeMap::new(),
            concurrent_events: BTreeSet::new(),
            branching_events: Vec::new(),
            joining_events: BTreeSet::new(),
            immediately_pre: BTreeMap::new(),
            succeeding_events: BTreeMap::new(),
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
        self.protocols.iter().all(|p| p.no_errors())
    }
}

pub fn get_branching_joining_proto_info(proto_info: &ProtoInfo) -> BTreeSet<EventType> {
    let get_pre_joins = |e: &EventType| -> BTreeSet<EventType> {
        let pre = proto_info.immediately_pre.get(e).cloned().unwrap_or_default();
        let product = pre.clone().into_iter().cartesian_product(&pre);
        product.filter(|(e1, e2)| *e1 != **e2 && proto_info.concurrent_events.contains(&unord_event_pair(e1.clone(), (*e2).clone())))
            .map(|(e1, e2)| [e1, e2.clone()])
            .flatten()
            .collect()
    };
    proto_info.branching_events.clone().into_iter()
        .flatten()
        .chain(proto_info.joining_events.clone().into_iter()
            .filter(|e| !get_pre_joins(e).is_empty()))
        .collect()
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CompositionComponent<T: SwarmInterface> {
    pub protocol: SwarmProtocol,
    pub interface: Option<T>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct InterfacingSwarms<T: SwarmInterface>(pub Vec<CompositionComponent<T>>);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Granularity {
    Fine,
    Medium,
    Coarse,
    TwoStep,
}

pub type SucceedingNonBranchingJoining = BTreeMap<EventType, BTreeSet<EventType>>;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionAndSucceedingMap {
    pub projection: Machine,
    pub branches: SucceedingNonBranchingJoining,
    pub special_event_types: BTreeSet<EventType>,
}

/* Used when combining machines and protocols */
pub trait EventLabel: Clone + Ord {
    fn get_event_type(&self) -> EventType;
}

impl EventLabel for SwarmLabel {
    fn get_event_type(&self) -> EventType {
        self.log_type[0].clone()
    }
}

impl EventLabel for MachineLabel {
    fn get_event_type(&self) -> EventType {
        match self {
            Self::Execute { log_type, .. } => log_type[0].clone(),
            Self::Input { event_type } => event_type.clone(),
        }
    }
}

/* Interface trait things */
pub trait ProtoLabel {
    fn get_labels(&self) -> BTreeSet<(Command, EventType, Role)>;
    fn get_roles(&self) -> BTreeSet<Role>;
    fn get_event_types(&self) -> BTreeSet<EventType>;
}

impl ProtoLabel for Graph {
    fn get_labels(&self) -> BTreeSet<(Command, EventType, Role)> {
        self.edge_references()
            .map(|e| {
                (
                    e.weight().cmd.clone(),
                    e.weight().get_event_type(),
                    e.weight().role.clone(),
                )
            })
            .collect()
    }

    fn get_roles(&self) -> BTreeSet<Role> {
        self.get_labels()
            .into_iter()
            .map(|(_, _, role)| role)
            .collect()
    }

    fn get_event_types(&self) -> BTreeSet<EventType> {
        self.get_labels()
            .into_iter()
            .map(|(_, event_type, _)| event_type)
            .collect()
    }
}

impl ProtoLabel for ProtoInfo {
    fn get_labels(&self) -> BTreeSet<(Command, EventType, Role)> {
        self.role_event_map
            .values()
            .flat_map(|role_info| {
                role_info
                    .iter()
                    .map(|sl| (sl.cmd.clone(), sl.get_event_type(), sl.role.clone()))
            })
            .collect()
    }

    fn get_roles(&self) -> BTreeSet<Role> {
        self.role_event_map.keys().cloned().collect()
    }

    fn get_event_types(&self) -> BTreeSet<EventType> {
        self.get_labels()
            .into_iter()
            .map(|(_, event_type, _)| event_type)
            .collect()
    }
}

// Interface trait. Check if piece something is an interface w.r.t. a and b and get the interfacing events.
// Made so that notion of interface can change, hopefully without making too much changes to rest of code.
pub trait SwarmInterface: Clone + Ord {
    fn check_interface<T: ProtoLabel>(&self, a: &T, b: &T) -> Vec<Error>;
    fn interfacing_event_types<T: ProtoLabel>(&self, a: &T, b: &T) -> BTreeSet<EventType>;
    fn interfacing_event_types_single<T: ProtoLabel>(&self, a: &T) -> BTreeSet<EventType>;
}

impl SwarmInterface for Role {
    fn check_interface<T: ProtoLabel>(&self, a: &T, b: &T) -> Vec<Error> {
        let role_intersection: BTreeSet<Role> = a
            .get_roles()
            .intersection(&b.get_roles())
            .cloned()
            .collect();

        // there should only be one role that appears in both protocols
        let mut errors =
            if role_intersection.contains(self) && role_intersection.iter().count() == 1 {
                vec![]
            } else {
                vec![Error::InvalidInterfaceRole(self.clone())]
            };

        let if_commands_1: BTreeSet<(Command, EventType, Role)> = a
            .get_labels()
            .into_iter()
            .filter(|(_, _, r)| *r == *self)
            .collect();
        let if_commands_2: BTreeSet<(Command, EventType, Role)> = b
            .get_labels()
            .into_iter()
            .filter(|(_, _, r)| *r == *self)
            .collect();

        // R<e> in proto1 iff. R<e> in proto2
        if if_commands_1 != if_commands_2 {
            let mut not_in_proto2: Vec<Error> = if_commands_1
                .difference(&if_commands_2)
                .map(|(_, e, _)| e.clone())
                .map(|event_type| Error::InterfaceEventNotInBothProtocols(event_type))
                .collect();
            let mut not_in_proto1: Vec<Error> = if_commands_2
                .difference(&if_commands_1)
                .map(|(_, e, _)| e.clone())
                .map(|event_type| Error::InterfaceEventNotInBothProtocols(event_type))
                .collect();
            errors.append(&mut not_in_proto1);
            errors.append(&mut not_in_proto2);
        }

        errors
    }

    fn interfacing_event_types<T: ProtoLabel>(&self, a: &T, b: &T) -> BTreeSet<EventType> {
        if !self.check_interface(a, b).is_empty() {
            return BTreeSet::new();
        }

        a.get_labels()
            .into_iter()
            .filter(|(_, _, r)| *self == *r)
            .map(|(_, e, _)| e)
            .collect()
    }

    // does not check anything. just returns any labels where role matches
    fn interfacing_event_types_single<T: ProtoLabel>(&self, a: &T) -> BTreeSet<EventType> {
        a.get_labels()
            .into_iter()
            .filter(|(_, _, r)| *self == *r)
            .map(|(_, e, _)| e)
            .collect()
    }
}
