use crate::{
    machine::{Error, Side},
};
use machine_types::types::{
    projection::OptionGraph, proto_graph::NodeId, typescript_types::{Command, EventType, MachineLabel}
};
use petgraph::{
    visit::EdgeRef,
    Direction::Outgoing,
};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
enum DeterministicLabel {
    Command(Command),
    Event(EventType),
}

impl From<&MachineLabel> for DeterministicLabel {
    fn from(label: &MachineLabel) -> Self {
        match label {
            MachineLabel::Execute { cmd, .. } => DeterministicLabel::Command(cmd.clone()),
            MachineLabel::Input { event_type } => DeterministicLabel::Event(event_type.clone()),
        }
    }
}
fn state_name(graph: &OptionGraph, index: NodeId) -> String {
    match &graph[index] {
        None => "".to_string(),
        Some(s) => s.to_string(),
    }
}
/// error messages are designed assuming that `left` is the reference and `right` the tested
pub fn equivalent(left: &OptionGraph, li: NodeId, right: &OptionGraph, ri: NodeId) -> Vec<Error> {
    use Side::*;

    let _span = tracing::info_span!("equivalent").entered();

    let mut errors = Vec::new();

    // dfs traversal stack
    // must hold index pairs because node mappings might be m:n
    let mut stack = vec![(li, ri)];
    let mut visited = BTreeSet::new();

    while let Some((li, ri)) = stack.pop() {
        tracing::debug!(left = %state_name(left, li), ?li, right = %state_name(right, ri), ?ri, to_go = stack.len(), "loop");
        visited.insert((li, ri));
        // get all outgoing edge labels for the left side
        let mut l_out = BTreeMap::new();
        for edge in left.edges_directed(li, Outgoing) {
            l_out
                .entry(DeterministicLabel::from(edge.weight()))
                .and_modify(|_| errors.push(Error::NonDeterministic(Left, edge.id())))
                .or_insert(edge);
        }
        // get all outgoing edge labels for the right side
        let mut r_out = BTreeMap::new();
        for edge in right.edges_directed(ri, Outgoing) {
            r_out
                .entry(DeterministicLabel::from(edge.weight()))
                .and_modify(|_| errors.push(Error::NonDeterministic(Right, edge.id())))
                .or_insert(edge);
        }
        // keep note of stack so we can undo additions if !same
        let stack_len = stack.len();

        // compare both sets; iteration must be in order of weights (hence the BTreeMap above)
        let mut same = true;
        let mut l_edges = l_out.into_values().peekable();
        let mut r_edges = r_out.into_values().peekable();
        loop {
            let l = l_edges.peek();
            let r = r_edges.peek();
            match (l, r) {
                (None, None) => break,
                (None, Some(r_edge)) => {
                    tracing::debug!("left missing {} 1", r_edge.weight());
                    errors.push(Error::MissingTransition(Left, li, r_edge.id()));
                    same = false;
                    r_edges.next();
                }
                (Some(l_edge), None) => {
                    tracing::debug!("right missing {} 2", l_edge.weight());
                    errors.push(Error::MissingTransition(Right, ri, l_edge.id()));
                    same = false;
                    l_edges.next();
                }
                (Some(l_edge), Some(r_edge)) => match l_edge.weight().cmp(r_edge.weight()) {
                    Ordering::Less => {
                        tracing::debug!("right missing {}", l_edge.weight());
                        errors.push(Error::MissingTransition(Right, ri, l_edge.id()));
                        same = false;
                        l_edges.next();
                    }
                    Ordering::Equal => {
                        tracing::debug!("found match for {}", l_edge.weight());
                        let lt = l_edge.target();
                        let rt = r_edge.target();
                        if !visited.contains(&(lt, rt)) {
                            tracing::debug!(?lt, ?rt, "pushing targets");
                            stack.push((lt, rt));
                        }

                        l_edges.next();
                        r_edges.next();
                    }
                    Ordering::Greater => {
                        tracing::debug!("left missing {}", r_edge.weight());
                        errors.push(Error::MissingTransition(Left, li, r_edge.id()));
                        same = false;
                        r_edges.next();
                    }
                },
            }
        }
        if !same {
            // don’t bother visiting subsequent nodes if this one had discrepancies
            tracing::debug!("dumping {} stack elements", stack.len() - stack_len);
            stack.truncate(stack_len);
        }
    }

    errors
}

/* #[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        composition::{
            composition_swarm::{
                compose_protocols, exact_well_formed_sub, from_json,
                overapprox_well_formed_sub, swarms_to_proto_info,
            },
        },
        machine::{self},
        MachineType, Subscriptions, SwarmProtocolType,
    };
    use machine_types::types::typescript_types::{Command, EventType, Role, Transition, Granularity, InterfacingProtocols};
    use tracing_subscriber::{fmt, fmt::format::FmtSpan, EnvFilter};

    fn setup_logger() {
        fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
            .try_init()
            .ok();
    }

    pub(in crate::composition) fn from_option_machine(graph: &OptionGraph) -> Graph {
        graph.map(
            |_, n| n.clone().unwrap().state_name().clone(),
            |_, x| x.clone(),
        )
    }
    fn from_adaptation_graph_to_graph(graph: &AdaptationGraph) -> Graph {
        graph.map(|_, n| n.state.state_name().clone(), |_, x| x.clone())
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
                    { "source": "0", "target": "1", "label": { "cmd": "build", "logType": ["car"], "role": "F" } },
                    { "source": "1", "target": "2", "label": { "cmd": "test", "logType": ["report"], "role": "TR" } },
                    { "source": "2", "target": "3", "label": { "cmd": "accept", "logType": ["ok"], "role": "QCR" } },
                    { "source": "2", "target": "3", "label": { "cmd": "reject", "logType": ["notOk"], "role": "QCR" } }
                ]
            }"#,
        )
        .unwrap()
    }
    fn get_proto_4() -> SwarmProtocolType {
        serde_json::from_str::<SwarmProtocolType>(
            r#"{
                "initial": "0",
                "transitions": [
                    { "source": "0", "target": "1", "label": { "cmd": "observe", "logType": ["observing"], "role": "QCR" } },
                    { "source": "1", "target": "2", "label": { "cmd": "build", "logType": ["car"], "role": "F" } },
                    { "source": "2", "target": "3", "label": { "cmd": "test", "logType": ["report"], "role": "QCR" } }
                ]
            }"#,
        )
        .unwrap()
    }

    fn get_interfacing_swarms_1() -> InterfacingProtocols {
        InterfacingProtocols(vec![get_proto1(), get_proto2()])
    }

    fn get_interfacing_swarms_1_reversed() -> InterfacingProtocols {
        InterfacingProtocols(vec![get_proto2(), get_proto1()])
    }

    fn get_interfacing_swarms_2() -> InterfacingProtocols {
        InterfacingProtocols(vec![get_proto1(), get_proto2(), get_proto3()])
    }

    fn get_interfacing_swarms_2_reversed() -> InterfacingProtocols {
        InterfacingProtocols(vec![get_proto3(), get_proto2(), get_proto1()])
    }

    fn get_interfacing_swarms_3() -> InterfacingProtocols {
        InterfacingProtocols(vec![get_proto1(), get_proto2(), get_proto_4()])
    }

    fn get_interfacing_swarms_warehouse() -> InterfacingProtocols {
        InterfacingProtocols(vec![get_proto1()])
    }

    fn get_whf_transport() -> MachineType {
        serde_json::from_str::<MachineType>(
            r#"{
                "initial": "0",
                "transitions": [
                    {
                    "label": {
                        "tag": "Execute",
                        "cmd": "request",
                        "logType": [
                        "partID"
                        ]
                    },
                    "source": "0",
                    "target": "0"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "time"
                    },
                    "source": "0",
                    "target": "5"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "partID"
                    },
                    "source": "0",
                    "target": "1"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "pos"
                    },
                    "source": "1",
                    "target": "2"
                    },
                    {
                    "label": {
                        "tag": "Execute",
                        "cmd": "deliver",
                        "logType": [
                        "part"
                        ]
                    },
                    "source": "2",
                    "target": "2"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "part"
                    },
                    "source": "2",
                    "target": "3"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "time"
                    },
                    "source": "3",
                    "target": "4"
                    }
                ]
                }
            "#,
        )
        .unwrap()
    }

    fn get_whf_door() -> MachineType {
        serde_json::from_str::<MachineType>(
            r#"{
                "initial": "0",
                "transitions": [
                    {
                    "label": {
                        "tag": "Execute",
                        "cmd": "close",
                        "logType": [
                        "time"
                        ]
                    },
                    "source": "0",
                    "target": "0"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "time"
                    },
                    "source": "0",
                    "target": "4"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "partID"
                    },
                    "source": "0",
                    "target": "1"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "part"
                    },
                    "source": "1",
                    "target": "2"
                    },
                    {
                    "label": {
                        "tag": "Execute",
                        "cmd": "close",
                        "logType": [
                        "time"
                        ]
                    },
                    "source": "2",
                    "target": "2"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "time"
                    },
                    "source": "2",
                    "target": "3"
                    }
                ]
                }
            "#,
        )
        .unwrap()
    }

    fn get_whf_forklift() -> MachineType {
        serde_json::from_str::<MachineType>(
            r#"{
                "initial": "0",
                "transitions": [
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "time"
                    },
                    "source": "0",
                    "target": "5"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "partID"
                    },
                    "source": "0",
                    "target": "1"
                    },
                    {
                    "label": {
                        "tag": "Execute",
                        "cmd": "get",
                        "logType": [
                        "pos"
                        ]
                    },
                    "source": "1",
                    "target": "1"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "pos"
                    },
                    "source": "1",
                    "target": "2"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "part"
                    },
                    "source": "2",
                    "target": "3"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "time"
                    },
                    "source": "3",
                    "target": "4"
                    }
                ]
                }
            "#,
        )
        .unwrap()
    }

    fn get_whf_f() -> MachineType {
        serde_json::from_str::<MachineType>(
            r#"{
                "initial": "0",
                "transitions": [
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "time"
                    },
                    "source": "0",
                    "target": "6"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "partID"
                    },
                    "source": "0",
                    "target": "1"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "part"
                    },
                    "source": "1",
                    "target": "2"
                    },
                    {
                    "label": {
                        "tag": "Execute",
                        "cmd": "build",
                        "logType": [
                        "car"
                        ]
                    },
                    "source": "2",
                    "target": "2"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "time"
                    },
                    "source": "2",
                    "target": "3"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "car"
                    },
                    "source": "2",
                    "target": "4"
                    },
                    {
                    "label": {
                        "tag": "Execute",
                        "cmd": "build",
                        "logType": [
                        "car"
                        ]
                    },
                    "source": "3",
                    "target": "3"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "time"
                    },
                    "source": "4",
                    "target": "5"
                    },
                    {
                    "label": {
                        "tag": "Input",
                        "eventType": "car"
                    },
                    "source": "3",
                    "target": "5"
                    }
                ]
                }
            "#,
        )
        .unwrap()
    }

    mod projection_tests {
        use super::*;
            #[test]
        fn test_projection_1() {
            setup_logger();
            // From Combining Swarm Protocols, example 5.
            let proto = serde_json::from_str::<SwarmProtocolType>(
                r#"{
                    "initial": "0",
                    "transitions": [
                        { "source": "0", "target": "1", "label": { "cmd": "request", "logType": ["tireID"], "role": "C" } },
                        { "source": "1", "target": "2", "label": { "cmd": "retrieve", "logType": ["position"], "role": "W" } },
                        { "source": "2", "target": "3", "label": { "cmd": "receive", "logType": ["tire"], "role": "C" } },
                        { "source": "3", "target": "4", "label": { "cmd": "build", "logType": ["car"], "role": "F" } }
                    ]
                }"#,
            )
            .unwrap();
            // contains superfluous subscriptions, but to match example in article
            let subs = serde_json::from_str::<Subscriptions>(
                r#"{
                "C":["tireID","position","tire","car"],
                "W":["tireID","position","tire"],
                "F":["tireID","tire","car"]
            }"#,
            )
            .unwrap();

            let role = Role::new("F");
            let (g, i, _) = from_json(proto);
            let (proj, proj_initial) = project(&g, i.unwrap(), &subs, role, false);
            let expected_m = MachineType {
                initial: State::new("0"),
                transitions: vec![
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("tireID"),
                        },
                        source: State::new("0"),
                        target: State::new("2"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("tire"),
                        },
                        source: State::new("2"),
                        target: State::new("3"),
                    },
                    Transition {
                        label: MachineLabel::Execute {
                            cmd: Command::new("build"),
                            log_type: vec![EventType::new("car")],
                        },
                        source: State::new("3"),
                        target: State::new("3"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("car"),
                        },
                        source: State::new("3"),
                        target: State::new("4"),
                    },
                ],
            };
            let (expected, expected_initial, errors) = crate::machine::from_json(expected_m);
            assert!(errors.is_empty());
            assert!(expected_initial.is_some());
            // from equivalent(): "error messages are designed assuming that `left` is the reference and `right` the tested"
            assert!(equivalent(
                &expected,
                expected_initial.unwrap(),
                &to_option_machine(&proj),
                proj_initial
            )
            .is_empty());
        }

        #[test]
        fn test_projection_2() {
            setup_logger();
            // warehouse example from coplaws slides
            let proto = get_proto1();
            let result_subs =
                exact_well_formed_sub(InterfacingProtocols(vec![proto.clone()]), &BTreeMap::new());
            assert!(result_subs.is_ok());
            let subs = result_subs.unwrap();
            let role = Role::new("FL");
            let (g, i, _) = from_json(proto);
            let (left, left_initial) = project(&g, i.unwrap(), &subs, role.clone(), false);
            let right_m = MachineType {
                initial: State::new("0"),
                transitions: vec![
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("partID"),
                        },
                        source: State::new("0"),
                        target: State::new("1"),
                    },
                    Transition {
                        label: MachineLabel::Execute {
                            cmd: Command::new("get"),
                            log_type: vec![EventType::new("pos")],
                        },
                        source: State::new("1"),
                        target: State::new("1"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("pos"),
                        },
                        source: State::new("1"),
                        target: State::new("2"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("partID"),
                        },
                        source: State::new("2"),
                        target: State::new("1"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("time"),
                        },
                        source: State::new("2"),
                        target: State::new("3"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("time"),
                        },
                        source: State::new("0"),
                        target: State::new("3"),
                    },
                ],
            };
            let (right, right_initial, errors) = crate::machine::from_json(right_m);
            let right = from_option_machine(&right);
            let right = to_option_machine(&right);

            assert!(errors.is_empty());

            let errors = equivalent(
                &to_option_machine(&left),
                left_initial,
                &right,
                right_initial.unwrap(),
            );
            assert!(errors.is_empty());
        }

        #[test]
        fn test_projection_3() {
            setup_logger();
            // car factory from coplaws example
            let proto = get_proto2();
            let result_subs =
                exact_well_formed_sub(InterfacingProtocols(vec![proto.clone()]), &BTreeMap::new());
            assert!(result_subs.is_ok());
            let subs = result_subs.unwrap();
            let role = Role::new("F");
            let (g, i, _) = from_json(proto);
            let (proj, proj_initial) = project(&g, i.unwrap(), &subs, role, false);
            let expected_m = MachineType {
                initial: State::new("1"),
                transitions: vec![
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("part"),
                        },
                        source: State::new("1"),
                        target: State::new("2"),
                    },
                    Transition {
                        label: MachineLabel::Execute {
                            cmd: Command::new("build"),
                            log_type: vec![EventType::new("car")],
                        },
                        source: State::new("2"),
                        target: State::new("2"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("car"),
                        },
                        source: State::new("2"),
                        target: State::new("3"),
                    },
                ],
            };
            let (expected, expected_initial, errors) = crate::machine::from_json(expected_m);

            assert!(errors.is_empty());
            assert!(expected_initial.is_some());
            // from equivalent(): "error messages are designed assuming that `left` is the reference and `right` the tested"
            assert!(equivalent(
                &expected,
                expected_initial.unwrap(),
                &to_option_machine(&proj),
                proj_initial
            )
            .is_empty());
        }

        #[test]
        fn test_projection_4() {
            setup_logger();
            // car factory from coplaws example
            let protos = get_interfacing_swarms_1();
            let result_subs = overapprox_well_formed_sub(
                protos.clone(),
                &BTreeMap::from([(Role::new("T"), BTreeSet::from([EventType::new("car")]))]),
                Granularity::Coarse,
            );
            assert!(result_subs.is_ok());
            let subs = result_subs.unwrap();

            let role = Role::new("T");
            let (g, i) = compose_protocols(protos).unwrap();
            let (proj, proj_initial) = project(&g, i, &subs, role, false);
            let expected_m = MachineType {
                initial: State::new("0"),
                transitions: vec![
                    Transition {
                        label: MachineLabel::Execute {
                            cmd: Command::new("request"),
                            log_type: vec![EventType::new("partID")],
                        },
                        source: State::new("0"),
                        target: State::new("0"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("partID"),
                        },
                        source: State::new("0"),
                        target: State::new("1"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("time"),
                        },
                        source: State::new("0"),
                        target: State::new("2"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("pos"),
                        },
                        source: State::new("1"),
                        target: State::new("3"),
                    },
                    Transition {
                        label: MachineLabel::Execute {
                            cmd: Command::new("deliver"),
                            log_type: vec![EventType::new("part")],
                        },
                        source: State::new("3"),
                        target: State::new("3"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("part"),
                        },
                        source: State::new("3"),
                        target: State::new("4"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("time"),
                        },
                        source: State::new("4"),
                        target: State::new("5"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("car"),
                        },
                        source: State::new("5"),
                        target: State::new("7"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("car"),
                        },
                        source: State::new("4"),
                        target: State::new("6"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("time"),
                        },
                        source: State::new("6"),
                        target: State::new("7"),
                    },
                ],
            };
            let (expected, expected_initial, errors) = crate::machine::from_json(expected_m);

            assert!(errors.is_empty());
            assert!(expected_initial.is_some());
            // from equivalent(): "error messages are designed assuming that `left` is the reference and `right` the tested"
            assert!(equivalent(
                &expected,
                expected_initial.unwrap(),
                &to_option_machine(&proj),
                proj_initial
            )
            .is_empty());
        }

        #[test]
        fn test_projection_fail_1() {
            setup_logger();
            // warehouse example from coplaws slides
            let proto = get_proto1();
            let result_subs =
                exact_well_formed_sub(InterfacingProtocols(vec![proto.clone()]), &BTreeMap::new());
            assert!(result_subs.is_ok());
            let subs = result_subs.unwrap();
            let role = Role::new("FL");
            let (g, i, _) = from_json(proto);
            let (left, left_initial) = project(&g, i.unwrap(), &subs, role.clone(), false);
            let right_m = MachineType {
                initial: State::new("0"),
                transitions: vec![
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("partID"),
                        },
                        source: State::new("0"),
                        target: State::new("1"),
                    },
                    Transition {
                        label: MachineLabel::Execute {
                            cmd: Command::new("get"),
                            log_type: vec![EventType::new("pos")],
                        },
                        source: State::new("1"),
                        target: State::new("1"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("pos"),
                        },
                        source: State::new("1"),
                        target: State::new("2"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("time"),
                        },
                        source: State::new("1"),
                        target: State::new("3"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("time"),
                        },
                        source: State::new("2"),
                        target: State::new("3"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("time"),
                        },
                        source: State::new("0"),
                        target: State::new("3"),
                    },
                ],
            };
            let (right, right_initial, errors) = crate::machine::from_json(right_m);
            let right = from_option_machine(&right);
            let right = to_option_machine(&right);

            assert!(errors.is_empty());

            let errors = equivalent(
                &to_option_machine(&left),
                left_initial,
                &right,
                right_initial.unwrap(),
            );
            assert!(!errors.is_empty());
        }
        #[test]
        fn test_projection_fail_2() {
            setup_logger();
            // warehouse example from coplaws slides
            let proto = get_proto1();
            let result_subs =
                exact_well_formed_sub(InterfacingProtocols(vec![proto.clone()]), &BTreeMap::new());
            assert!(result_subs.is_ok());
            let subs = result_subs.unwrap();
            let role = Role::new("FL");
            let (g, i, _) = from_json(proto);
            let (left, left_initial) = project(&g, i.unwrap(), &subs, role.clone(), false);
            let right_m = MachineType {
                initial: State::new("0"),
                transitions: vec![
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("partID"),
                        },
                        source: State::new("0"),
                        target: State::new("1"),
                    },
                    Transition {
                        label: MachineLabel::Execute {
                            cmd: Command::new("get"),
                            log_type: vec![EventType::new("pos")],
                        },
                        source: State::new("1"),
                        target: State::new("1"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("pos"),
                        },
                        source: State::new("1"),
                        target: State::new("2"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("partID"),
                        },
                        source: State::new("2"),
                        target: State::new("2"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("time"),
                        },
                        source: State::new("2"),
                        target: State::new("3"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("time"),
                        },
                        source: State::new("0"),
                        target: State::new("3"),
                    },
                ],
            };
            let (right, right_initial, errors) = crate::machine::from_json(right_m);
            let right = from_option_machine(&right);
            let right = to_option_machine(&right);

            assert!(errors.is_empty());

            let errors = equivalent(
                &to_option_machine(&left),
                left_initial,
                &right,
                right_initial.unwrap(),
            );
            assert!(!errors.is_empty());
        }
        #[test]
        fn test_projection_fail_3() {
            setup_logger();
            // warehouse example from coplaws slides
            let proto = get_proto1();
            let result_subs =
                exact_well_formed_sub(InterfacingProtocols(vec![proto.clone()]), &BTreeMap::new());
            assert!(result_subs.is_ok());
            let subs = result_subs.unwrap();
            let role = Role::new("FL");
            let (g, i, _) = from_json(proto);
            let (left, left_initial) = project(&g, i.unwrap(), &subs, role.clone(), false);
            let right_m = MachineType {
                initial: State::new("0"),
                transitions: vec![
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("partID"),
                        },
                        source: State::new("0"),
                        target: State::new("1"),
                    },
                    Transition {
                        label: MachineLabel::Execute {
                            cmd: Command::new("get"),
                            log_type: vec![EventType::new("pos")],
                        },
                        source: State::new("1"),
                        target: State::new("1"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("pos"),
                        },
                        source: State::new("1"),
                        target: State::new("2"),
                    },
                    Transition {
                        label: MachineLabel::Execute {
                            cmd: Command::new("get"),
                            log_type: vec![EventType::new("pos")],
                        },
                        source: State::new("2"),
                        target: State::new("2"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("time"),
                        },
                        source: State::new("2"),
                        target: State::new("3"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("time"),
                        },
                        source: State::new("0"),
                        target: State::new("3"),
                    },
                ],
            };
            let (right, right_initial, errors) = crate::machine::from_json(right_m);
            let right = from_option_machine(&right);
            let right = to_option_machine(&right);

            assert!(errors.is_empty());

            let errors = equivalent(
                &to_option_machine(&left),
                left_initial,
                &right,
                right_initial.unwrap(),
            );
            assert!(!errors.is_empty());
        }
        #[test]
        fn test_projection_fail_4() {
            setup_logger();
            // warehouse example from coplaws slides
            let proto = get_proto1();
            let result_subs =
                exact_well_formed_sub(InterfacingProtocols(vec![proto.clone()]), &BTreeMap::new());
            assert!(result_subs.is_ok());
            let subs = result_subs.unwrap();
            let role = Role::new("FL");
            let (g, i, _) = from_json(proto);
            let (left, left_initial) = project(&g, i.unwrap(), &subs, role.clone(), false);
            let right_m = MachineType {
                initial: State::new("0"),
                transitions: vec![
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("partID"),
                        },
                        source: State::new("0"),
                        target: State::new("1"),
                    },
                    Transition {
                        label: MachineLabel::Execute {
                            cmd: Command::new("get"),
                            log_type: vec![EventType::new("pos")],
                        },
                        source: State::new("1"),
                        target: State::new("1"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("pos"),
                        },
                        source: State::new("1"),
                        target: State::new("2"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("time"),
                        },
                        source: State::new("2"),
                        target: State::new("3"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("time"),
                        },
                        source: State::new("0"),
                        target: State::new("3"),
                    },
                ],
            };
            let (right, right_initial, errors) = crate::machine::from_json(right_m);
            let right = from_option_machine(&right);
            let right = to_option_machine(&right);

            assert!(errors.is_empty());

            let errors = equivalent(
                &to_option_machine(&left),
                left_initial,
                &right,
                right_initial.unwrap(),
            );
            assert!(!errors.is_empty());
        }
    }

    mod machine_composition_tests {
        use super::*;

        #[test]
        fn test_combine_machines_1() {
            setup_logger();
            // Example from coplaws slides. Use generated WWF subscriptions. Project over T.
            let role = Role::new("T");
            let subs1 = crate::composition::composition_swarm::overapprox_well_formed_sub(
                get_interfacing_swarms_1(),
                &BTreeMap::new(),
                Granularity::Coarse,
            );
            assert!(subs1.is_ok());
            let subs1 = subs1.unwrap();
            let proto_info = swarms_to_proto_info(get_interfacing_swarms_1());
            assert!(proto_info.no_errors());

            let (proj_combined1, proj_combined_initial1) =
                project_combine(&proto_info, &subs1, role.clone(), false);

            let subs2 = crate::composition::composition_swarm::overapprox_well_formed_sub(
                get_interfacing_swarms_1_reversed(),
                &BTreeMap::new(),
                Granularity::Coarse,
            );
            assert!(subs2.is_ok());
            let subs2 = subs2.unwrap();
            let proto_info = swarms_to_proto_info(get_interfacing_swarms_1_reversed());
            assert!(proto_info.no_errors());

            let (proj_combined2, proj_combined_initial2) =
                project_combine(&proto_info, &subs2, role.clone(), false);

            // compose(a, b) should be equal to compose(b, a)
            assert_eq!(subs1, subs2);
            assert!(equivalent(
                &proj_combined1,
                proj_combined_initial1.unwrap(),
                &proj_combined2,
                proj_combined_initial2.unwrap()
            )
            .is_empty());

            let composition = compose_protocols(get_interfacing_swarms_1());
            assert!(composition.is_ok());
            let (composed_graph, composed_initial) = composition.unwrap();
            let (proj, proj_initial) = project(
                &composed_graph,
                composed_initial,
                &subs1,
                role.clone(),
                true,
            );

            assert!(equivalent(
                &proj_combined2,
                proj_combined_initial2.unwrap(),
                &to_option_machine(&proj),
                proj_initial
            )
            .is_empty());
        }

        #[test]
        fn test_combine_machines_2() {
            setup_logger();
            // fails when you use the exact subscriptions because that way not all roles subscribe to ALL interfaces? Ordering gets messed up.
            // the projected over the explicit composition may be correct, but the combined projections look weird and out of order.
            let composition = compose_protocols(get_interfacing_swarms_2());
            assert!(composition.is_ok());
            let (composed_graph, composed_initial) = composition.unwrap();
            let subs = crate::composition::composition_swarm::overapprox_well_formed_sub(
                get_interfacing_swarms_2(),
                &BTreeMap::new(),
                Granularity::Coarse,
            );
            assert!(subs.is_ok());
            let subs = subs.unwrap();
            let all_roles = vec![
                Role::new("T"),
                Role::new("FL"),
                Role::new("D"),
                Role::new("F"),
                Role::new("TR"),
                Role::new("QCR"),
            ];

            for role in all_roles {
                let subs1 = crate::composition::composition_swarm::overapprox_well_formed_sub(
                    get_interfacing_swarms_2(),
                    &BTreeMap::new(),
                    Granularity::Coarse,
                );
                assert!(subs1.is_ok());
                let subs1 = subs1.unwrap();
                let proto_info = swarms_to_proto_info(get_interfacing_swarms_2());
                assert!(proto_info.no_errors());

                let (proj_combined1, proj_combined_initial1) =
                    project_combine(&proto_info, &subs1, role.clone(), false);

                let subs2 = crate::composition::composition_swarm::overapprox_well_formed_sub(
                    get_interfacing_swarms_2_reversed(),
                    &BTreeMap::new(),
                    Granularity::Coarse,
                );
                assert!(subs2.is_ok());
                let subs2 = subs2.unwrap();
                let proto_info = swarms_to_proto_info(get_interfacing_swarms_2_reversed());
                assert!(proto_info.no_errors());

                let (proj_combined2, proj_combined_initial2) =
                    project_combine(&proto_info, &subs2, role.clone(), false);

                // compose(a, b) should be equal to compose(b, a)
                assert_eq!(subs1, subs2);
                assert!(equivalent(
                    &proj_combined1,
                    proj_combined_initial1.unwrap(),
                    &proj_combined2,
                    proj_combined_initial2.unwrap()
                )
                .is_empty());
                assert_eq!(subs2, subs);

                let (proj, proj_initial) =
                    project(&composed_graph, composed_initial, &subs, role.clone(), true);
                let errors = equivalent(
                    &proj_combined2,
                    proj_combined_initial2.unwrap(),
                    &to_option_machine(&proj),
                    proj_initial,
                );

                assert!(errors.is_empty());
            }
        }

        #[test]
        fn test_all_projs_whf() {
            setup_logger();
            let composition = compose_protocols(get_interfacing_swarms_1());
            assert!(composition.is_ok());
            let (composed_graph, composed_initial) = composition.unwrap();
            let subs = crate::composition::composition_swarm::overapprox_well_formed_sub(
                get_interfacing_swarms_1(),
                &BTreeMap::new(),
                Granularity::TwoStep
            );
            assert!(subs.is_ok());
            let subs = subs.unwrap();

            let all_roles = vec![
                Role::new("T"),
                Role::new("FL"),
                Role::new("D"),
                Role::new("F"),
            ];

            let expected_projs = BTreeMap::from([
                (Role::new("T"), get_whf_transport()),
                (Role::new("FL"), get_whf_forklift()),
                (Role::new("D"), get_whf_door()),
                (Role::new("F"), get_whf_f()),
            ]);

            for role in all_roles {
                let (expand_proj, expand_proj_initial) = project(
                    &composed_graph,
                    composed_initial,
                    &subs,
                    role.clone(),
                    true,
                );
                let (combined_proj, combined_proj_initial) =
                    project_combine(&swarms_to_proto_info(get_interfacing_swarms_1()), &subs, role.clone(), true);

                assert!(equivalent(
                    &to_option_machine(&expand_proj),
                    expand_proj_initial,
                    &combined_proj,
                    combined_proj_initial.unwrap())
                    .is_empty()
                );

                let (expected, expected_initial, _) = crate::machine::from_json(expected_projs.get(&role).unwrap().clone());

                assert!(equivalent(
                    &expected,
                    expected_initial.unwrap(),
                    &combined_proj,
                    combined_proj_initial.unwrap())
                    .is_empty());
            }
        }

        #[test]
        fn test_compose_zero() {
            let left = MachineType {
                initial: State::new("left_0"),
                transitions: vec![
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("a"),
                        },
                        source: State::new("left_0"),
                        target: State::new("left_1"),
                    },
                    Transition {
                        label: MachineLabel::Execute {
                            cmd: Command::new("cmd_a"),
                            log_type: vec![EventType::new("a")],
                        },
                        source: State::new("left_0"),
                        target: State::new("left_0"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("b"),
                        },
                        source: State::new("left_1"),
                        target: State::new("left_2"),
                    },
                    Transition {
                        label: MachineLabel::Execute {
                            cmd: Command::new("cmd_b"),
                            log_type: vec![EventType::new("b")],
                        },
                        source: State::new("left_1"),
                        target: State::new("left_1"),
                    },
                ],
            };
            let right = MachineType {
                initial: State::new("right_0"),
                transitions: vec![
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("b"),
                        },
                        source: State::new("right_0"),
                        target: State::new("right_1"),
                    },
                    Transition {
                        label: MachineLabel::Execute {
                            cmd: Command::new("cmd_b"),
                            log_type: vec![EventType::new("b")],
                        },
                        source: State::new("right_0"),
                        target: State::new("right_0"),
                    },
                    Transition {
                        label: MachineLabel::Input {
                            event_type: EventType::new("a"),
                        },
                        source: State::new("right_1"),
                        target: State::new("right_2"),
                    },
                    Transition {
                        label: MachineLabel::Execute {
                            cmd: Command::new("cmd_a"),
                            log_type: vec![EventType::new("a")],
                        },
                        source: State::new("right_1"),
                        target: State::new("right_1"),
                    },
                ],
            };
            let (left, left_initial, _) = crate::machine::from_json(left);
            let left = from_option_graph_to_graph(&left);
            let (right, right_initial, _) = crate::machine::from_json(right);
            let right = from_option_graph_to_graph(&right);
            let interface = BTreeSet::from([EventType::new("a"), EventType::new("b")]);
            let (combined, combined_initial) = compose(
                right,
                right_initial.unwrap(),
                left,
                left_initial.unwrap(),
                interface,
                gen_state_name,
            );
            let combined = to_json_machine(combined, combined_initial);

            let expected = MachineType {
                initial: State::new("right_0 || left_0"),
                transitions: vec![],
            };

            assert_eq!(combined, expected);
        }
    }

    // TODO:
    // Move tests related to adaptation and adaptation info to a module. Make one more (one that currently just prints).
    // Add a test somewhere that uses WH || F || QC

    #[test]
    fn test_example_from_text_machine() {
        setup_logger();
        let role = Role::new("F");
        let subs = crate::composition::composition_swarm::overapprox_well_formed_sub(
            get_interfacing_swarms_3(),
            &BTreeMap::new(),
            Granularity::Medium,
        );
        assert!(subs.is_ok());
        let subs = subs.unwrap();
        let proto_info = swarms_to_proto_info(get_interfacing_swarms_3());
        assert!(proto_info.no_errors());
        let (proj, proj_initial) = project_combine(&proto_info, &subs, role.clone(), false);
        println!(
            "projection of {}: {}",
            role.to_string(),
            serde_json::to_string_pretty(&from_option_to_machine(proj, proj_initial.unwrap()))
                .unwrap()
        );
    }

    #[test]
    #[ignore]
    fn combine_with_self() {
        setup_logger();

        let proto = get_proto1();
        let result_subs = overapprox_well_formed_sub(
            InterfacingProtocols(vec![proto.clone(), get_proto2()]),
            &BTreeMap::new(),
            Granularity::TwoStep,
        );
        assert!(result_subs.is_ok());
        let subs = result_subs.unwrap();
        println!("subs: {}", serde_json::to_string_pretty(&subs).unwrap());
        let role = Role::new("FL");
        let (g, i, _) = from_json(proto);
        let (left, left_initial) = project(&g, i.unwrap(), &subs, role.clone(), false);
        let right_m = MachineType {
            initial: State::new("0"),
            transitions: vec![
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("partID"),
                    },
                    source: State::new("0"),
                    target: State::new("1"),
                },
                Transition {
                    label: MachineLabel::Execute {
                        cmd: Command::new("get"),
                        log_type: vec![EventType::new("pos")],
                    },
                    source: State::new("1"),
                    target: State::new("1"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("pos"),
                    },
                    source: State::new("1"),
                    target: State::new("2"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("partID"),
                    },
                    source: State::new("2"),
                    target: State::new("1"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("time"),
                    },
                    source: State::new("2"),
                    target: State::new("3"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("time"),
                    },
                    source: State::new("0"),
                    target: State::new("3"),
                },
            ],
        };
        let (right, right_initial, errors) = crate::machine::from_json(right_m);
        let right = from_option_machine(&right);
        let right_option = to_option_machine(&right);

        println!(
            "left {:?}: {}",
            role.clone(),
            serde_json::to_string_pretty(&to_json_machine(left.clone(), left_initial)).unwrap()
        );
        println!(
            "right {:?}: {}",
            role,
            serde_json::to_string_pretty(&from_option_to_machine(
                right_option.clone(),
                right_initial.unwrap()
            ))
            .unwrap()
        );
        assert!(errors.is_empty());

        /* let errors = equivalent(
            &to_option_machine(&left),
            left_initial,
            &right_option,
            right_initial.unwrap());
        assert!(errors.is_empty());
        let errors: Vec<String> = errors.into_iter().map(crate::machine::Error::convert(&to_option_machine(&left), &right_option)).collect(); */
        println!("{:?}", errors);
        let interface = BTreeSet::from([
            EventType::new("partID"),
            EventType::new("pos"),
            EventType::new("time"),
        ]);
        // right left swapped here on purpose
        let (combined, combined_initial) = compose(
            right,
            right_initial.unwrap(),
            left,
            left_initial,
            interface,
            gen_state_name,
        );
        println!(
            "combined {:?}: {}",
            role.clone(),
            serde_json::to_string_pretty(&to_json_machine(combined.clone(), combined_initial))
                .unwrap()
        );
    }

    #[test]
    #[ignore]
    fn test_adapted_projection_fl() {
        setup_logger();

        let fl_m = MachineType {
            initial: State::new("0"),
            transitions: vec![
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("partID"),
                    },
                    source: State::new("0"),
                    target: State::new("1"),
                },
                Transition {
                    label: MachineLabel::Execute {
                        cmd: Command::new("get"),
                        log_type: vec![EventType::new("pos")],
                    },
                    source: State::new("1"),
                    target: State::new("1"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("pos"),
                    },
                    source: State::new("1"),
                    target: State::new("2"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("partID"),
                    },
                    source: State::new("2"),
                    target: State::new("1"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("time"),
                    },
                    source: State::new("2"),
                    target: State::new("3"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("time"),
                    },
                    source: State::new("0"),
                    target: State::new("3"),
                },
            ],
        };
        let (fl_m_graph, fl_m_graph_initial, _) = crate::machine::from_json(fl_m);

        let role = Role::new("FL");
        let swarms = get_interfacing_swarms_1();
        let subs1 = crate::composition::composition_swarm::overapprox_well_formed_sub(
            swarms.clone(),
            &BTreeMap::new(),
            Granularity::TwoStep,
        );
        assert!(subs1.is_ok());
        let subs1 = subs1.unwrap();
        println!("subs: {}", serde_json::to_string_pretty(&subs1).unwrap());
        let proto_info = swarms_to_proto_info(swarms.clone());
        assert!(proto_info.no_errors());

        let adapted = adapted_projection(
            &proto_info,
            &subs1,
            role.clone(),
            (fl_m_graph.clone(), fl_m_graph_initial.unwrap()),
            0,
            true,
        );
        let (adapted_proj, adapted_proj_initial) = adapted.unwrap();
        println!(
            "left {:?}: {}",
            role.clone(),
            serde_json::to_string_pretty(&from_option_to_machine(
                fl_m_graph.clone(),
                fl_m_graph_initial.unwrap()
            ))
            .unwrap()
        );
        println!(
            "right {:?}: {}",
            role,
            serde_json::to_string_pretty(&to_json_machine(
                from_adaptation_graph_to_graph(&adapted_proj.clone()),
                adapted_proj_initial.unwrap()
            ))
            .unwrap()
        );

        let role = Role::new("FL");
        let swarms = get_interfacing_swarms_3();
        let subs2 = crate::composition::composition_swarm::overapprox_well_formed_sub(
            swarms.clone(),
            &BTreeMap::new(),
            Granularity::TwoStep,
        );
        assert!(subs2.is_ok());
        let subs2 = subs2.unwrap();
        println!("subs: {}", serde_json::to_string_pretty(&subs2).unwrap());
        let proto_info = swarms_to_proto_info(swarms.clone());
        assert!(proto_info.no_errors());

        //let (adapted_proj, adapted_proj_initial) = adapted_projection(&proto_info.protocols, &subs2, role.clone(), (fl_m_graph.clone(), fl_m_graph_initial.unwrap()), 0);
        let adapted = adapted_projection(
            &proto_info,
            &subs2,
            role.clone(),
            (fl_m_graph.clone(), fl_m_graph_initial.unwrap()),
            0,
            true,
        );
        let (adapted_proj, adapted_proj_initial) = adapted.unwrap();
        println!(
            "left {:?}: {}",
            role.clone(),
            serde_json::to_string_pretty(&from_option_to_machine(
                fl_m_graph.clone(),
                fl_m_graph_initial.unwrap()
            ))
            .unwrap()
        );
        println!(
            "right {:?}: {}",
            role,
            serde_json::to_string_pretty(&to_json_machine(
                from_adaptation_graph_to_graph(&adapted_proj.clone()),
                adapted_proj_initial.unwrap()
            ))
            .unwrap()
        );
    }

    #[test]
    #[ignore]
    fn test_adapted_projection_r() {
        setup_logger();

        let f_m = MachineType {
            initial: State::new("0"),
            transitions: vec![
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("part"),
                    },
                    source: State::new("0"),
                    target: State::new("1"),
                },
                Transition {
                    label: MachineLabel::Execute {
                        cmd: Command::new("build"),
                        log_type: vec![EventType::new("car")],
                    },
                    source: State::new("1"),
                    target: State::new("1"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("car"),
                    },
                    source: State::new("1"),
                    target: State::new("2"),
                },
            ],
        };
        let (f_m_graph, f_m_graph_initial, _) = crate::machine::from_json(f_m);

        let role = Role::new("F");
        let swarms = get_interfacing_swarms_1();
        let subs1 = crate::composition::composition_swarm::overapprox_well_formed_sub(
            swarms.clone(),
            &BTreeMap::new(),
            Granularity::TwoStep,
        );
        assert!(subs1.is_ok());
        let subs1 = subs1.unwrap();
        println!("subs: {}", serde_json::to_string_pretty(&subs1).unwrap());
        let proto_info = swarms_to_proto_info(swarms.clone());
        assert!(proto_info.no_errors());

        let adapted = adapted_projection(
            &proto_info,
            &subs1,
            role.clone(),
            (f_m_graph.clone(), f_m_graph_initial.unwrap()),
            1,
            true,
        );
        let (adapted_proj, adapted_proj_initial) = adapted.unwrap();
        println!(
            "left {:?}: {}",
            role.clone(),
            serde_json::to_string_pretty(&from_option_to_machine(
                f_m_graph.clone(),
                f_m_graph_initial.unwrap()
            ))
            .unwrap()
        );
        println!(
            "right {:?}: {}",
            role,
            serde_json::to_string_pretty(&to_json_machine(
                from_adaptation_graph_to_graph(&adapted_proj.clone()),
                adapted_proj_initial.unwrap()
            ))
            .unwrap()
        );

        let role = Role::new("F");
        let swarms = get_interfacing_swarms_3();
        let subs2 = crate::composition::composition_swarm::overapprox_well_formed_sub(
            swarms.clone(),
            &BTreeMap::new(),
            Granularity::TwoStep,
        );
        assert!(subs2.is_ok());
        let subs2 = subs2.unwrap();
        println!("subs: {}", serde_json::to_string_pretty(&subs2).unwrap());
        let proto_info = swarms_to_proto_info(swarms.clone());
        assert!(proto_info.no_errors());

        let adapted = adapted_projection(
            &proto_info,
            &subs2,
            role.clone(),
            (f_m_graph.clone(), f_m_graph_initial.unwrap()),
            1,
            true,
        );
        let (adapted_proj, adapted_proj_initial) = adapted.unwrap();
        println!(
            "left {:?}: {}",
            role.clone(),
            serde_json::to_string_pretty(&from_option_to_machine(
                f_m_graph.clone(),
                f_m_graph_initial.unwrap()
            ))
            .unwrap()
        );
        println!(
            "right {:?}: {}",
            role,
            serde_json::to_string_pretty(&to_json_machine(
                from_adaptation_graph_to_graph(&adapted_proj.clone()),
                adapted_proj_initial.unwrap()
            ))
            .unwrap()
        );
    }

    #[test]
    #[ignore]
    fn test_det_proj_1() {
        setup_logger();
        let composition = compose_protocols(get_interfacing_swarms_1());
        assert!(composition.is_ok());
        let (composed_graph, composed_initial) = composition.unwrap();
        let subs = crate::composition::composition_swarm::exact_well_formed_sub(
            get_interfacing_swarms_1(),
            &BTreeMap::new(),
        );
        assert!(subs.is_ok());
        let subs = subs.unwrap();
        println!(
            "subscription: {}",
            serde_json::to_string_pretty(&subs).unwrap()
        );
        //let all_roles = vec![Role::new("T"), Role::new("FL"), Role::new("D"), Role::new("F")];
        let (proj, proj_initial) = project(
            &composed_graph,
            composed_initial,
            &subs,
            Role::new("D"),
            false,
        );
        println!(
            "{}: {}",
            Role::new("D").to_string(),
            serde_json::to_string_pretty(&to_json_machine(proj, proj_initial)).unwrap()
        );
        /* for role in all_roles {
            let (proj, proj_initial) = project(&composed_graph, composed_initial, &subs, role.clone());
            println!("{}: {}", role.clone().to_string(), serde_json::to_string_pretty(&to_json_machine(proj, proj_initial)).unwrap());
        } */
    }

    #[test]
    #[ignore]
    fn test_det_proj_2() {
        setup_logger();
        let composition = compose_protocols(get_interfacing_swarms_3());
        assert!(composition.is_ok());
        let (composed_graph, composed_initial) = composition.unwrap();
        let subs = crate::composition::composition_swarm::exact_well_formed_sub(
            get_interfacing_swarms_3(),
            &BTreeMap::new(),
        );
        assert!(subs.is_ok());
        let subs = subs.unwrap();
        //println!("subscription: {}", serde_json::to_string_pretty(&subs).unwrap());
        let (proj, proj_initial) = project(
            &composed_graph,
            composed_initial,
            &subs,
            Role::new("D"),
            false,
        );
        println!(
            "{}: {}",
            Role::new("D").to_string(),
            serde_json::to_string_pretty(&to_json_machine(proj, proj_initial)).unwrap()
        );
        /* let all_roles = vec![Role::new("T"), Role::new("FL"), Role::new("D"), Role::new("F"), Role::new("QCR")];
        for role in all_roles {
            let (proj, proj_initial) = project(&composed_graph, composed_initial, &subs, role.clone());
            //println!("{}: {}", role.clone().to_string(), serde_json::to_string_pretty(&to_json_machine(proj, proj_initial)).unwrap());
            println!("{}\n$$$$\n", serde_json::to_string_pretty(&to_json_machine(proj, proj_initial)).unwrap());
        } */
    }

    #[test]
    #[ignore]
    fn test_det_proj_3() {
        setup_logger();
        let composition = compose_protocols(get_interfacing_swarms_warehouse());
        assert!(composition.is_ok());
        let (composed_graph, composed_initial) = composition.unwrap();
        let subs = crate::composition::composition_swarm::exact_well_formed_sub(
            get_interfacing_swarms_warehouse(),
            &BTreeMap::new(),
        );
        assert!(subs.is_ok());
        let subs = subs.unwrap();
        //println!("subscription: {}", serde_json::to_string_pretty(&subs).unwrap());
        let all_roles = vec![Role::new("T"), Role::new("FL"), Role::new("D")];

        for role in all_roles {
            let (proj, proj_initial) = project(
                &composed_graph,
                composed_initial,
                &subs,
                role.clone(),
                false,
            );
            //println!("{}: {}", role.clone().to_string(), serde_json::to_string_pretty(&to_json_machine(proj, proj_initial)).unwrap());
            println!(
                "{}\n$$$$\n",
                serde_json::to_string_pretty(&to_json_machine(proj, proj_initial)).unwrap()
            );
        }
    }

    #[test]
    fn test_projection_information_1() {
        setup_logger();

        let fl_m = MachineType {
            initial: State::new("0"),
            transitions: vec![
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("partID"),
                    },
                    source: State::new("0"),
                    target: State::new("1"),
                },
                Transition {
                    label: MachineLabel::Execute {
                        cmd: Command::new("get"),
                        log_type: vec![EventType::new("pos")],
                    },
                    source: State::new("1"),
                    target: State::new("1"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("pos"),
                    },
                    source: State::new("1"),
                    target: State::new("2"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("partID"),
                    },
                    source: State::new("2"),
                    target: State::new("1"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("time"),
                    },
                    source: State::new("2"),
                    target: State::new("3"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("time"),
                    },
                    source: State::new("0"),
                    target: State::new("3"),
                },
            ],
        };

        let expected_proj = MachineType {
            initial: State::new("0 || { { 0 } } || { { 0 } }"),
            transitions: vec![
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("time"),
                    },
                    source: State::new("0 || { { 0 } } || { { 0 } }"),
                    target: State::new("3 || { { 3 } } || { { 0 } }"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("partID"),
                    },
                    source: State::new("0 || { { 0 } } || { { 0 } }"),
                    target: State::new("1 || { { 1 } } || { { 1 } }"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("pos"),
                    },
                    source: State::new("1 || { { 1 } } || { { 1 } }"),
                    target: State::new("2 || { { 2 } } || { { 1 } }"),
                },
                Transition {
                    label: MachineLabel::Execute {
                        cmd: Command::new("get"),
                        log_type: vec![EventType::new("pos")],
                    },
                    source: State::new("1 || { { 1 } } || { { 1 } }"),
                    target: State::new("1 || { { 1 } } || { { 1 } }"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("part"),
                    },
                    source: State::new("2 || { { 2 } } || { { 1 } }"),
                    target: State::new("2 || { { 0 } } || { { 2 } }"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("time"),
                    },
                    source: State::new("2 || { { 0 } } || { { 2 } }"),
                    target: State::new("3 || { { 3 } } || { { 2 } }"),
                },
            ],
        };

        let (fl_m_graph, fl_m_graph_initial, _) = crate::machine::from_json(fl_m);
        let role = Role::new("FL");
        let swarms = get_interfacing_swarms_1();
        let subs1 = crate::composition::composition_swarm::overapprox_well_formed_sub(
            swarms.clone(),
            &BTreeMap::new(),
            Granularity::TwoStep,
        );
        assert!(subs1.is_ok());
        let subs1 = subs1.unwrap();
        //println!("subs: {}", serde_json::to_string_pretty(&subs1).unwrap());
        let proto_info = swarms_to_proto_info(swarms.clone());

        let projection_info = projection_information(
            &proto_info,
            &subs1,
            role,
            (fl_m_graph.clone(), fl_m_graph_initial.unwrap()),
            0,
            true,
        );
        let projection_info = match projection_info {
            None => panic!(),
            Some(projection_info) => {
                //println!("proj: {}", serde_json::to_string_pretty(&projection_info.projection).unwrap());
                //println!("map: {}", serde_json::to_string_pretty(&projection_info.proj_to_machine_states).unwrap());
                //println!("branches: {}", serde_json::to_string_pretty(&projection_info.branches).unwrap());
                //println!("special event types: {}", serde_json::to_string_pretty(&projection_info.special_event_types).unwrap());
                projection_info
            }
        };
        let (actual_graph, actual_initial, _) = machine::from_json(projection_info.projection);
        let (expected_graph, expected_initial, _) = crate::machine::from_json(expected_proj);
        let expected_proj_to_machine_states = BTreeMap::from([
            (
                State::new("(0 || { { 0 } }) || { { 0 } }"),
                vec![State::new("0")],
            ),
            (
                State::new("(1 || { { 1 } }) || { { 1 } }"),
                vec![State::new("1")],
            ),
            (
                State::new("(2 || { { 0 } }) || { { 2 } }"),
                vec![State::new("2")],
            ),
            (
                State::new("(2 || { { 2 } }) || { { 1 } }"),
                vec![State::new("2")],
            ),
            (
                State::new("(3 || { { 3 } }) || { { 0 } }"),
                vec![State::new("3")],
            ),
            (
                State::new("(3 || { { 3 } }) || { { 2 } }"),
                vec![State::new("3")],
            ),
        ]);
        let expected_branches = BTreeMap::from([
            (EventType::new("part"), vec![EventType::new("time")]),
            (
                EventType::new("partID"),
                vec![
                    EventType::new("part"),
                    EventType::new("pos"),
                    EventType::new("time"),
                ],
            ),
            (
                EventType::new("pos"),
                vec![EventType::new("part"), EventType::new("time")],
            ),
            (EventType::new("time"), vec![]),
        ]);
        let expected_special_event_types =
            BTreeSet::from([EventType::new("partID"), EventType::new("time")]);
        let errors = equivalent(
            &expected_graph,
            expected_initial.unwrap(),
            &actual_graph,
            actual_initial.unwrap(),
        );
        let is_empty = errors.is_empty();
        //println!("{:?}", errors.map(machine::Error::convert(&expected_graph, &actual_graph)));
        assert!(is_empty);
        assert_eq!(
            expected_proj_to_machine_states,
            projection_info.proj_to_machine_states
        );
        assert_eq!(expected_branches, projection_info.branches);
        assert_eq!(
            expected_special_event_types,
            projection_info.special_event_types
        );
    }
    #[test]
    fn test_projection_information_2() {
        setup_logger();

        let fl_m = MachineType {
            initial: State::new("0"),
            transitions: vec![
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("partID"),
                    },
                    source: State::new("0"),
                    target: State::new("1"),
                },
                Transition {
                    label: MachineLabel::Execute {
                        cmd: Command::new("get"),
                        log_type: vec![EventType::new("pos")],
                    },
                    source: State::new("1"),
                    target: State::new("1"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("pos"),
                    },
                    source: State::new("1"),
                    target: State::new("0"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("time"),
                    },
                    source: State::new("0"),
                    target: State::new("3"),
                },
            ],
        };

        let expected_proj = MachineType {
            initial: State::new("0 || { { 0 } }"),
            transitions: vec![
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("time"),
                    },
                    source: State::new("0 || { { 0 } }"),
                    target: State::new("3 || { { 3 } }"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("partID"),
                    },
                    source: State::new("0 || { { 0 } }"),
                    target: State::new("1 || { { 1 } }"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("pos"),
                    },
                    source: State::new("1 || { { 1 } }"),
                    target: State::new("0 || { { 2 } }"),
                },
                Transition {
                    label: MachineLabel::Execute {
                        cmd: Command::new("get"),
                        log_type: vec![EventType::new("pos")],
                    },
                    source: State::new("1 || { { 1 } }"),
                    target: State::new("1 || { { 1 } }"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("part"),
                    },
                    source: State::new("0 || { { 2 } }"),
                    target: State::new("0 || { { 0 } }"),
                },
            ],
        };

        let (fl_m_graph, fl_m_graph_initial, _) = crate::machine::from_json(fl_m.clone());
        let role = Role::new("FL");
        let swarms: InterfacingProtocols = InterfacingProtocols(vec![get_proto1()]);
        let swarms_for_sub = get_interfacing_swarms_1();
        let larger_than_necessary_sub =
            crate::composition::composition_swarm::overapprox_well_formed_sub(
                swarms_for_sub,
                &BTreeMap::new(),
                Granularity::TwoStep,
            );
        assert!(larger_than_necessary_sub.is_ok());
        let subs1 = larger_than_necessary_sub.unwrap();
        //println!("subs: {}", serde_json::to_string_pretty(&subs1).unwrap());
        let proto_info = swarms_to_proto_info(swarms.clone());

        let projection_info = projection_information(
            &proto_info,
            &subs1,
            role,
            (fl_m_graph.clone(), fl_m_graph_initial.unwrap()),
            0,
            true,
        );
        let projection_info = match projection_info {
            None => panic!(),
            Some(projection_info) => {
                /* println!("proj: {}", serde_json::to_string_pretty(&projection_info.projection).unwrap());
                println!("fl_m: {}", serde_json::to_string_pretty(&fl_m).unwrap());
                println!("map: {}", serde_json::to_string_pretty(&projection_info.proj_to_machine_states).unwrap());
                println!("branches: {}", serde_json::to_string_pretty(&projection_info.branches).unwrap());
                println!("special event types: {}", serde_json::to_string_pretty(&projection_info.special_event_types).unwrap()); */
                projection_info
            }
        };
        let (actual_graph, actual_initial, _) = machine::from_json(projection_info.projection);
        let (expected_graph, expected_initial, _) = crate::machine::from_json(expected_proj);
        let expected_proj_to_machine_states = BTreeMap::from([
            (State::new("(0 || { { 0 } })"), vec![State::new("0")]),
            (State::new("(0 || { { 2 } })"), vec![State::new("0")]),
            (State::new("(1 || { { 1 } })"), vec![State::new("1")]),
            (State::new("(3 || { { 3 } })"), vec![State::new("3")]),
        ]);
        let expected_branches = BTreeMap::from([
            (
                EventType::new("part"),
                vec![EventType::new("partID"), EventType::new("time")],
            ),
            (
                EventType::new("partID"),
                vec![
                    EventType::new("part"),
                    EventType::new("partID"),
                    EventType::new("pos"),
                    EventType::new("time"),
                ],
            ),
            (
                EventType::new("pos"),
                vec![
                    EventType::new("part"),
                    EventType::new("partID"),
                    EventType::new("time"),
                ],
            ),
            (EventType::new("time"), vec![]),
        ]);
        let expected_special_event_types =
            BTreeSet::from([EventType::new("partID"), EventType::new("time")]);
        let errors = equivalent(
            &expected_graph,
            expected_initial.unwrap(),
            &actual_graph,
            actual_initial.unwrap(),
        );
        let is_empty = errors.is_empty();
        //println!("{:?}", errors.map(machine::Error::convert(&expected_graph, &actual_graph)));
        assert!(is_empty);
        assert_eq!(
            expected_proj_to_machine_states,
            projection_info.proj_to_machine_states
        );
        assert_eq!(expected_branches, projection_info.branches);
        assert_eq!(
            expected_special_event_types,
            projection_info.special_event_types
        );
    }

    mod big_example_i_can_be_deleted {
        use machine_types::types::typescript_types::DataResult;

        use super::*;

        fn get_steel_press_proto() -> SwarmProtocolType {
            serde_json::from_str::<SwarmProtocolType>(
                r#"{
                    "initial": "0",
                    "transitions": [
                        {
                        "source": "0",
                        "target": "1",
                        "label": {
                            "cmd": "pickUpSteelRoll",
                            "role": "SteelTransport",
                            "logType": [
                            "SteelRoll"
                            ]
                        }
                        },
                        {
                        "source": "1",
                        "target": "2",
                        "label": {
                            "cmd": "pressSteel",
                            "role": "Stamp",
                            "logType": [
                            "SteelParts"
                            ]
                        }
                        },
                        {
                        "source": "2",
                        "target": "0",
                        "label": {
                            "cmd": "assembleBody",
                            "role": "BodyAssembler",
                            "logType": [
                            "PartialCarBody"
                            ]
                        }
                        },
                        {
                        "source": "0",
                        "target": "3",
                        "label": {
                            "cmd": "carBodyDone",
                            "role": "CarBodyChecker",
                            "logType": [
                            "CarBody"
                            ]
                        }
                        }
                    ]
                }"#,
            )
            .unwrap()
        }

        fn get_paint_proto() -> SwarmProtocolType {
            serde_json::from_str::<SwarmProtocolType>(
                r#"{
                    "initial": "0",
                    "transitions": [
                        {
                        "source": "0",
                        "target": "1",
                        "label": {
                            "cmd": "carBodyDone",
                            "role": "CarBodyChecker",
                            "logType": [
                            "CarBody"
                            ]
                        }
                        },
                        {
                        "source": "1",
                        "target": "2",
                        "label": {
                            "cmd": "applyPaint",
                            "role": "Painter",
                            "logType": [
                            "PaintedCarBody"
                            ]
                        }
                        }
                    ]
                    }"#,
            )
            .unwrap()
        }
        fn get_engine_installation_proto() -> SwarmProtocolType {
            serde_json::from_str::<SwarmProtocolType>(
                r#"{
                    "initial": "0",
                    "transitions": [
                        {
                        "source": "0",
                        "target": "1",
                        "label": {
                            "cmd": "applyPaint",
                            "role": "Painter",
                            "logType": [
                            "PaintedCarBody"
                            ]
                        }
                        },
                        {
                        "source": "1",
                        "target": "2",
                        "label": {
                            "cmd": "requestEngine",
                            "role": "EngineInstaller",
                            "logType": [
                            "RequestEngine"
                            ]
                        }
                        },
                        {
                        "source": "2",
                        "target": "3",
                        "label": {
                            "cmd": "request",
                            "role": "Warehouse",
                            "logType": [
                            "ItemRequest"
                            ]
                        }
                        },
                        {
                        "source": "3",
                        "target": "4",
                        "label": {
                            "cmd": "deliver",
                            "role": "Warehouse",
                            "logType": [
                            "ItemDeliver"
                            ]
                        }
                        },
                        {
                        "source": "4",
                        "target": "5",
                        "label": {
                            "cmd": "installEngine",
                            "role": "EngineInstaller",
                            "logType": [
                            "EngineInstalled"
                            ]
                        }
                        }
                    ]
                    }"#,
            )
            .unwrap()
        }
        fn get_warehouse_proto() -> SwarmProtocolType {
            serde_json::from_str::<SwarmProtocolType>(
                r#"{
                    "initial": "0",
                    "transitions": [
                        {
                        "source": "0",
                        "target": "1",
                        "label": {
                            "cmd": "request",
                            "role": "Warehouse",
                            "logType": [
                            "ItemRequest"
                            ]
                        }
                        },
                        {
                        "source": "1",
                        "target": "1",
                        "label": {
                            "cmd": "bid",
                            "role": "Transport",
                            "logType": [
                            "Bid"
                            ]
                        }
                        },
                        {
                        "source": "1",
                        "target": "2",
                        "label": {
                            "cmd": "select",
                            "role": "Transport",
                            "logType": [
                            "Selected"
                            ]
                        }
                        },
                        {
                        "source": "2",
                        "target": "3",
                        "label": {
                            "cmd": "needGuidance",
                            "role": "Transport",
                            "logType": [
                            "ReqGuidance"
                            ]
                        }
                        },
                        {
                        "source": "3",
                        "target": "4",
                        "label": {
                            "cmd": "giveGuidance",
                            "role": "BaseStation",
                            "logType": [
                            "GiveGuidance"
                            ]
                        }
                        },
                        {
                        "source": "4",
                        "target": "5",
                        "label": {
                            "cmd": "basicPickup",
                            "role": "Transport",
                            "logType": [
                            "ItemPickupBasic"
                            ]
                        }
                        },
                        {
                        "source": "2",
                        "target": "5",
                        "label": {
                            "cmd": "smartPickup",
                            "role": "Transport",
                            "logType": [
                            "ItemPickupSmart"
                            ]
                        }
                        },
                        {
                        "source": "5",
                        "target": "6",
                        "label": {
                            "cmd": "handover",
                            "role": "Transport",
                            "logType": [
                            "Handover"
                            ]
                        }
                        },
                        {
                        "source": "6",
                        "target": "0",
                        "label": {
                            "cmd": "deliver",
                            "role": "Warehouse",
                            "logType": [
                            "ItemDeliver"
                            ]
                        }
                        }
                    ]
                    }"#,
            )
            .unwrap()
        }
        fn get_wheel_installation_proto() -> SwarmProtocolType {
            serde_json::from_str::<SwarmProtocolType>(
                r#"{
                    "initial": "0",
                    "transitions": [
                        {
                        "source": "0",
                        "target": "1",
                        "label": {
                            "cmd": "installEngine",
                            "role": "EngineInstaller",
                            "logType": [
                            "EngineInstalled"
                            ]
                        }
                        },
                        {
                        "source": "1",
                        "target": "2",
                        "label": {
                            "cmd": "pickUpWheel",
                            "role": "WheelInstaller",
                            "logType": [
                            "WheelPickup"
                            ]
                        }
                        },
                        {
                        "source": "2",
                        "target": "1",
                        "label": {
                            "cmd": "installWheel",
                            "role": "WheelInstaller",
                            "logType": [
                            "WheelInstalled"
                            ]
                        }
                        },
                        {
                        "source": "1",
                        "target": "3",
                        "label": {
                            "cmd": "wheelsDone",
                            "role": "WheelInstaller",
                            "logType": [
                            "AllWheelsInstalled"
                            ]
                        }
                        },
                        {
                        "source": "3",
                        "target": "4",
                        "label": {
                            "cmd": "carDone",
                            "role": "QualityTransport",
                            "logType": [
                            "FinishedCar"
                            ]
                        }
                        }
                    ]
                    } "#,
            )
            .unwrap()
        }

        fn get_interfacing_protocols_1() -> InterfacingProtocols {
                InterfacingProtocols(vec![
                    get_steel_press_proto(),
                    get_paint_proto(),
                    get_engine_installation_proto(),
                    get_warehouse_proto(),
                    get_wheel_installation_proto()])
            }

        fn get_wheel_installer() -> MachineType {
            serde_json::from_str::<MachineType>(
                r#"{
                    "initial": "s0",
                    "transitions": [
                        {
                        "source": "s1",
                        "target": "s1",
                        "label": {
                            "tag": "Execute",
                            "cmd": "pickUpWheel",
                            "logType": [
                            "WheelPickup"
                            ]
                        }
                        },
                        {
                        "source": "s1",
                        "target": "s1",
                        "label": {
                            "tag": "Execute",
                            "cmd": "wheelsDone",
                            "logType": [
                            "AllWheelsInstalled"
                            ]
                        }
                        },
                        {
                        "source": "s2",
                        "target": "s2",
                        "label": {
                            "tag": "Execute",
                            "cmd": "installWheel",
                            "logType": [
                            "WheelInstalled"
                            ]
                        }
                        },
                        {
                        "source": "s0",
                        "target": "s1",
                        "label": {
                            "tag": "Input",
                            "eventType": "EngineInstalled"
                        }
                        },
                        {
                        "source": "s1",
                        "target": "s2",
                        "label": {
                            "tag": "Input",
                            "eventType": "WheelPickup"
                        }
                        },
                        {
                        "source": "s1",
                        "target": "s3",
                        "label": {
                            "tag": "Input",
                            "eventType": "AllWheelsInstalled"
                        }
                        },
                        {
                        "source": "s2",
                        "target": "s1",
                        "label": {
                            "tag": "Input",
                            "eventType": "WheelInstalled"
                        }
                        }
                    ]
                    }"#,
            )
            .unwrap()
        }
        #[test]
        #[ignore]
        fn projection_information_wheel_installer() {
            let sub_result = overapprox_well_formed_sub(
                get_interfacing_protocols_1(),
                &BTreeMap::new(),
                Granularity::TwoStep,
            );
            let subscriptions = sub_result.unwrap();
            let projection_information =
                crate::composition::projection_information(Role::new(&format!("WheelInstaller")), get_interfacing_protocols_1(), 4, serde_json::to_string(&subscriptions).unwrap(), get_wheel_installer(), true);
            println!("ORIGINAL MACHINE: {}", serde_json::to_string_pretty(&get_wheel_installer()).unwrap());
            match projection_information {
                DataResult::ERROR {..} => assert!(false),
                DataResult::OK { data } =>  {
                    println!("ADAPTED: {}", serde_json::to_string_pretty(&data.projection).unwrap());
                }
            }
        }
    }
}
 */