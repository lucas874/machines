use std::collections::{BTreeMap, BTreeSet};

use itertools::Itertools;
use petgraph::{
    graph::EdgeReference,
    visit::{EdgeFiltered, EdgeRef, IntoEdgeReferences, IntoEdgesDirected, IntoNodeReferences},
    Direction::{Incoming, Outgoing},
};

use super::{
    composition_types::EventLabel,
    types::{StateName, Transition},
    EventType, Machine, MachineLabel, NodeId, Role, State, Subscriptions, SwarmLabel,
};

// types more or less copied from machine.rs.
type Graph = petgraph::Graph<State, MachineLabel>;
type OptionGraph = petgraph::Graph<Option<State>, MachineLabel>;
type ERef<'a> = <&'a super::Graph as IntoEdgeReferences>::EdgeRef;

impl From<String> for State {
    fn from(value: String) -> State {
        State::new(&value)
    }
}

// projection as described in Composing Swarm Protocols by Florian Furbach
pub fn project(
    swarm: &super::Graph,
    initial: NodeId,
    subs: &Subscriptions,
    role: Role,
) -> (Graph, NodeId) {
    //  assume each command emits exactly one event type
    //  find all nodes with incoming edges in subscription union {initial}
    //  these are the nodes of the projection
    //  for all nodes in projection:
    //      starting at node find nearest nodes with outgoing edges in sub. starting at node meaning node included in this search.
    //      for each edge at such a node:
    //          if edge describe commands performed by role add an execute self loop
    //          add an inpute edge terrminating where they terminate in protocol.
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
    nfa_to_dfa(machine, m_nodes[initial.index()])
}

// precondition: the protocols interfaces on the supplied interfaces.
// precondition: the composition of the protocols in swarms is wwf w.r.t. subs.
// the type of the input paremeter not nice? reconsider
pub fn project_combine(
    swarms: &Vec<(super::Graph, NodeId, BTreeSet<EventType>)>,
    subs: &Subscriptions,
    role: Role,
) -> (OptionGraph, Option<NodeId>) {
    // check this anyway
    if swarms.is_empty()
        || !swarms[0].2.is_empty()
        || swarms[1..]
            .iter()
            .any(|(_, _, interface)| interface.is_empty())
    {
        return (OptionGraph::new(), None);
    }

    let mapper = |(graph, initial, interface): &(super::Graph, NodeId, BTreeSet<EventType>)| -> (Graph, NodeId, BTreeSet<EventType>) {
        let (projection, projection_initial) = project(&graph, *initial, subs, role.clone());
        (projection, projection_initial, interface.clone())
    };

    let projections: Vec<_> = swarms.into_iter().map(mapper).collect();

    let (acc_machine, acc_initial, _) = projections[0].clone();
    let (combined_projection, combined_initial) = projections[1..].to_vec().into_iter().fold(
        (acc_machine, acc_initial),
        |(acc, acc_i), (m, i, interface)| compose(acc, acc_i, m, i, interface),
    );

    // why option here COME BACK
    (
        to_option_machine(&combined_projection),
        Some(combined_initial),
    )
}

pub fn project_combine_all(
    swarms: &Vec<(super::Graph, NodeId, BTreeSet<EventType>)>,
    subs: &Subscriptions,
) -> Vec<(OptionGraph, Option<NodeId>)> {
    subs.keys()
        .map(|role| project_combine(swarms, subs, role.clone()))
        .collect()
}

// nfa to dfa using subset construction. Hopcroft, Motwani and Ullman section 2.3.5
fn nfa_to_dfa(nfa: Graph, i: NodeId) -> (Graph, NodeId) {
    let mut dfa = Graph::new();
    // maps vectors of NodeIds from the nfa to a NodeId in the new dfa
    let mut dfa_nodes: BTreeMap<Vec<NodeId>, NodeId> = BTreeMap::new();

    // push to and pop from in loop until empty and NFA has been turned into a dfa
    let mut stack: Vec<Vec<NodeId>> = Vec::new();

    // [0, 1, 2] becomes Some(State("{0, 1, 2}"))
    let state_name = |nodes: &Vec<NodeId>| -> State {
        let name = format!("{{ {} }}", nodes.iter().map(|n| nfa[*n].clone()).join(", "));
        State::new(&name)
    };

    // get all outgoing edges of the sources. turn into a map from machine labels to vectors of target states.
    let outgoing_map = |srcs: &Vec<NodeId>| -> BTreeMap<MachineLabel, Vec<NodeId>> {
        srcs.iter()
            .flat_map(|src| {
                nfa.edges_directed(*src, Outgoing)
                    .map(|e| (e.weight().clone(), e.target()))
            })
            .collect::<BTreeSet<(MachineLabel, NodeId)>>()
            .into_iter()
            .fold(BTreeMap::new(), |mut m, (edge_label, target)| {
                m.entry(edge_label)
                    .and_modify(|v: &mut Vec<NodeId>| v.push(target))
                    .or_insert(vec![target]);
                m
            })
    };

    // add initial state to dfa
    dfa_nodes.insert(vec![i], dfa.add_node(state_name(&vec![i])));
    // add initial state to stack
    stack.push(vec![i]);

    while let Some(states) = stack.pop() {
        let map = outgoing_map(&states);

        for edge in map.keys() {
            if !dfa_nodes.contains_key(&map[edge]) {
                stack.push(map[edge].clone());
            }
            let target: NodeId = *dfa_nodes
                .entry(map[edge].clone())
                .or_insert(dfa.add_node(state_name(&map[edge])));
            let src: NodeId = *dfa_nodes.get(&states).unwrap();
            dfa.add_edge(src, target, edge.clone());
        }
    }

    (dfa, dfa_nodes[&vec![i]])
}

// precondition: both machines are projected from wwf protocols?
// precondition: m1 and m2 subscribe to all events in interface? Sort of works without but not really?
// takes type parameters to make it work for machines and protocols.
pub(in crate::composition) fn compose<N: StateName + From<String>, E: EventLabel>(
    m1: petgraph::Graph<N, E>,
    i1: NodeId,
    m2: petgraph::Graph<N, E>,
    i2: NodeId,
    interface: BTreeSet<EventType>,
) -> (petgraph::Graph<N, E>, NodeId) {
    let mut machine = petgraph::Graph::<N, E>::new();
    let mut node_map: BTreeMap<(NodeId, NodeId), NodeId> = BTreeMap::new();

    let gen_state_name = |s1: &N, s2: &N| -> N {
        let name = format!("{} || {}", s1.state_name(), s2.state_name());
        N::from(name)
    };

    let weight_target_mapper = |e: EdgeReference<'_, E>| (e.weight().clone(), e.target());

    let outgoing_map = |m: &petgraph::Graph<N, E>, src: NodeId| -> BTreeMap<E, NodeId> {
        m.edges_directed(src, Outgoing)
            .map(weight_target_mapper)
            .collect()
    };

    // take the outgoing edges of a node an split into two vectors: one for the edges involving interfacing events and one for the edges that do not
    let partitioned = |m: &petgraph::Graph<N, E>, node: NodeId| -> (Vec<E>, Vec<E>) {
        m.edges_directed(node, Outgoing)
            .map(|e| e.weight().clone())
            .partition(|e| interface.contains(&e.get_event_type()))
    };

    let outgoing_to_visit = |m1: &petgraph::Graph<N, E>,
                             s1: NodeId,
                             m2: &petgraph::Graph<N, E>,
                             s2: NodeId|
     -> Vec<E> {
        let (interfacing1, non_interfacing1) = partitioned(m1, s1);
        let (interfacing2, non_interfacing2) = partitioned(m2, s2);

        let interfacing_in_both: Vec<E> = interfacing1
            .iter()
            .cloned()
            .collect::<BTreeSet<E>>()
            .intersection(&interfacing2.iter().cloned().collect::<BTreeSet<E>>())
            .cloned()
            .collect();
        vec![non_interfacing1, non_interfacing2, interfacing_in_both]
            .into_iter()
            .flatten()
            .collect()
    };

    let combined_initial = machine.add_node(gen_state_name(&m1[i1], &m2[i2]));
    node_map.insert((i1, i2), combined_initial);
    let mut worklist = vec![(combined_initial, (i1, i2))];

    while let Some((src, (old_src1, old_src2))) = worklist.pop() {
        let map1 = outgoing_map(&m1, old_src1);
        let map2 = outgoing_map(&m2, old_src2);
        let outgoing_edges = outgoing_to_visit(&m1, old_src1, &m2, old_src2);

        // add all outgoing edges from src node. only visit edges that are not interfacing or interfacing and both outgoing of old_src1 and old_src2
        // if a edge leads to a node that does not exist yet, create the node.
        for e in outgoing_edges {
            let (dst1, dst2) = match (map1.get(&e), map2.get(&e)) {
                (Some(e1), Some(e2)) => (*e1, *e2),
                (Some(e1), None) => (*e1, old_src2),
                (None, Some(e2)) => (old_src1, *e2),
                _ => unimplemented!(),
            };
            if node_map.contains_key(&(dst1, dst2)) {
                let dst = node_map.get(&(dst1, dst2)).unwrap();
                machine.add_edge(src, *dst, e);
            } else {
                let new_dst = machine.add_node(gen_state_name(&m1[dst1], &m2[dst2]));
                machine.add_edge(src, new_dst, e);
                node_map.insert((dst1, dst2), new_dst);
                worklist.push((new_dst, (dst1, dst2)));
            }
        }
    }

    (machine, combined_initial)
}

fn to_option_machine(graph: &Graph) -> OptionGraph {
    graph.map(|_, n| Some(n.state_name().clone()), |_, x| x.clone())
}

pub fn to_json_machine(graph: Graph, initial: NodeId) -> Machine {
    let machine_label_mapper = |m: &Graph, eref: EdgeReference<'_, MachineLabel>| {
        let label = eref.weight().clone();
        let source = m[eref.source()].clone();
        let target = m[eref.target()].clone();
        Transition {
            label,
            source,
            target,
        }
    };

    let transitions: Vec<_> = graph
        .edge_references()
        .map(|e| machine_label_mapper(&graph, e))
        .collect();

    Machine {
        initial: graph[initial].clone(),
        transitions,
    }
}

pub fn from_option_to_machine(
    graph: petgraph::Graph<Option<State>, MachineLabel>,
    initial: NodeId,
) -> Machine {
    let machine_label_mapper =
        |m: &petgraph::Graph<Option<State>, MachineLabel>,
         eref: EdgeReference<'_, MachineLabel>| {
            let label = eref.weight().clone();
            let source = m[eref.source()].clone().unwrap_or(State::from(""));
            let target = m[eref.target()].clone().unwrap_or(State::from(""));
            Transition {
                label,
                source,
                target,
            }
        };

    let transitions: Vec<_> = graph
        .edge_references()
        .map(|e| machine_label_mapper(&graph, e))
        .collect();

    Machine {
        initial: graph[initial].clone().unwrap_or(State::from("")),
        transitions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        composition::{
            self,
            composition_swarm::{from_json, implicit_composition_swarms, weak_well_formed_sub},
            composition_types::{CompositionInput, CompositionInputVec},
        },
        types::{Command, EventType, Role, Transition},
        Machine, Subscriptions, SwarmProtocol,
    };

    // Example from coplaws slides
    fn get_proto1() -> SwarmProtocol {
        serde_json::from_str::<SwarmProtocol>(
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
    fn get_proto2() -> SwarmProtocol {
        serde_json::from_str::<SwarmProtocol>(
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
    fn get_proto3() -> SwarmProtocol {
        serde_json::from_str::<SwarmProtocol>(
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
    fn get_proto1_proto2_composed() -> SwarmProtocol {
        serde_json::from_str::<SwarmProtocol>(
            r#"{
                "initial": "0 || 0",
                "transitions": [
                    { "source": "0 || 0", "target": "1 || 1", "label": { "cmd": "request", "logType": ["partID"], "role": "T" } },
                    { "source": "0 || 0", "target": "3 || 0", "label": { "cmd": "close", "logType": ["time"], "role": "D" } },
                    { "source": "1 || 1", "target": "2 || 1", "label": { "cmd": "get", "logType": ["pos"], "role": "FL" } },
                    { "source": "2 || 1", "target": "0 || 2", "label": { "cmd": "deliver", "logType": ["part"], "role": "T" } },
                    { "source": "0 || 2", "target": "0 || 3", "label": { "cmd": "build", "logType": ["car"], "role": "F" } },
                    { "source": "0 || 2", "target": "3 || 2", "label": { "cmd": "close", "logType": ["time"], "role": "D" } },
                    { "source": "0 || 3", "target": "3 || 3", "label": { "cmd": "close", "logType": ["time"], "role": "D" } },
                    { "source": "3 || 2", "target": "3 || 3", "label": { "cmd": "build", "logType": ["car"], "role": "F" } }
                ]
            }"#,
        )
        .unwrap()
    }

    fn get_composition_input_vec1() -> CompositionInputVec {
        vec![
            CompositionInput {
                protocol: get_proto1(),
                subscription: weak_well_formed_sub(get_proto1()),
                interface: None,
            },
            CompositionInput {
                protocol: get_proto2(),
                subscription: weak_well_formed_sub(get_proto2()),
                interface: Some(Role::new("T")),
            },
            CompositionInput {
                protocol: get_proto3(),
                subscription: weak_well_formed_sub(get_proto3()),
                interface: Some(Role::new("F")),
            },
        ]
    }

    fn get_composition_input_vec2() -> CompositionInputVec {
        vec![
            CompositionInput {
                protocol: get_proto2(),
                subscription: weak_well_formed_sub(get_proto2()),
                interface: None,
            },
            CompositionInput {
                protocol: get_proto1(),
                subscription: weak_well_formed_sub(get_proto1()),
                interface: Some(Role::new("T")),
            },
            CompositionInput {
                protocol: get_proto3(),
                subscription: weak_well_formed_sub(get_proto3()),
                interface: Some(Role::new("F")),
            },
        ]
    }

    #[test]
    fn test_projection_1() {
        // From Combining Swarm Protocols, example 5.
        let proto = serde_json::from_str::<SwarmProtocol>(
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
        let (g, i, _) = from_json(proto, &subs);
        let (proj, proj_initial) = project(&g, i.unwrap(), &subs, role);
        let expected_m = Machine {
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
        // from machine::equivalent(): "error messages are designed assuming that `left` is the reference and `right` the tested"
        assert!(crate::machine::equivalent(
            &expected,
            expected_initial.unwrap(),
            &to_option_machine(&proj),
            proj_initial
        )
        .is_empty());
    }

    #[test]
    fn test_projection_2() {
        // warehouse example from coplaws slides
        let proto = get_proto1();
        let subs = weak_well_formed_sub(proto.clone());
        let role = Role::new("FL");
        let (g, i, _) = from_json(proto, &subs);
        let (proj, proj_initial) = project(&g, i.unwrap(), &subs, role);
        let expected_m = Machine {
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
        let (expected, expected_initial, errors) = crate::machine::from_json(expected_m);
        //println!("computed {:?}: {}", role.clone(), serde_json::to_string_pretty(&to_machine(proj.clone(), proj_initial)).unwrap());
        //println!("expected {:?}: {}", role, serde_json::to_string_pretty(&to_machine(expected.clone(), expected_initial.unwrap())).unwrap());
        assert!(errors.is_empty());
        assert!(expected_initial.is_some());
        // from machine::equivalent(): "error messages are designed assuming that `left` is the reference and `right` the tested"
        assert!(crate::machine::equivalent(
            &expected,
            expected_initial.unwrap(),
            &to_option_machine(&proj),
            proj_initial
        )
        .is_empty());
    }

    #[test]
    fn test_projection_3() {
        // car factory from coplaws example
        let proto = get_proto2();
        let subs = weak_well_formed_sub(proto.clone());
        let role = Role::new("F");
        let (g, i, _) = from_json(proto, &subs);
        let (proj, proj_initial) = project(&g, i.unwrap(), &subs, role);
        let expected_m = Machine {
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
        // from machine::equivalent(): "error messages are designed assuming that `left` is the reference and `right` the tested"
        assert!(crate::machine::equivalent(
            &expected,
            expected_initial.unwrap(),
            &to_option_machine(&proj),
            proj_initial
        )
        .is_empty());
    }

    #[test]
    fn test_combine_machines_1() {
        // Example from coplaws slides. Use generated WWF subscriptions. Project over T.
        let role = Role::new("T");

        let protos = get_composition_input_vec1()[0..2].to_vec();
        let (swarms, subs) = implicit_composition_swarms(protos);
        let swarms = swarms
            .into_iter()
            .map(|((g, i, _), s)| (g, i.unwrap(), s))
            .collect();
        let (proj_combined1, proj_combined_initial1) =
            project_combine(&swarms, &subs, role.clone());

        let protos = get_composition_input_vec2()[0..2].to_vec();
        let (swarms, subs) = implicit_composition_swarms(protos);
        let swarms = swarms
            .into_iter()
            .map(|((g, i, _), s)| (g, i.unwrap(), s))
            .collect();
        let (proj_combined2, proj_combined_initial2) =
            project_combine(&swarms, &subs, role.clone());

        // compose(a, b) should be equal to compose(b, a)
        assert!(crate::machine::equivalent(
            &proj_combined1,
            proj_combined_initial1.unwrap(),
            &proj_combined2,
            proj_combined_initial2.unwrap()
        )
        .is_empty());

        // sub becomes smaller when we analyse the explicitily composed protocol. so for them to be equivalent use the overapproximated sub.
        //println!("SUBS: {}", serde_json::to_string_pretty(&subs).unwrap());
        let proto = get_proto1_proto2_composed();
        // Interesting to generate the WWF sub using the explicit composition and see difference!
        //let subs = weak_well_formed_sub(proto.clone());
        //println!("SUBS: {}", serde_json::to_string_pretty(&subs).unwrap());
        let (g, i, _) = from_json(proto, &subs);
        let (proj, proj_initial) = project(&g, i.unwrap(), &subs, role.clone());
        //println!("EXPECTED: {}", serde_json::to_string_pretty(&to_json_machine(proj.clone(), proj_initial)).unwrap());

        // project(compose(proto1, proto2), r, sub) should equal compose(project(proto1, r, sub), project(proto2, r, sub))
        assert!(crate::machine::equivalent(
            &to_option_machine(&proj),
            proj_initial,
            &proj_combined1,
            proj_combined_initial1.unwrap()
        )
        .is_empty());
    }

    #[test]
    fn test_combine_machines_2() {
        // Example from coplaws slides. Use generated WWF subscriptions. Project over T.
        let role = Role::new("FL");

        let protos = get_composition_input_vec1()[0..2].to_vec();
        let (swarms, subs) = implicit_composition_swarms(protos);
        let swarms = swarms
            .into_iter()
            .map(|((g, i, _), s)| (g, i.unwrap(), s))
            .collect();
        let (proj_combined1, proj_combined_initial1) =
            project_combine(&swarms, &subs, role.clone());

        let protos = get_composition_input_vec2()[0..2].to_vec();
        let (swarms, subs) = implicit_composition_swarms(protos);
        let swarms = swarms
            .into_iter()
            .map(|((g, i, _), s)| (g, i.unwrap(), s))
            .collect();
        let (proj_combined2, proj_combined_initial2) =
            project_combine(&swarms, &subs, role.clone());

        // compose(a, b) should be equal to compose(b, a)
        assert!(crate::machine::equivalent(
            &proj_combined1,
            proj_combined_initial1.unwrap(),
            &proj_combined2,
            proj_combined_initial2.unwrap()
        )
        .is_empty());

        let proto = get_proto1_proto2_composed();
        //let subs = weak_well_formed_sub(proto.clone());
        let (g, i, _) = from_json(proto, &subs);
        let (proj, proj_initial) = project(&g, i.unwrap(), &subs, role.clone());

        // project(compose(proto1, proto2), r, sub) should equal compose(project(proto1, r, sub), project(proto2, r, sub))
        assert!(crate::machine::equivalent(
            &to_option_machine(&proj),
            proj_initial,
            &proj_combined1,
            proj_combined_initial1.unwrap()
        )
        .is_empty());
    }

    #[test]
    fn test_combine_machines_3() {
        // Example from coplaws slides. Use generated WWF subscriptions. Project over T.
        let role = Role::new("D");

        let protos = get_composition_input_vec1()[0..2].to_vec();
        let (swarms, subs) = implicit_composition_swarms(protos);
        let swarms = swarms
            .into_iter()
            .map(|((g, i, _), s)| (g, i.unwrap(), s))
            .collect();
        let (proj_combined1, proj_combined_initial1) =
            project_combine(&swarms, &subs, role.clone());

        let protos = get_composition_input_vec2()[0..2].to_vec();
        let (swarms, subs) = implicit_composition_swarms(protos);
        let swarms = swarms
            .into_iter()
            .map(|((g, i, _), s)| (g, i.unwrap(), s))
            .collect();
        let (proj_combined2, proj_combined_initial2) =
            project_combine(&swarms, &subs, role.clone());

        // compose(a, b) should be equal to compose(b, a)
        assert!(crate::machine::equivalent(
            &proj_combined1,
            proj_combined_initial1.unwrap(),
            &proj_combined2,
            proj_combined_initial2.unwrap()
        )
        .is_empty());

        let proto = get_proto1_proto2_composed();
        //let subs = weak_well_formed_sub(proto.clone());
        let (g, i, _) = from_json(proto, &subs);
        let (proj, proj_initial) = project(&g, i.unwrap(), &subs, role.clone());

        // project(compose(proto1, proto2), r, sub) should equal compose(project(proto1, r, sub), project(proto2, r, sub))
        assert!(crate::machine::equivalent(
            &to_option_machine(&proj),
            proj_initial,
            &proj_combined1,
            proj_combined_initial1.unwrap()
        )
        .is_empty());
    }

    #[test]
    fn test_combine_machines_4() {
        // Example from coplaws slides. Use generated WWF subscriptions. Project over T.
        let role = Role::new("F");

        let protos = get_composition_input_vec1()[0..2].to_vec();
        let (swarms, subs) = implicit_composition_swarms(protos);
        let swarms = swarms
            .into_iter()
            .map(|((g, i, _), s)| (g, i.unwrap(), s))
            .collect();
        let (proj_combined1, proj_combined_initial1) =
            project_combine(&swarms, &subs, role.clone());

        let protos = get_composition_input_vec2()[0..2].to_vec();
        let (swarms, subs) = implicit_composition_swarms(protos);
        let swarms = swarms
            .into_iter()
            .map(|((g, i, _), s)| (g, i.unwrap(), s))
            .collect();
        let (proj_combined2, proj_combined_initial2) =
            project_combine(&swarms, &subs, role.clone());

        // compose(a, b) should be equal to compose(b, a)
        assert!(crate::machine::equivalent(
            &proj_combined1,
            proj_combined_initial1.unwrap(),
            &proj_combined2,
            proj_combined_initial2.unwrap()
        )
        .is_empty());

        let proto = get_proto1_proto2_composed();
        //let subs = weak_well_formed_sub(proto.clone());
        let (g, i, _) = from_json(proto, &subs);
        let (proj, proj_initial) = project(&g, i.unwrap(), &subs, role.clone());

        // project(compose(proto1, proto2), r, sub) should equal compose(project(proto1, r, sub), project(proto2, r, sub))
        assert!(crate::machine::equivalent(
            &to_option_machine(&proj),
            proj_initial,
            &proj_combined1,
            proj_combined_initial1.unwrap()
        )
        .is_empty());
    }

    #[test]
    fn test_combine_machines_5() {
        // Example from coplaws slides. Use generated WWF subscriptions. Project over T.
        let role = Role::new("QCR");

        let protos = get_composition_input_vec1();
        let (swarms, subs) = implicit_composition_swarms(protos);
        let swarms = swarms
            .into_iter()
            .map(|((g, i, _), s)| (g, i.unwrap(), s))
            .collect();
        let (proj_combined1, proj_combined_initial1) =
            project_combine(&swarms, &subs, role.clone());

        let protos = get_composition_input_vec2().to_vec();
        let (swarms, subs) = implicit_composition_swarms(protos);
        let swarms = swarms
            .into_iter()
            .map(|((g, i, _), s)| (g, i.unwrap(), s))
            .collect();
        let (proj_combined2, proj_combined_initial2) =
            project_combine(&swarms, &subs, role.clone());

        // compose(a, b) should be equal to compose(b, a)
        assert!(crate::machine::equivalent(
            &proj_combined1,
            proj_combined_initial1.unwrap(),
            &proj_combined2,
            proj_combined_initial2.unwrap()
        )
        .is_empty());

        let (g, i) =
            composition::composition_swarm::compose_protocols(get_composition_input_vec1())
                .unwrap();
        let (proj, proj_initial) = project(&g, i, &subs, role.clone());

        // project(compose(proto1, proto2), r, sub) should equal compose(project(proto1, r, sub), project(proto2, r, sub))
        assert!(crate::machine::equivalent(
            &to_option_machine(&proj),
            proj_initial,
            &proj_combined1,
            proj_combined_initial1.unwrap()
        )
        .is_empty());
    }
}
