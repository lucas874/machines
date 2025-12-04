use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use tsify::{declare, Tsify};

use machine_types::types::
    typescript_types::{EventType, State, MachineType, SwarmProtocolType};

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