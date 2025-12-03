use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use tsify::{declare, Tsify};

use crate::{
    composition::composition_swarm::Error,
    types::{Command, EventType, MachineLabel, Role, State, SwarmLabel},
    Graph, MachineType,
};

use super::{NodeId, SwarmProtocolType};

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

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct CompositionComponent<T> {
    pub protocol: SwarmProtocolType,
    pub interface: Option<T>,
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct InterfacingSwarms<T>(pub Vec<CompositionComponent<T>>);

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct InterfacingProtocols(pub Vec<SwarmProtocolType>);

#[derive(Tsify, Serialize, Deserialize, Debug, Clone)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub enum Granularity {
    Fine,
    Medium,
    Coarse,
    TwoStep,
}

#[declare]
pub type BranchMap = BTreeMap<EventType, Vec<EventType>>;
#[declare]
pub type SpecialEventTypes = BTreeSet<EventType>;
#[declare]
pub type ProjToMachineStates = BTreeMap<State, Vec<State>>;
/* #[derive(Serialize, Deserialize)]
pub struct EventSet(pub BTreeSet<EventType>);

impl Tsify for EventSet {
    const DECL: &'static str = "Set<string>";
} */

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct ProjectionInfo {
    pub projection: MachineType,
    pub branches: BranchMap,
    pub special_event_types: SpecialEventTypes,
    pub proj_to_machine_states: ProjToMachineStates,
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
        println!("{:?}", role_intersection);
        // there should only be one role that appears in both protocols
        let mut errors =
            if role_intersection.contains(self) && role_intersection.iter().count() == 1 {
                vec![]
            } else {
                vec![Error::InvalidInterfaceRole(self.clone())]
            };

        let triples_a: BTreeSet<(Command, EventType, Role)> = a.get_labels().into_iter().collect();
        let triples_b: BTreeSet<(Command, EventType, Role)> = b.get_labels().into_iter().collect();
        let event_types_a: BTreeSet<EventType> =
            triples_a.iter().map(|(_, et, _)| et).cloned().collect();
        let commands_a: BTreeSet<Command> = triples_a.iter().map(|(c, _, _)| c).cloned().collect();
        let event_types_b: BTreeSet<EventType> =
            triples_b.iter().map(|(_, et, _)| et).cloned().collect();
        let commands_b: BTreeSet<Command> = triples_b.iter().map(|(c, _, _)| c).cloned().collect();

        let matcher = |triple: &(Command, EventType, Role),
                       reference_triples: &BTreeSet<(Command, EventType, Role)>,
                       reference_event_types: &BTreeSet<EventType>,
                       reference_commands: &BTreeSet<Command>| match triple {
            (_, et, r) if *r == *self && !reference_triples.contains(triple) => {
                Some(Error::InterfaceEventNotInBothProtocols(et.clone()))
            }
            (c, et, r)
                if *r != *self
                    && (reference_event_types.contains(et) || reference_commands.contains(c)) =>
            {
                Some(Error::SpuriousInterface(c.clone(), et.clone(), r.clone()))
            }
            _ => None,
        };

        errors.append(
            &mut triples_a
                .iter()
                .map(|triple| matcher(triple, &triples_b, &event_types_b, &commands_b))
                .filter_map(|e| e)
                .collect(),
        );
        errors.append(
            &mut triples_b
                .iter()
                .map(|triple| matcher(triple, &triples_a, &event_types_a, &commands_a))
                .filter_map(|e| e)
                .collect(),
        );

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