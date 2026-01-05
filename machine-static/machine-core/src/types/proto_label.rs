use std::collections::BTreeSet;

use crate::types::{
    proto_graph::Graph,
    proto_info::ProtoInfo,
    typescript_types::{Command, EventLabel, EventType, Role},
};

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
