use intern_arc::{global::hash_interner, InternedHash};
use serde::{Deserialize, Serialize};
use std::{borrow::Borrow, fmt, ops::Deref};
use tsify::Tsify;

macro_rules! decl_str {
    ($n:ident) => {
        #[derive(Tsify, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Deserialize)]
        #[serde(from = "&str")]
        #[tsify(from_wasm_abi)]
        pub struct $n(InternedHash<str>);

        impl<'a> From<&'a str> for $n {
            fn from(s: &'a str) -> Self {
                Self::new(s)
            }
        }

        impl Serialize for $n {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(&**self)
            }
        }

        impl $n {
            pub fn new(name: &str) -> Self {
                Self(hash_interner().intern_ref(name))
            }
        }

        impl Deref for $n {
            type Target = str;

            fn deref(&self) -> &Self::Target {
                self.0.as_ref()
            }
        }

        impl Borrow<str> for $n {
            fn borrow(&self) -> &str {
                self.0.borrow()
            }
        }

        impl fmt::Debug for $n {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({:?})", stringify!($n), self.0)
            }
        }

        impl fmt::Display for $n {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&*self.0)
            }
        }
    };
}

decl_str!(State);
decl_str!(Role);
decl_str!(Command);
decl_str!(EventType);

#[derive(Tsify, Serialize)]
#[serde(tag = "type")]
#[tsify(into_wasm_abi)]
pub enum CheckResult {
    OK,
    ERROR { errors: Vec<String> },
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct ProtocolType<L> {
    pub initial: State,
    pub transitions: Vec<Transition<L>>,
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct Transition<L> {
    pub label: L,
    pub source: State,
    pub target: State,
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "camelCase")]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct SwarmLabel {
    pub cmd: Command,
    pub log_type: Vec<EventType>,
    pub role: Role,
}

impl fmt::Display for SwarmLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}<", self.cmd, self.role)?;
        print_log(&self.log_type, f)?;
        write!(f, ">")
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(tag = "tag")]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub enum MachineLabel {
    #[serde(rename_all = "camelCase")]
    Execute {
        cmd: Command,
        log_type: Vec<EventType>,
    },
    #[serde(rename_all = "camelCase")]
    Input { event_type: EventType },
}

impl fmt::Display for MachineLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MachineLabel::Execute { cmd, log_type } => {
                write!(f, "{}/", cmd)?;
                print_log(&log_type, f)
            }
            MachineLabel::Input { event_type } => write!(f, "{event_type}?"),
        }
    }
}

fn print_log(log: &[EventType], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for (i, t) in log.iter().enumerate() {
        if i > 0 {
            write!(f, ",")?;
        }
        write!(f, "{}", t)?;
    }
    Ok(())
}

pub trait StateName {
    fn state_name(&self) -> &State;
}

impl StateName for State {
    fn state_name(&self) -> &State {
        self
    }
}

#[derive(Tsify, Serialize, Deserialize)]
#[serde(tag = "type")]
#[tsify(into_wasm_abi)]
pub enum DataResult<T> {
    OK { data: T },
    ERROR { errors: Vec<String> },
}