use std::collections::{BTreeMap, BTreeSet};

use petgraph::visit::EdgeRef;

use crate::{errors::composition_errors::Error, types::{proto_graph::EdgeId, proto_info::{ProtoInfo, ProtoStruct}, typescript_types::{Command, EventLabel, EventType}}};

// Retrieve a graph or return an error.
macro_rules! get_ith_or_error {
    ($proto_info:expr, $proto_pointer:expr) => {
        match $proto_info.get_ith_proto($proto_pointer) {
            Some(ProtoStruct {
                graph: g,
                initial: Some(i),
                errors: e,
                roles: _,
            }) => (g, i, e),
            Some(ProtoStruct {
                graph: _,
                initial: None,
                errors: e,
                roles: _,
            }) => return e,
            None => return vec![Error::InvalidArg],
        }
    };
}

// Perform confusion freeness check on every protocol in a ProtoInfo.
pub fn confusion_free_proto_info(proto_info: ProtoInfo) -> ProtoInfo {
    let _span = tracing::info_span!("confusion_free_proto_info").entered();
    let protocols: Vec<_> = proto_info
        .protocols
        .clone()
        .into_iter()
        .enumerate()
        .map(|(i, p)| {
            let errors = vec![p.errors, confusion_free(&proto_info, i)].concat();
            ProtoStruct { errors, ..p }
        })
        .collect();

    ProtoInfo {
        protocols,
        ..proto_info
    }
}

// Check confusion-freeness of a concurrency-free protocol at index proto_pointer in proto_info.
fn confusion_free(proto_info: &ProtoInfo, proto_pointer: usize) -> Vec<Error> {
    let _span = tracing::info_span!("confusion_free").entered();
    let (graph, _, _) = get_ith_or_error!(proto_info, proto_pointer);

    // Map from event types to vec of edge id
    // Map from commands to vec of edge id
    // Error accumulator
    let mut event_types: BTreeMap<EventType, Vec<EdgeId>> = BTreeMap::new();
    let mut commands: BTreeMap<Command, Vec<EdgeId>> = BTreeMap::new();
    let mut errors = vec![];

    // Populate maps and check that each event type/command is only emitted/enabled in one transition.
    for edge in graph.edge_references() {
        let weight = edge.weight();
        event_types
            .entry(weight.get_event_type())
            .and_modify(|edge_ids| edge_ids.push(edge.id()))
            .or_insert_with(|| vec![edge.id()]);
        commands
            .entry(weight.cmd.clone())
            .and_modify(|edge_ids| edge_ids.push(edge.id()))
            .or_insert_with(|| vec![edge.id()]);
    }

    for (event_type, edge_indices) in event_types.iter() {
        if edge_indices.len() > 1 {
            errors.push(Error::EventEmittedMultipleTimes(
                event_type.clone(),
                edge_indices.clone(),
            ));
        }
    }
    for (command, edge_indices) in commands.iter() {
        if edge_indices.len() > 1 {
            errors.push(Error::CommandOnMultipleTransitions(
                command.clone(),
                edge_indices.clone(),
            ));
        }
    }

    errors
}

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

#[cfg(test)]
mod tests {
    use tracing_subscriber::{fmt, fmt::format::FmtSpan, EnvFilter};
    use super::*;
    use crate::types::typescript_types::{InterfacingProtocols, SwarmProtocolType};
    fn setup_logger() {
        fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
            .try_init()
            .ok();
    }

    fn get_proto1() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "request", "logType": ["partID"], "role": "T" } },
                    { "source": "1", "target": "2", "label": { "cmd": "get", "logType": ["pos"], "role": "FL" } },
                    { "source": "2", "target": "0", "label": { "cmd": "deliver", "logType": ["part"], "role": "T" } },
                    { "source": "0", "target": "3", "label": { "cmd": "close", "logType": ["time"], "role": "D" } }
                ]
            }"#,
        )
        .unwrap()
    }
    fn get_proto2() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "request", "logType": ["partID"], "role": "T" } },
                    { "source": "1", "target": "2", "label": { "cmd": "deliver", "logType": ["part"], "role": "T" } },
                    { "source": "2", "target": "3", "label": { "cmd": "build", "logType": ["car"], "role": "F" } }
                ]
            }"#,
        )
        .unwrap()
    }
    fn get_proto3() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "observe", "logType": ["report1"], "role": "TR" } },
                    { "source": "1", "target": "2", "label": { "cmd": "build", "logType": ["car"], "role": "F" } },
                    { "source": "2", "target": "3", "label": { "cmd": "test", "logType": ["report2"], "role": "TR" } },
                    { "source": "3", "target": "4", "label": { "cmd": "accept", "logType": ["ok"], "role": "QCR" } },
                    { "source": "3", "target": "4", "label": { "cmd": "reject", "logType": ["notOk"], "role": "QCR" } }
                ]
            }"#,
        )
        .unwrap()
    }

    fn get_interfacing_swarms_1() -> InterfacingProtocols {
        InterfacingProtocols(vec![get_proto1(), get_proto2()])
    }
        
    // two event types in close, request appears multiple times, get emits no events
    fn get_malformed_proto1() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "request", "logType": ["partID"], "role": "T" } },
                    { "source": "1", "target": "2", "label": { "cmd": "get", "logType": [], "role": "FL" } },
                    { "source": "2", "target": "0", "label": { "cmd": "request", "logType": ["part"], "role": "T" } },
                    { "source": "0", "target": "0", "label": { "cmd": "close", "logType": ["time", "time2"], "role": "D" } }
                ]
            }"#,
        )
        .unwrap()
    }

    // initial state state unreachable
    fn get_malformed_proto2() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "1", "target": "2", "label": { "cmd": "get", "logType": ["pos"], "role": "FL" } },
                    { "source": "2", "target": "3", "label": { "cmd": "deliver", "logType": ["partID"], "role": "T" } }
                ]
            }"#,
        )
        .unwrap()
    }

    // all states not reachable
    fn get_malformed_proto3() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "request", "logType": ["partID"], "role": "T" } },
                    { "source": "2", "target": "3", "label": { "cmd": "deliver", "logType": ["part"], "role": "T" } },
                    { "source": "4", "target": "5", "label": { "cmd": "build", "logType": ["car"], "role": "F" } }
                ]
            }"#,
        )
        .unwrap()
    }
    
    // pos event type associated with multiple commands and nondeterminism at 0.
    // No terminal state can be reached from any state -- OK according to confusion freeness
    fn get_confusionful_proto1() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "request", "logType": ["partID"], "role": "T" } },
                    { "source": "0", "target": "0", "label": { "cmd": "request", "logType": ["partID"], "role": "T" } },
                    { "source": "1", "target": "2", "label": { "cmd": "get", "logType": ["pos"], "role": "FL" } },
                    { "source": "2", "target": "0", "label": { "cmd": "request", "logType": ["pos"], "role": "T" } },
                    { "source": "0", "target": "0", "label": { "cmd": "close", "logType": ["time"], "role": "D" } }
                ]
            }"#,
        )
        .unwrap()
    }

    // No terminal state can be reached from any state -- OK according to confusion freeness
    fn get_some_nonterminating_proto() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "a", "logType": ["a"], "role": "a" } },
                    { "source": "0", "target": "2", "label": { "cmd": "c", "logType": ["c"], "role": "c" } },
                    { "source": "2", "target": "3", "label": { "cmd": "b", "logType": ["b"], "role": "b" } },
                    { "source": "1", "target": "4", "label": { "cmd": "d", "logType": ["d"], "role": "d" } },
                    { "source": "4", "target": "5", "label": { "cmd": "e", "logType": ["e"], "role": "e" } },
                    { "source": "5", "target": "1", "label": { "cmd": "f", "logType": ["f"], "role": "f" } }
                ]
            }"#,
        )
        .unwrap()
    }

    // Tests relating to confusion-freeness of protocols.
    mod confusion_freeness_tests {
        use std::collections::{BTreeMap, BTreeSet};

        use super::*;
        use crate::types::{proto_info, typescript_types::{EventType, Role, SwarmLabel}};

        #[test]
        fn test_prepare_graph_confusionfree() {
            setup_logger();
            let composition = get_interfacing_swarms_1();
            let proto_info = proto_info::combine_proto_infos(proto_info::prepare_proto_infos(composition));
            let proto_info = proto_info::explicit_composition_proto_info(proto_info);

            assert!(proto_info.get_ith_proto(0).is_some());
            assert!(proto_info.get_ith_proto(0).unwrap().errors.is_empty());
            assert_eq!(
                proto_info.concurrent_events,
                BTreeSet::from([
                    proto_info::unord_event_pair(EventType::new("time"), EventType::new("car")),
                    proto_info::unord_event_pair(EventType::new("pos"), EventType::new("car"))
                ])
            );
            assert_eq!(
                proto_info.branching_events,
                vec![BTreeSet::from([
                    EventType::new("time"),
                    EventType::new("partID")
                ])]
            );
            assert_eq!(proto_info.joining_events, BTreeMap::new());
            let expected_role_event_map = BTreeMap::from([
                (
                    Role::from("T"),
                    BTreeSet::from([
                        SwarmLabel {
                            cmd: Command::new("deliver"),
                            log_type: vec![EventType::new("part")],
                            role: Role::new("T"),
                        },
                        SwarmLabel {
                            cmd: Command::new("request"),
                            log_type: vec![EventType::new("partID")],
                            role: Role::new("T"),
                        },
                    ]),
                ),
                (
                    Role::from("FL"),
                    BTreeSet::from([SwarmLabel {
                        cmd: Command::new("get"),
                        log_type: vec![EventType::new("pos")],
                        role: Role::new("FL"),
                    }]),
                ),
                (
                    Role::from("D"),
                    BTreeSet::from([SwarmLabel {
                        cmd: Command::new("close"),
                        log_type: vec![EventType::new("time")],
                        role: Role::new("D"),
                    }]),
                ),
                (
                    Role::from("F"),
                    BTreeSet::from([SwarmLabel {
                        cmd: Command::new("build"),
                        log_type: vec![EventType::new("car")],
                        role: Role::new("F"),
                    }]),
                ),
            ]);
            assert_eq!(proto_info.role_event_map, expected_role_event_map);
            let proto_info = proto_info::prepare_proto_info(get_proto1());
            assert!(proto_info.get_ith_proto(0).is_some());
            assert!(proto_info.get_ith_proto(0).unwrap().errors.is_empty());
            assert_eq!(proto_info.concurrent_events, BTreeSet::new());
            assert_eq!(
                proto_info.branching_events,
                vec![BTreeSet::from([
                    EventType::new("time"),
                    EventType::new("partID")
                ])]
            );
            assert_eq!(proto_info.joining_events, BTreeMap::new());

            let proto_info = proto_info::prepare_proto_info(get_proto2());
            assert!(proto_info.get_ith_proto(0).is_some());
            assert!(proto_info.get_ith_proto(0).unwrap().errors.is_empty());
            assert_eq!(proto_info.concurrent_events, BTreeSet::new());
            assert_eq!(proto_info.branching_events, Vec::new());
            assert_eq!(proto_info.joining_events, BTreeMap::new());

            let proto_info = proto_info::prepare_proto_info(get_proto3());
            assert!(proto_info.get_ith_proto(0).is_some());
            assert!(proto_info.get_ith_proto(0).unwrap().errors.is_empty());
            assert_eq!(proto_info.concurrent_events, BTreeSet::new());

            // Should not contain any branching event types since only state with two outgoing is 3
            // and both of these outgoing transitions go to state 4:
            // { "source": "3", "target": "4", "label": { "cmd": "accept", "logType": ["ok"], "role": "QCR" } },
            // { "source": "3", "target": "4", "label": { "cmd": "reject", "logType": ["notOk"], "role": "QCR" } }
            assert_eq!(proto_info.branching_events, vec![]);
            assert_eq!(proto_info.joining_events, BTreeMap::new());
        }

        #[test]
        fn test_prepare_graph_malformed() {
            setup_logger();
            let proto1 = get_malformed_proto1();
            let proto_info = proto_info::prepare_proto_info(proto1.clone());
            let mut errors: Vec<String> = vec![proto_info.get_ith_proto(0).unwrap().errors]
                .concat()
                .into_iter()
                .map(Error::convert(&proto_info.get_ith_proto(0).unwrap().graph))
                .collect();

            let mut expected_erros = vec![
                "transition (0)--[close@D<time,time2>]-->(0) emits more than one event type",
                "log type must not be empty (1)--[get@FL<>]-->(2)",
            ];
            errors.sort();
            expected_erros.sort();
            assert_eq!(errors, expected_erros);

            let proto_info = proto_info::prepare_proto_info(get_malformed_proto2());
            let errors: Vec<String> = vec![
                confusion_free(&proto_info, 0),
                proto_info.get_ith_proto(0).unwrap().errors,
            ]
            .concat()
            .into_iter()
            .map(Error::convert(&proto_info.get_ith_proto(0).unwrap().graph))
            .collect();

            let expected_errors = vec![
                "initial swarm protocol state has no transitions",
                "initial swarm protocol state has no transitions",
            ];
            assert_eq!(errors, expected_errors);

            let proto_info = proto_info::prepare_proto_info(get_malformed_proto3());
            let errors: Vec<String> = proto_info
                .get_ith_proto(0)
                .unwrap()
                .errors
                .into_iter()
                .map(Error::convert(&proto_info.get_ith_proto(0).unwrap().graph))
                .collect();

            let expected_errors = vec![
                "state 2 is unreachable from initial state",
                "state 3 is unreachable from initial state",
                "state 4 is unreachable from initial state",
                "state 5 is unreachable from initial state",
            ];
            assert_eq!(errors, expected_errors);
        }

        // pos event type associated with multiple commands and nondeterminism at 0
        #[test]
        fn test_prepare_graph_confusionful() {
            setup_logger();
            let proto = get_confusionful_proto1();

            let proto_info = proto_info::prepare_proto_info(proto); //proto, None);
            let mut errors: Vec<String> = vec![
                confusion_free(&proto_info, 0),
                proto_info.get_ith_proto(0).unwrap().errors,
            ]
            .concat()
            .into_iter()
            .map(Error::convert(&proto_info.get_ith_proto(0).unwrap().graph))
            .collect();

            let mut expected_errors = vec![
                "command request enabled in more than one transition: (0)--[request@T<partID>]-->(1), (0)--[request@T<partID>]-->(0), (2)--[request@T<pos>]-->(0)",
                "event type partID emitted in more than one transition: (0)--[request@T<partID>]-->(1), (0)--[request@T<partID>]-->(0)",
                "event type pos emitted in more than one transition: (1)--[get@FL<pos>]-->(2), (2)--[request@T<pos>]-->(0)",
            ];
            errors.sort();
            expected_errors.sort();
            assert_eq!(errors, expected_errors);

            let proto = get_some_nonterminating_proto();
            let proto_info = proto_info::prepare_proto_info(proto);
            let mut errors: Vec<String> = vec![
                confusion_free(&proto_info, 0),
                proto_info.get_ith_proto(0).unwrap().errors,
            ]
            .concat()
            .into_iter()
            .map(Error::convert(&proto_info.get_ith_proto(0).unwrap().graph))
            .collect();

            let mut expected_errors: Vec<String> = vec![];
            errors.sort();
            expected_errors.sort();
            assert_eq!(errors, expected_errors);
        }
    }
}