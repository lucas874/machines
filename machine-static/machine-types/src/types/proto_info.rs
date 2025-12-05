use std::collections::{BTreeMap, BTreeSet};

use itertools::Itertools;

use crate::errors::composition_errors::Error;
use crate::types::proto_label::ProtoLabel;
use crate::types::typescript_types::{Command, EventLabel};
use crate::types::{
    typescript_types::{EventType, Role, SwarmLabel},
    Graph, NodeId,
};
use crate::{util, composability_check};

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

// Overapproximate concurrent events. 
// Anything from different protocols that are not interfacing events is considered concurrent.
// Pre: interface has been checked.
fn get_concurrent_events(
    proto_info1: &ProtoInfo,
    proto_info2: &ProtoInfo,
    interfacing_event_types: &BTreeSet<EventType>,
) -> BTreeSet<UnordEventPair> {
    let _span = tracing::info_span!("get_concurrent_events").entered();
    let concurrent_events_union: BTreeSet<UnordEventPair> = proto_info1
        .concurrent_events
        .union(&proto_info2.concurrent_events)
        .cloned()
        .collect();
    let events_proto1: BTreeSet<EventType> = proto_info1
        .get_event_types()
        .difference(interfacing_event_types)
        .cloned()
        .collect();
    let events_proto2: BTreeSet<EventType> = proto_info2
        .get_event_types()
        .difference(interfacing_event_types)
        .cloned()
        .collect();
    let cartesian_product = events_proto1
        .into_iter()
        .cartesian_product(&events_proto2)
        .map(|(a, b)| unord_event_pair(a, b.clone()))
        .collect();

    concurrent_events_union
        .union(&cartesian_product)
        .cloned()
        .collect()
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

// Set of interfacing roles between two protocols
#[inline]
fn get_interfacing_roles(proto_info1: &ProtoInfo, proto_info2: &ProtoInfo) -> BTreeSet<Role> {
    proto_info1
        .protocols
        .iter()
        .flat_map(|protostruct| protostruct.roles.clone())
        .collect::<BTreeSet<Role>>()
        .intersection(
            &proto_info2
                .protocols
                .iter()
                .flat_map(|protostruct| protostruct.roles.clone())
                .collect::<BTreeSet<Role>>(),
        )
        .cloned()
        .collect()
}

// The interfacing roles are those roles that appear in proto_info1 and in proto_info2
// The interfacing event types are those emitted by the interfacing role in either proto_info1 or proto_info2.
// Assumes that proto_info1 and proto_info2 interface correctly.
#[inline]
fn get_interfacing_event_types(
    proto_info1: &ProtoInfo,
    proto_info2: &ProtoInfo,
) -> BTreeSet<EventType> {
    get_interfacing_roles(proto_info1, proto_info2)
        .iter()
        .flat_map(|r| {
            proto_info1
                .role_event_map
                .get(r)
                .unwrap()
                .union(&proto_info2.role_event_map.get(r).unwrap())
        })
        .map(|swarm_label| swarm_label.get_event_type())
        .collect()
}

// Construct map from joining event types to concurrent events preceding joining event types.
#[inline]
fn joining_event_types_map(proto_info: &ProtoInfo) -> BTreeMap<EventType, BTreeSet<EventType>> {
    let pre_joins = |e: &EventType| -> BTreeSet<EventType> {
        let pre = proto_info
            .immediately_pre
            .get(e)
            .cloned()
            .unwrap_or_default();
        let product = pre.clone().into_iter().cartesian_product(&pre);
        product
            .filter(|(e1, e2)| {
                *e1 != **e2 // necessary? Not the case if in set of concurrent?
                    && proto_info
                        .concurrent_events
                        .contains(&unord_event_pair(e1.clone(), (*e2).clone()))
            })
            .map(|(e1, e2)| [e1, e2.clone()])
            .flatten()
            .collect()
    };
    // Get those interfacing event types with immediately preceding conucurrent event types and turn it into a map
    proto_info
        .interfacing_events
        .iter()
        .map(|e| (e.clone(), pre_joins(e)))
        .filter(|(_, pre)| !pre.is_empty())
        .collect()
}

pub fn flatten_joining_map(
    joining_event_types: &BTreeMap<EventType, BTreeSet<EventType>>,
) -> BTreeSet<EventType> {
    joining_event_types
        .iter()
        .flat_map(|(join, pre)| pre.clone().into_iter().chain([join.clone()]))
        .collect()
}

// Combine fields of two proto infos.
// Do not compute transitive closure of happens after and do not compute joining event types.
fn combine_two_proto_infos(proto_info1: ProtoInfo, proto_info2: ProtoInfo) -> ProtoInfo {
    let _span = tracing::info_span!("combine_proto_infos").entered();
    let interface_errors = composability_check::check_interface(&proto_info1, &proto_info2);
    let interfacing_event_types = get_interfacing_event_types(&proto_info1, &proto_info2);
    let protocols = vec![proto_info1.protocols.clone(), proto_info2.protocols.clone()].concat();
    let role_event_map = util::combine_maps(
        proto_info1.role_event_map.clone(),
        proto_info2.role_event_map.clone(),
        None,
    );
    // get concurrent event types based on current set of interfacing event types.
    let concurrent_events =
        get_concurrent_events(&proto_info1, &proto_info2, &interfacing_event_types);
    let branching_events: Vec<BTreeSet<EventType>> = proto_info1
        .branching_events
        .into_iter()
        .chain(proto_info2.branching_events.into_iter())
        .collect();
    let immediately_pre = util::combine_maps(
        proto_info1.immediately_pre.clone(),
        proto_info2.immediately_pre.clone(),
        None,
    );
    let happens_after = util::combine_maps(
        proto_info1.succeeding_events,
        proto_info2.succeeding_events,
        None,
    );

    let interfacing_event_types = [
        proto_info1.interfacing_events,
        proto_info2.interfacing_events,
        interfacing_event_types,
    ]
    .into_iter()
    .flatten()
    .collect();

    let infinitely_looping_events = proto_info1
        .infinitely_looping_events
        .into_iter()
        .chain(proto_info2.infinitely_looping_events.into_iter())
        .collect();

    ProtoInfo::new(
        protocols,
        role_event_map,
        concurrent_events,
        branching_events,
        BTreeMap::new(),
        immediately_pre,
        happens_after,
        interfacing_event_types,
        infinitely_looping_events,
        [
            proto_info1.interface_errors,
            proto_info2.interface_errors,
            interface_errors,
        ]
        .concat(),
    )
}

pub fn combine_proto_infos(protos: Vec<ProtoInfo>) -> ProtoInfo {
    let _span = tracing::info_span!("combine_proto_infos_fold").entered();
    if protos.is_empty() {
        return ProtoInfo::new_only_proto(vec![]);
    }

    let mut combined = protos[1..]
        .to_vec()
        .into_iter()
        .fold(protos[0].clone(), |acc, p| combine_two_proto_infos(acc, p));

    combined.joining_events = joining_event_types_map(&combined);
    combined
}