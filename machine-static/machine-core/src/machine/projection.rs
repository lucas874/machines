use std::collections::BTreeSet;

use petgraph::{
    Direction::{Incoming, Outgoing},
    graph::EdgeReference,
    visit::{EdgeFiltered, EdgeRef, IntoEdgeReferences, IntoEdgesDirected, IntoNodeReferences},
};

use crate::{
    machine::minimize,
    types::{
        projection::{ChainedProjections, ChainedProtos, OptionGraph},
        proto_info::{ProtoInfo, ProtoStruct},
        typescript_types::EventType,
    },
};

use crate::composition;
use crate::types::{
    projection::Graph,
    proto_graph::NodeId,
    typescript_types::{EventLabel, MachineLabel, Role, StateName, Subscriptions, SwarmLabel},
};

// Edge reference for graphs representing protocols. Used when filtering out edges that are not to be included in projection.
type ERef<'a> = <&'a crate::types::proto_graph::Graph as IntoEdgeReferences>::EdgeRef;

// Similar to machine::project, except that transitions with event types
// not subscribed to by role are skipped.
pub fn project(
    swarm: &crate::types::proto_graph::Graph,
    initial: NodeId,
    subs: &Subscriptions,
    role: Role,
    minimize: bool,
) -> (Graph, NodeId) {
    let _span = tracing::info_span!("project", %role).entered();
    let mut machine = Graph::new();
    let sub = BTreeSet::new();
    let sub = subs.get(&role).unwrap_or(&sub);
    // need to keep track of corresponding machine node for each swarm node. maps nodes in protocol to nodes in projection
    let mut m_nodes: Vec<NodeId> = vec![NodeId::end(); swarm.node_count()];

    let interested = |edge: ERef| sub.contains(&edge.weight().get_event_type());
    let filtered = EdgeFiltered(swarm, interested);

    // find all nodes that should be in the projection
    let nodes_in_proj: Vec<NodeId> = swarm
        .node_references()
        .filter(|(ni, _)| *ni == initial || filtered.edges_directed(*ni, Incoming).count() > 0)
        .map(|(ni, _)| ni)
        .collect();

    // add the nodes identified above
    for node in nodes_in_proj.iter() {
        m_nodes[node.index()] = machine.add_node(swarm[*node].state_name().clone());
    }

    let find_interesting_edges = |node: NodeId| -> Vec<EdgeReference<'_, SwarmLabel>> {
        let mut stack: Vec<NodeId> = vec![node];
        let mut visited: BTreeSet<NodeId> = BTreeSet::from([node]);
        let mut interesting_edges: Vec<EdgeReference<'_, SwarmLabel>> = vec![];

        while let Some(n) = stack.pop() {
            for edge in swarm.edges_directed(n, Outgoing) {
                if sub.contains(&edge.weight().get_event_type()) {
                    interesting_edges.push(edge);
                } else {
                    if !visited.contains(&edge.target()) {
                        stack.push(edge.target());
                        visited.insert(edge.target());
                    }
                }
            }
        }

        interesting_edges
    };

    for node in nodes_in_proj {
        let interesting_edges: Vec<_> = find_interesting_edges(node);
        for edge in interesting_edges {
            if edge.weight().role == role {
                let execute_label = MachineLabel::Execute {
                    cmd: edge.weight().cmd.clone(),
                    log_type: vec![edge.weight().get_event_type()],
                };
                machine.add_edge(m_nodes[node.index()], m_nodes[node.index()], execute_label);
            }
            let input_label = MachineLabel::Input {
                event_type: edge.weight().get_event_type(),
            };
            machine.add_edge(
                m_nodes[node.index()],
                m_nodes[edge.target().index()],
                input_label,
            );
        }
    }
    //(machine, m_nodes[initial.index()])
    if minimize {
        let (dfa, dfa_initial) = minimize::nfa_to_dfa(machine, m_nodes[initial.index()]); // make deterministic. slight deviation from projection operation formally.
        minimize::minimal_machine(&dfa, dfa_initial) // when minimizing we get a machine that is a little different but equivalent to the one prescribed by the projection operator formally
    } else {
        (machine, m_nodes[initial.index()])
    }
}

// Map the protocols of a proto_info to a ChainedProtos
pub(crate) fn to_chained_protos(proto_info: &ProtoInfo) -> ChainedProtos {
    let folder = |(acc, roles_prev): (ChainedProtos, BTreeSet<Role>),
                  proto: ProtoStruct|
     -> (ChainedProtos, BTreeSet<Role>) {
        let interfacing_event_types = roles_prev
            .intersection(&proto.roles)
            .flat_map(|role| {
                proto_info
                    .role_event_map
                    .get(role)
                    .unwrap()
                    .iter()
                    .map(|swarm_label| swarm_label.get_event_type())
            })
            .collect();
        let acc = acc
            .into_iter()
            .chain([(proto.graph, proto.initial.unwrap(), interfacing_event_types)])
            .collect();

        (acc, proto.roles.union(&roles_prev).cloned().collect())
    };
    let (chained_protos, _) = proto_info
        .protocols
        .clone()
        .into_iter()
        .fold((vec![], BTreeSet::new()), folder);
    chained_protos
}

// Map a ChainedProtos to a ChainedProjections
pub(crate) fn to_chained_projections(
    chained_protos: ChainedProtos,
    subs: &Subscriptions,
    role: Role,
    minimize: bool,
) -> ChainedProjections {
    let mapper = |(graph, initial, interface)| -> (Graph, NodeId, BTreeSet<EventType>) {
        let (projection, projection_initial) =
            project(&graph, initial, subs, role.clone(), minimize);
        (projection, projection_initial, interface)
    };

    chained_protos.into_iter().map(mapper).collect()
}

// precondition: the protocols interfaces on the supplied interfaces.
// precondition: the composition of the protocols in swarms is wwf w.r.t. subs.
pub fn project_combine(
    proto_info: &ProtoInfo,
    subs: &Subscriptions,
    role: Role,
    minimize: bool,
) -> (OptionGraph, Option<NodeId>) {
    let _span = tracing::info_span!("project_combine", %role).entered();

    let projections = to_chained_projections(to_chained_protos(proto_info), subs, role, minimize);

    match combine_projs(projections, composition::gen_state_name) {
        Some((combined_projection, combined_initial)) =>
        //let (combined_projection, combined_initial) = minimal_machine(&combined_projection, combined_initial);
        // option because used in equivalent. Consider changing.
        {
            (
                to_option_machine(&combined_projection),
                Some(combined_initial),
            )
        }
        None => (OptionGraph::new(), Some(NodeId::end())),
    }
}

// rename this to combine_projections.
pub(crate) fn combine_projs<N: Clone, E: Clone + EventLabel>(
    projections: Vec<(petgraph::Graph<N, E>, NodeId, BTreeSet<EventType>)>,
    gen_node: fn(&N, &N) -> N,
) -> Option<(petgraph::Graph<N, E>, NodeId)> {
    let _span = tracing::info_span!("combine_projs").entered();
    if projections.is_empty() {
        return None;
    }
    let (acc_machine, acc_initial, _) = projections[0].clone();
    let (combined_projection, combined_initial) = projections[1..].to_vec().into_iter().fold(
        (acc_machine, acc_initial),
        |(acc, acc_i), (m, i, interface)| {
            composition::compose(acc, acc_i, m, i, interface, gen_node)
        },
    );
    Some((combined_projection, combined_initial))
}

fn to_option_machine(graph: &Graph) -> OptionGraph {
    graph.map(|_, n| Some(n.state_name().clone()), |_, x| x.clone())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::machine::util;
    use crate::subscription::{exact, overapproximation};
    use crate::types::typescript_types::{
        Command, Granularity, InterfacingProtocols, MachineType, State, Transition,
    };
    use crate::types::{proto_graph, proto_info};
    use crate::{test_utils, types::typescript_types::SwarmProtocolType};

    /* fn print_machines(m1: &MachineType, m2: &MachineType) {
        println!("{}", serde_json::to_string_pretty(&m1).unwrap());
        println!("{}", serde_json::to_string_pretty(&m2).unwrap());
    } */

    #[test]
    fn test_projection_1() {
        test_utils::setup_logger();
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
        let subs = serde_json::from_str::<Subscriptions>(
            r#"{
            "C":["tireID","position","tire","car"],
            "W":["tireID","position","tire"],
            "F":["tireID","tire","car"]
        }"#,
        )
        .unwrap();

        let role = Role::new("F");
        let (g, i, _) = proto_graph::from_json(proto);
        let (proj, proj_initial) = project(&g, i.unwrap(), &subs, role, false);
        let mut proj_machine = util::to_json_machine(proj, proj_initial);
        let mut expected_machine = MachineType {
            initial: State::new("0"),
            transitions: vec![
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("tireID"),
                    },
                    source: State::new("0"),
                    target: State::new("1"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("tire"),
                    },
                    source: State::new("1"),
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
        proj_machine.transitions.sort();
        expected_machine.transitions.sort();
        assert_eq!(proj_machine, expected_machine)
    }

    #[test]
    fn test_projection_2() {
        test_utils::setup_logger();
        // warehouse example from coplaws slides
        let proto = test_utils::get_proto1();
        let result_subs = exact::exact_well_formed_sub(
            InterfacingProtocols(vec![proto.clone()]),
            &BTreeMap::new(),
        );
        assert!(result_subs.is_ok());
        let subs = result_subs.unwrap();
        let role = Role::new("FL");
        let (g, i, _) = proto_graph::from_json(proto);
        let (proj, proj_initial) = project(&g, i.unwrap(), &subs, role.clone(), false);
        let mut proj_machine = util::to_json_machine(proj, proj_initial);
        let mut expected_machine = MachineType {
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
        proj_machine.transitions.sort();
        expected_machine.transitions.sort();
        assert_eq!(proj_machine, expected_machine);
    }

    #[test]
    fn test_projection_3() {
        test_utils::setup_logger();
        // car factory from coplaws example
        let proto = test_utils::get_proto2();
        let result_subs = exact::exact_well_formed_sub(
            InterfacingProtocols(vec![proto.clone()]),
            &BTreeMap::new(),
        );
        assert!(result_subs.is_ok());
        let subs = result_subs.unwrap();
        let role = Role::new("F");
        let (g, i, _) = proto_graph::from_json(proto);
        let (proj, proj_initial) = project(&g, i.unwrap(), &subs, role, false);
        let mut proj_machine = util::to_json_machine(proj, proj_initial);
        let mut expected_machine = MachineType {
            initial: State::new("0"),
            transitions: vec![
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("part"),
                    },
                    source: State::new("0"),
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
        proj_machine.transitions.sort();
        expected_machine.transitions.sort();
        assert_eq!(proj_machine, expected_machine);
    }

    #[test]
    fn test_projection_4() {
        test_utils::setup_logger();
        // car factory from coplaws example
        let protos = test_utils::get_interfacing_swarms_1();
        let result_subs = overapproximation::overapprox_well_formed_sub(
            protos.clone(),
            &BTreeMap::from([(Role::new("T"), BTreeSet::from([EventType::new("car")]))]),
            Granularity::Coarse,
        );
        assert!(result_subs.is_ok());
        let subs = result_subs.unwrap();

        let role = Role::new("T");
        let (g, i) = proto_info::compose_protocols(protos).unwrap();
        let (proj, proj_initial) = project(&g, i, &subs, role, false);
        let mut proj_machine = util::to_json_machine(proj, proj_initial);
        let mut expected_machine = MachineType {
            initial: State::new("0 || 0"),
            transitions: vec![
                Transition {
                    label: MachineLabel::Execute {
                        cmd: Command::new("request"),
                        log_type: vec![EventType::new("partID")],
                    },
                    source: State::new("0 || 0"),
                    target: State::new("0 || 0"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("partID"),
                    },
                    source: State::new("0 || 0"),
                    target: State::new("1 || 1"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("time"),
                    },
                    source: State::new("0 || 0"),
                    target: State::new("3 || 0"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("pos"),
                    },
                    source: State::new("1 || 1"),
                    target: State::new("2 || 1"),
                },
                Transition {
                    label: MachineLabel::Execute {
                        cmd: Command::new("deliver"),
                        log_type: vec![EventType::new("part")],
                    },
                    source: State::new("2 || 1"),
                    target: State::new("2 || 1"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("part"),
                    },
                    source: State::new("2 || 1"),
                    target: State::new("0 || 2"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("time"),
                    },
                    source: State::new("0 || 2"),
                    target: State::new("3 || 2"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("car"),
                    },
                    source: State::new("3 || 2"),
                    target: State::new("3 || 3"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("car"),
                    },
                    source: State::new("0 || 2"),
                    target: State::new("0 || 3"),
                },
                Transition {
                    label: MachineLabel::Input {
                        event_type: EventType::new("time"),
                    },
                    source: State::new("0 || 3"),
                    target: State::new("3 || 3"),
                },
            ],
        };
        proj_machine.transitions.sort();
        expected_machine.transitions.sort();
        assert_eq!(proj_machine, expected_machine);
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
        let from_option_graph_to_graph = |graph: &OptionGraph| -> Graph {
            graph.map(
                |_, n| n.clone().unwrap_or_else(|| State::new("")),
                |_, x| x.clone(),
            )
        };
        let (left, left_initial, _) = util::from_json(left);
        let left = from_option_graph_to_graph(&left);
        let (right, right_initial, _) = util::from_json(right);
        let right = from_option_graph_to_graph(&right);
        let interface = BTreeSet::from([EventType::new("a"), EventType::new("b")]);
        let (combined, combined_initial) = composition::compose(
            right,
            right_initial.unwrap(),
            left,
            left_initial.unwrap(),
            interface,
            composition::gen_state_name,
        );
        let combined = util::to_json_machine(combined, combined_initial);

        let expected = MachineType {
            initial: State::new("right_0 || left_0"),
            transitions: vec![],
        };

        assert_eq!(combined, expected);
    }
}
