use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    composition::composition_swarm::Error,
    types::{Command, EventType, MachineLabel, Role, SwarmLabel},
    Graph,
};

use super::{NodeId, Subscriptions, SwarmProtocol};

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum DataResult {
    OK { data: String },
    ERROR { errors: Vec<String> },
}

/* #[derive(Clone, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventTypeInfo {
    pub cmd: Command,
    pub event_type: EventType,
    pub role: Role,
}

impl EventTypeInfo {
    pub fn new(cmd: Command, event_type: EventType, role: Role) -> Self {
        Self {
            cmd,
            role,
            event_type,
        }
    }
} */

/* #[derive(Clone, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct RoleInfo {
    pub swarm_labels: BTreeSet<SwarmLabel>,
    pub immediately_pre: BTreeSet<EventType>,
} */

/* impl RoleInfo {
    pub fn new(swarm_labels: BTreeSet<SwarmLabel>, immediately_pre: BTreeSet<EventType>) -> Self {
        Self {
            swarm_labels,
            immediately_pre,
        }
    }

    pub fn same_commands(&self, other: &RoleInfo) -> bool {
        self.swarm_labels == other.swarm_labels
    }

    pub fn combine(&self, other: &RoleInfo) -> RoleInfo {
        RoleInfo::new(
            self.swarm_labels.clone().into_iter().chain(other.swarm_labels.clone().into_iter()).collect(),
            self.immediately_pre.clone().into_iter().chain(other.immediately_pre.clone().into_iter()).collect(),
        )
    }
} */

pub type RoleEventMap = BTreeMap<Role, BTreeSet<SwarmLabel>>;//RoleInfo>;//BTreeSet<EventTypeInfo>>;

/* pub fn combine_role_event_maps(a: &RoleEventMap, b: &RoleEventMap) -> RoleEventMap {
    let all_keys: BTreeSet<Role> = a.keys().chain(a.keys()).cloned().collect();
    let mapper = |role: Role| -> (Role, RoleInfo) {
        let empty1 = RoleInfo::new(BTreeSet::new(), BTreeSet::new());
        let empty2 = RoleInfo::new(BTreeSet::new(), BTreeSet::new());
        let role_in_a = a.get(&role).unwrap_or(&empty1);
        let role_in_b = b.get(&role).unwrap_or(&empty2);
        (role, role_in_a.combine(role_in_b))
    };

    all_keys.into_iter().map(mapper).collect()
    //unimplemented!()
} */

/* pub fn combine_role_event_maps(a: &RoleEventMap, b: &RoleEventMap) -> RoleEventMap {
    let intersection: BTreeSet<Role> = a.keys().cloned().collect::<BTreeSet<_>>().union(&b.keys().cloned().collect::<BTreeSet<_>>()).cloned().collect();
    intersection
        .iter()
        .map(|role| (role.clone(), a[role].combine(&b[role])))
        .chain(a.keys().cloned().collect::<BTreeSet<_>>().difference(&intersection).map(|role| (role.clone(), a[role].clone())))
        .chain(b.keys().cloned().collect::<BTreeSet<_>>().difference(&intersection).map(|role| (role.clone(), b[role].clone())))
        .collect()
} */

pub type UnordEventPair = BTreeSet<EventType>;

pub fn unord_event_pair(a: EventType, b: EventType) -> UnordEventPair {
    BTreeSet::from([a, b])
}

#[derive(Debug, Clone)]
pub struct ProtoInfo {
    pub protocols: Vec<((Graph, Option<NodeId>, Vec<Error>), BTreeSet<EventType>)>, // maybe weird to have an interface as if it was related to one protocol. but convenient. "a graph interfaces with rest on if"
    pub role_event_map: RoleEventMap,
    pub subscription: Subscriptions,
    pub concurrent_events: BTreeSet<UnordEventPair>, // consider to make a more specific type. unordered pair.
    pub branching_events: BTreeSet<EventType>,
    pub joining_events: BTreeSet<EventType>,
    pub immediately_pre: BTreeMap<EventType, BTreeSet<EventType>>,
}

impl ProtoInfo {
    pub fn new(
        protocols: Vec<((Graph, Option<NodeId>, Vec<Error>), BTreeSet<EventType>)>,
        role_event_map: RoleEventMap,
        subscription: Subscriptions,
        concurrent_events: BTreeSet<UnordEventPair>,
        branching_events: BTreeSet<EventType>,
        joining_events: BTreeSet<EventType>,
        immediately_pre: BTreeMap<EventType, BTreeSet<EventType>>,
    ) -> Self {
        Self {
            protocols,
            role_event_map,
            subscription,
            concurrent_events,
            branching_events,
            joining_events,
            immediately_pre,
        }
    }

    pub fn new_only_proto(
        protocols: Vec<((Graph, Option<NodeId>, Vec<Error>), BTreeSet<EventType>)>
    ) -> Self {
        Self {
            protocols,
            role_event_map: BTreeMap::new(),
            subscription: BTreeMap::new(),
            concurrent_events: BTreeSet::new(),
            branching_events: BTreeSet::new(),
            joining_events: BTreeSet::new(),
            immediately_pre: BTreeMap::new(),
        }
    }

    pub fn get_ith_proto(&self, i: usize) -> Option<(Graph, Option<NodeId>, Vec<Error>)> {
        if i >= self.protocols.len() {
            None
        } else {
            Some(self.protocols[i].0.clone())
        }
    }

    pub fn no_errors(&self) -> bool {
        self.protocols.iter().all(|((_, _, e), _)| e.is_empty())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CompositionInput {
    pub protocol: SwarmProtocol,
    pub subscription: Subscriptions,
    pub interface: Option<Role>,
}

pub type CompositionInputVec = Vec<CompositionInput>;

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
