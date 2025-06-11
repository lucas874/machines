use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::types::{EventType, MachineLabel, State, StateName, SwarmLabel};

// Serializable result type returned by functions in composition.rs
#[derive(Tsify, Serialize, Deserialize)]
#[serde(tag = "type")]
#[tsify(into_wasm_abi)]
pub enum DataResult1<T> {
    OK { data: T },
    ERROR { errors: Vec<String> },
}

// An unordered pair of event types
pub type UnordEventPair1 = BTreeSet<EventType>;
pub fn unord_event_pair1(a: EventType, b: EventType) -> UnordEventPair1 {
    BTreeSet::from([a, b])
}


// Trait implemented by SwarmLabel and MachineLabel so that we can use the same function for composition
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

impl From<String> for State {
    fn from(value: String) -> State {
        State::new(&value)
    }
}

pub fn gen_state_name<N: StateName + From<String>>(n1: &N, n2: &N) -> N {
    let name = format!("{} || {}", n1.state_name(), n2.state_name());
    N::from(name)
}
