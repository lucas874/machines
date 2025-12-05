use serde::{Deserialize, Serialize};
use tsify::{Tsify};

use machine_types::types::
    typescript_types::{SwarmProtocolType};

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct CompositionComponent<T> {
    pub protocol: SwarmProtocolType,
    pub interface: Option<T>,
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct InterfacingSwarms<T>(pub Vec<CompositionComponent<T>>);