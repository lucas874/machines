use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::{
    composition::composition_swarm::Error,
    types::{Command, EventType, MachineLabel, Role, SwarmLabel},
    Graph
};

use super::{NodeId, Subscriptions};

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum DataResult {
    OK { data: String },
    ERROR { errors: Vec<String> },
}

#[derive(Clone, Debug, Serialize)]
pub struct EventTypeInfo {
    pub event_type: EventType,
    pub role: Role,
    pub cmd: Command,
}

pub type RoleEventMap = BTreeMap<Role, Vec<EventTypeInfo>>;

pub type UnordEventPair = BTreeSet<EventType>;

pub fn unord_event_pair(a: EventType, b: EventType) -> UnordEventPair {
    BTreeSet::from([a, b])
}

#[derive(Debug)]
pub struct ProtoInfo {
    pub protocols: Vec<(Graph, Option<NodeId>, Vec<Error>)>,
    pub role_event_map: RoleEventMap,
    pub subscription: Subscriptions,
    pub concurrent_events: BTreeSet<UnordEventPair>, // consider to make a more specific type. unordered pair.
    pub branching_events: BTreeSet<EventType>,
    pub joining_events: BTreeSet<EventType>,
    pub interfaces: Vec<Role>,
}

impl ProtoInfo {
    pub fn new(
        protocols: Vec<(Graph, Option<NodeId>, Vec<Error>)>,
        role_event_map: RoleEventMap,
        subscription: Subscriptions,
        concurrent_events: BTreeSet<UnordEventPair>,
        branching_events: BTreeSet<EventType>,
        joining_events: BTreeSet<EventType>,
        interfaces: Vec<Role>,
    ) -> Self {
        Self {
            protocols,
            role_event_map,
            subscription,
            concurrent_events,
            branching_events,
            joining_events,
            interfaces,
        }
    }

    pub fn get_ith(&self, i: usize) -> Option<(Graph, Option<NodeId>, Vec<Error>)> {
        if i >= self.protocols.len() {
            None
        } else {
            Some(self.protocols[i].clone())
        }
    }
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
}

impl ProtoLabel for Graph {
    fn get_labels(&self) -> BTreeSet<(Command, EventType, Role)> {
        self.edge_references().map(|e| (e.weight().cmd.clone(), e.weight().get_event_type(), e.weight().role.clone())).collect()
    }

    fn get_roles(&self) -> BTreeSet<Role> {
        self.get_labels().into_iter().map(|(_, _, role)| role).collect()
    }
}

// Interface trait. Check if piece something is an interface w.r.t. a and b and get the interfacing events.
// Made so that notion of interface can change, hopefully without making too much changes to rest of code.
pub trait SwarmInterface {
    fn check_interface<T: ProtoLabel>(
        &self,
        a: &T,
        b: &T,
    ) -> Vec<Error>;
    fn interfacing_event_types<T: ProtoLabel>(
        &self,
        a: &T,
        b: &T,
    ) -> BTreeSet<EventType>;
}

impl SwarmInterface for Role {
    fn check_interface<T: ProtoLabel>(
        &self,
        a: &T,
        b: &T
    ) -> Vec<Error> {
        let role_intersection: BTreeSet<Role> =
            a.get_roles().intersection(&b.get_roles()).cloned().collect();//roles(g1).intersection(&roles(g2)).cloned().collect();

        // there should only be one role that appears in both protocols
        let mut errors =
            if role_intersection.contains(self) && role_intersection.iter().count() == 1 {
                vec![]
            } else {
                vec![Error::InvalidInterfaceRole(self.clone())]
            };

        let if_commands_1: BTreeSet<(Command, EventType, Role)> = a.get_labels()
            .into_iter()
            .filter(|(_, _, r)| *r == *self)
            .collect();
        let if_commands_2: BTreeSet<(Command, EventType, Role)> = b.get_labels()
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

    fn interfacing_event_types<T: ProtoLabel>(
        &self,
        a: &T,
        b: &T,
    ) -> BTreeSet<EventType> {
        if !self.check_interface(a, b).is_empty() {
            return BTreeSet::new();
        }

        a.get_labels().into_iter().map(|(_, e, _)| e).collect()
    }
}