use std::collections::BTreeSet;

use crate::{errors::composition_errors::Error, types::{proto_info::ProtoInfo, typescript_types::{Command, EventType}}};

// Check that for any c@R<t> in proto_info1, c'@R'<t> in proto_info2 c = c' and R = R'
fn cross_protocol_event_type_errors(
    proto_info1: &ProtoInfo,
    proto_info2: &ProtoInfo,
) -> Vec<Error> {
    // Map event types to the their associated (command, role) pairs in their protocol.
    let event_type_map1 = proto_info1.event_type_map();
    let event_type_map2 = proto_info2.event_type_map();
    let event_type_intersection: Vec<EventType> = event_type_map1
        .keys()
        .cloned()
        .collect::<BTreeSet<EventType>>()
        .intersection(
            &event_type_map2
                .keys()
                .cloned()
                .collect::<BTreeSet<EventType>>(),
        )
        .cloned()
        .collect();

    // True if map1 and map2 both contain t but map1[t] is not equal to map2[t]
    let event_type_violation_filter = |t: &EventType| -> bool {
        match (event_type_map1.get(t), event_type_map2.get(t)) {
            (Some((c1, r1)), Some((c2, r2))) if *c1 != *c2 || *r1 != *r2 => true,
            _ => false,
        }
    };

    // Map any event type violations to errors
    let event_type_errors = event_type_intersection
        .into_iter()
        .filter(|t| event_type_violation_filter(t))
        .map(|t| {
            let (c1, r1) = event_type_map1.get(&t).unwrap();
            let (c2, r2) = event_type_map2.get(&t).unwrap();
            Error::EventTypeOnDifferentLabels(
                t.clone(),
                c1.clone(),
                r1.clone(),
                c2.clone(),
                r2.clone(),
            )
        })
        .collect();

    event_type_errors
}

// Check that for any c@R<t> in proto_info1, c@R'<t'> in proto_info2 t = t' and R = R'
fn cross_protocol_command_errors(proto_info1: &ProtoInfo, proto_info2: &ProtoInfo) -> Vec<Error> {
    // Map commands to the their associated (command, role) pairs in their protocol.
    let command_map1 = proto_info1.command_map();
    let command_map2 = proto_info2.command_map();
    let command_intersection: Vec<Command> = command_map1
        .keys()
        .cloned()
        .collect::<BTreeSet<Command>>()
        .intersection(&command_map2.keys().cloned().collect::<BTreeSet<Command>>())
        .cloned()
        .collect();

    // True if map1 and map2 both contain t but map1[t] is not equal to map2[t]
    let command_violation_filter = |c: &Command| -> bool {
        match (command_map1.get(c), command_map2.get(c)) {
            (Some((t1, r1)), Some((t2, r2))) if *t1 != *t2 || *r1 != *r2 => true,
            _ => false,
        }
    };

    // Map any command violations to errors
    let command_errors = command_intersection
        .into_iter()
        .filter(|t| command_violation_filter(t))
        .map(|c| {
            let (t1, r1) = command_map1.get(&c).unwrap();
            let (t2, r2) = command_map2.get(&c).unwrap();
            Error::CommandOnDifferentLabels(
                c.clone(),
                t1.clone(),
                r1.clone(),
                t2.clone(),
                r2.clone(),
            )
        })
        .collect();

    command_errors
}

// Checks that event types (commands) appearing in different swarm protocols are associated with the same commands (event types) and roles
pub fn check_interface(proto_info1: &ProtoInfo, proto_info2: &ProtoInfo) -> Vec<Error> {
    vec![
        cross_protocol_event_type_errors(proto_info1, proto_info2),
        cross_protocol_command_errors(proto_info1, proto_info2),
    ]
    .concat()
}