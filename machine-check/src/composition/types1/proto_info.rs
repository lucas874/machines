use std::collections::{BTreeMap, BTreeSet};

use itertools::Itertools;

use crate::{composition::types1::{error::{InterfaceError1, WellFormednessError}, util::{unord_event_pair1, UnordEventPair1}}, types::{EventType, Role, SwarmLabel}, Graph, NodeId};

pub type RoleEventMap = BTreeMap<Role, BTreeSet<SwarmLabel>>;

#[derive(Debug, Clone)]
pub struct ProtoStruct {
    pub graph: Graph,
    pub initial: Option<NodeId>,
    pub errors: Vec<WellFormednessError>,
    pub interface: BTreeSet<EventType>,
}

impl ProtoStruct {
    pub fn new(
        graph: Graph,
        initial: Option<NodeId>,
        errors: Vec<WellFormednessError>,
        interface: BTreeSet<EventType>,
    ) -> Self {
        Self {
            graph,
            initial,
            errors,
            interface,
        }
    }

    pub fn get_triple(&self) -> (Graph, Option<NodeId>, Vec<WellFormednessError>) {
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
    pub concurrent_events: BTreeSet<UnordEventPair1>, // consider to make a more specific type. unordered pair.
    pub branching_events: Vec<BTreeSet<EventType>>,
    pub joining_events: BTreeSet<EventType>,
    pub immediately_pre: BTreeMap<EventType, BTreeSet<EventType>>,
    pub succeeding_events: BTreeMap<EventType, BTreeSet<EventType>>,
    pub interface_errors: Vec<InterfaceError1>,
}

impl ProtoInfo {
    pub fn new(
        protocols: Vec<ProtoStruct>,
        role_event_map: RoleEventMap,
        concurrent_events: BTreeSet<UnordEventPair1>,
        branching_events: Vec<BTreeSet<EventType>>,
        joining_events: BTreeSet<EventType>,
        immediately_pre: BTreeMap<EventType, BTreeSet<EventType>>,
        succeeding_events: BTreeMap<EventType, BTreeSet<EventType>>,
        interface_errors: Vec<InterfaceError1>,
    ) -> Self {
        Self {
            protocols,
            role_event_map,
            concurrent_events,
            branching_events,
            joining_events,
            immediately_pre,
            succeeding_events,
            interface_errors,
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
        self.protocols.iter().all(|p| p.no_errors())
    }
}

// rename to get_updating_events?
pub fn get_branching_joining_proto_info(proto_info: &ProtoInfo) -> BTreeSet<EventType> {
    let get_pre_joins = |e: &EventType| -> BTreeSet<EventType> {
        let pre = proto_info
            .immediately_pre
            .get(e)
            .cloned()
            .unwrap_or_default();
        let product = pre.clone().into_iter().cartesian_product(&pre);
        product
            .filter(|(e1, e2)| {
                *e1 != **e2
                    && proto_info
                        .concurrent_events
                        .contains(&unord_event_pair1(e1.clone(), (*e2).clone()))
            })
            .map(|(e1, e2)| [e1, e2.clone()])
            .flatten()
            .collect()
    };
    proto_info
        .branching_events
        .clone()
        .into_iter()
        .flatten()
        .chain(
            proto_info
                .joining_events
                .clone()
                .into_iter()
                .filter(|e| !get_pre_joins(e).is_empty()),
        )
        .collect()
}