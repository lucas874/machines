use machine_check::{
    composition::{check_composed_projection, check_wwf_swarm, compose_protocols, composition_types::{CompositionComponent, DataResult, Granularity, InterfacingSwarms}, exact_weak_well_formed_sub, overapproximated_weak_well_formed_sub, project_combine, revised_projection}, types::{Command, EventType, Role, State, StateName, SwarmLabel, Transition}, EdgeId, Graph, Machine, NodeId, Subscriptions, SwarmProtocol
};
use petgraph::{
    graph::EdgeReference,
    visit::{Dfs, EdgeRef, Reversed, Walker},
    Direction::{Incoming, Outgoing},
};
use proptest::prelude::*;
use rand::{distributions::Bernoulli, prelude::*};
use serde::{Deserialize, Serialize};
use std::{
    cmp,
    collections::{BTreeMap, BTreeSet},
    iter::zip,
    sync::Mutex,
    path::Path,
    fs::{File, create_dir},
    io::prelude::*,
};

use walkdir::WalkDir;

// reimplemented here because we need to Deserialize. To not change in types.rs
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum CheckResult {
    OK,
    ERROR { errors: Vec<String> },
}

static FILE_COUNTER_MAX: Mutex<u32> = Mutex::new(0);

// for uniquely named roles. not strictly necessary? but nice. little ugly idk
static ROLE_COUNTER_MUTEX: Mutex<u32> = Mutex::new(0);
fn fresh_i() -> u32 {
    let mut mut_guard = ROLE_COUNTER_MUTEX.lock().unwrap();
    let i: u32 = *mut_guard;
    *mut_guard += 1;
    i
}

static R_BASE: &str = "R";
static IR_BASE: &str = "IR";
static CMD_BASE: &str = "cmd";
static E_BASE: &str = "e";

prop_compose! {
    fn vec_swarm_label(role: Role, max_events: usize)(vec in prop::collection::vec((CMD_BASE, E_BASE), 1..max_events)) -> Vec<SwarmLabel> {
        vec.into_iter()
        .enumerate()
        .map(|(i, (cmd, event))|
            SwarmLabel { cmd: Command::new(&format!("{role}_{cmd}_{i}")), log_type: vec![EventType::new(&format!("{role}_{event}_{i}"))], role: role.clone()})
        .collect()
    }
}

prop_compose! {
    fn vec_role(max_roles: usize)(vec in prop::collection::vec(R_BASE, 1..max_roles)) -> Vec<Role> {
        vec
        .into_iter()
        .map(|role| {
            let i = fresh_i();
            Role::new(&format!("{role}{i}"))
        }).collect()
    }
}

prop_compose! {
    fn all_labels(max_roles: usize, max_events: usize)
                (roles in vec_role(max_roles))
                (labels in roles.into_iter().map(|role| vec_swarm_label(role, max_events)).collect::<Vec<_>>()) -> Vec<SwarmLabel> {
        labels.concat()
    }
}

prop_compose! {
    fn all_labels_1(roles: Vec<Role>, max_events: usize)
                (labels in roles.into_iter().map(|role| vec_swarm_label(role, max_events)).collect::<Vec<_>>()) -> Vec<Vec<SwarmLabel>> {
        labels
    }
}

prop_compose! {
    fn all_labels_2(roles: Vec<Role>, max_roles: usize, max_events: usize)
                ((labels, ir_labels) in (prop::collection::vec(all_labels(max_roles, max_events), roles.len()), roles.into_iter().map(|role| vec_swarm_label(role, max_events)).collect::<Vec<_>>()))
                -> Vec<(Vec<SwarmLabel>, Vec<SwarmLabel>)> {
        zip(ir_labels, labels).collect()
    }
}

prop_compose! {
    fn all_labels_and_if(max_roles: usize, max_events: usize)
            (roles in vec_role(max_roles))
            (index in 0..roles.len(), labels in roles.into_iter().map(|role| vec_swarm_label(role, max_events)).collect::<Vec<_>>())
            -> (Vec<SwarmLabel>, Vec<SwarmLabel>) {
        let interfacing = labels[index].clone();
        (labels.concat(), interfacing)
    }
}
prop_compose! {
    fn all_labels_composition(max_roles: usize, max_events: usize, max_protos: usize, exactly_max: bool)
            (tuples in prop::collection::vec(all_labels_and_if(max_roles, max_events), if exactly_max {max_protos..=max_protos} else {1..=max_protos}))
            -> Vec<(Option<Role>, Vec<SwarmLabel>)> {
        let (labels, interfaces): (Vec<_>, Vec<_>) = tuples.into_iter().unzip();
        let tmp: Vec<(Option<Role>, Vec<SwarmLabel>)>  = interfaces[..interfaces.len()].to_vec().into_iter().map(|interface| (Some(interface[0].role.clone()), interface)).collect();
        let interfaces: Vec<(Option<Role>, Vec<SwarmLabel>)> = vec![vec![(None, vec![])], tmp].concat();
        labels.into_iter().zip(interfaces.into_iter()).map(|(labels, (interface, interfacing_cmds))| (interface, vec![labels, interfacing_cmds].concat())).collect()
    }
}

// shuffle labels before calling, then call random graph
fn random_graph_shuffle_labels(
    base_graph: Option<(Graph, NodeId)>,
    mut swarm_labels: Vec<SwarmLabel>,
) -> (Graph, NodeId) {
    let mut rng = rand::thread_rng();
    swarm_labels.shuffle(&mut rng);
    random_graph(base_graph, swarm_labels)
}

// add option (graph, nodeid) argument and build on top of this graph if some
// if base_graph is some, add nodes and edges to this graph. otherwise create from scratch.
fn random_graph(
    base_graph: Option<(Graph, NodeId)>,
    mut swarm_labels: Vec<SwarmLabel>,
) -> (Graph, NodeId) {
    let (mut graph, initial, mut nodes) = if base_graph.is_some() {
        let (base, base_initial) = base_graph.unwrap();
        let nodes: Vec<NodeId> = base.node_indices().into_iter().collect();
        (base, base_initial, nodes)
    } else {
        let mut graph = Graph::new();
        let initial = graph.add_node(State::new(&fresh_i().to_string()));
        let nodes = vec![initial];
        (graph, initial, nodes)
    };
    let mut rng = rand::thread_rng();
    let b_dist = Bernoulli::new(0.2).unwrap(); // bernoulli distribution with propability 0.2 of success
    let gen_state_name = || -> State { State::new(&fresh_i().to_string()) };

    while let Some(label) = swarm_labels.pop() {
        // consider bernoulli thing. and distrbutions etc. bc documentations says that these once are optimised for cases where only a single sample is needed... if just faster does not matter
        // generate new or select old source? Generate new or select old, generate new target or select old?
        // same because you would have to connect to graph at some point anyway...?
        // exclusive range upper limit
        let source_node = if b_dist.sample(&mut rng) {
            nodes[rng.gen_range(0..nodes.len())]
        } else {
            // this whole thing was to have fewer branches... idk. loop will terminate because we always can reach 0?
            let mut source = nodes[rng.gen_range(0..nodes.len())];
            while graph.edges_directed(source, Outgoing).count() > 0 {
                source = nodes[rng.gen_range(0..nodes.len())];
            }

            source
        };

        // if generated bool then select an existing node as target
        // otherwise generate a new node as target
        if b_dist.sample(&mut rng) && swarm_labels.len() > 0 {
            let index = rng.gen_range(0..nodes.len());
            let target_node = nodes[index];
            //nodes.push(graph.add_node(State::new(&graph.node_count().to_string())));
            graph.add_edge(source_node, target_node, label);
            // we should be able to reach a terminating node from all nodes.
            // we check that swarm_labels is not empty before entering this branch
            // so we should be able to generate new node and add and edge from
            // target node to this new node
            if !node_can_reach_zero(&graph, target_node) {
                let new_target_node = graph.add_node(gen_state_name());
                // consider not pushing?
                nodes.push(new_target_node);
                let new_weight = swarm_labels.pop().unwrap();
                graph.add_edge(target_node, new_target_node, new_weight);
            }
        } else {
            let target_node = graph.add_node(gen_state_name());
            nodes.push(target_node);
            graph.add_edge(source_node, target_node, label);
        }
    }

    (graph, initial)
}

fn node_can_reach_zero<N, E>(graph: &petgraph::Graph<N, E>, node: NodeId) -> bool {
    for n in Dfs::new(&graph, node).iter(&graph) {
        if graph.edges_directed(n, Outgoing).count() == 0 {
            return true;
        }
    }
    false
}

pub fn to_swarm_json(graph: crate::Graph, initial: NodeId) -> SwarmProtocol {
    let machine_label_mapper = |g: &crate::Graph, eref: EdgeReference<'_, SwarmLabel>| {
        let label = eref.weight().clone();
        let source = g[eref.source()].state_name().clone();
        let target = g[eref.target()].state_name().clone();
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

    SwarmProtocol {
        initial: graph[initial].state_name().clone(),
        transitions,
    }
}
// generate a number of protocols that interface. interfacing events may appear in different orderes in the protocols
// and may be scattered across different branches: we may 'lose' a lot of behavior.
prop_compose! {
    fn generate_interfacing_swarms(max_roles: usize, max_events: usize, max_protos: usize, exactly_max: bool)
                      (vec in all_labels_composition(max_roles, max_events, max_protos, exactly_max))
                      -> InterfacingSwarms<Role> {
        InterfacingSwarms(vec.into_iter()
            .map(|(interface, swarm_labels)| (random_graph_shuffle_labels(None, swarm_labels), interface))
            .map(|((graph, initial), interface)| {
                let protocol = to_swarm_json(graph, initial);
                CompositionComponent { protocol, interface }
                }
            ).collect())

    }
}

// generate a number of protocols that interface and where protocol i 'refines' protocol i+1
prop_compose! {
    fn generate_interfacing_swarms_refinement(max_roles: usize, max_events: usize, num_protos: usize)
                      (vec in prop::collection::vec(all_labels(max_roles, max_events), cmp::max(0, num_protos-1)))
                      -> InterfacingSwarms<Role> {
        let level_0_proto = refinement_initial_proto();
        let mut graphs = vec![CompositionComponent {protocol: to_swarm_json(level_0_proto.0, level_0_proto.1), interface: None}];
        let mut vec = vec
            .into_iter()
            .map(|swarm_labels| random_graph_shuffle_labels(None, swarm_labels))
            .enumerate()
            .map(|(level, (proto, initial))| (level, refinement_shape(level, proto, initial)))
            .map(|(level, (proto, initial))|
                    CompositionComponent { protocol: to_swarm_json(proto, initial), interface: Some(Role::new(&format!("{IR_BASE}_{level}")))}
                )
            .collect();
        graphs.append(&mut vec);

        InterfacingSwarms(graphs)
    }
}

prop_compose! {
    fn protos_refinement_2(max_events: usize, num_protos: usize)
                (labels in all_labels_1((0..num_protos).into_iter().map(|i| Role::new(&format!("{IR_BASE}_{i}"))).collect(), max_events))
                -> Vec<(Graph, NodeId)> {
        labels.into_iter().map(|labels| random_graph(None, labels)).collect()
    }
}

prop_compose! {
    fn protos_refinement_22(max_roles: usize, max_events: usize, num_protos: usize)
                (labels in all_labels_2((0..num_protos).into_iter().map(|i| Role::new(&format!("{IR_BASE}_{i}"))).collect(), cmp::max(0, max_roles-1), max_events))
                -> Vec<((Graph, NodeId), Vec<SwarmLabel>)> {
        labels.into_iter().map(|(ir_labels, labels)| (random_graph(None, ir_labels.into_iter().rev().collect()), labels)).collect()
    }
}

prop_compose! {
    fn generate_interfacing_swarms_refinement_2(max_roles: usize, max_events: usize, num_protos: usize)
                (protos in protos_refinement_22(max_roles, max_events, num_protos))
                -> InterfacingSwarms<Role> {
        let mut rng = rand::thread_rng();
        let protos_altered: Vec<_> = protos.clone()
            .into_iter()
            .enumerate()
            .map(|(i, ((graph, initial), mut labels))| {
                let (graph, initial) = if i == 0 {
                    (graph, initial)
                } else {
                    // create a graph by inserting protos[i] into protos[i-1]
                    insert_into(protos[i-1].0.clone(), (graph, initial))
                };
                //(graph, initial)
                labels.shuffle(&mut rng);
                expand_graph(graph, initial, labels)
            }).collect();

        InterfacingSwarms(protos_altered.into_iter()
            .enumerate()
            .map(|(i, (graph, initial))|
                CompositionComponent { protocol: to_swarm_json(graph, initial), interface: if i == 0 { None } else { Some(Role::new(&format!("{IR_BASE}_{level}", level=i-1))) } })
            .collect())
    }

}

fn refinement_initial_proto() -> (Graph, NodeId) {
    let mut graph = Graph::new();
    let initial = graph.add_node(State::new(&fresh_i().to_string()));
    let middle = graph.add_node(State::new(&fresh_i().to_string()));
    let last = graph.add_node(State::new(&fresh_i().to_string()));

    let start_label = SwarmLabel {
        cmd: Command::new(&format!("{IR_BASE}_0_{CMD_BASE}_0")),
        log_type: vec![EventType::new(&format!("{IR_BASE}_0_{E_BASE}_0"))],
        role: Role::new(&format!("{IR_BASE}_0")),
    };
    let end_label = SwarmLabel {
        cmd: Command::new(&format!("{IR_BASE}_0_{CMD_BASE}_1")),
        log_type: vec![EventType::new(&format!("{IR_BASE}_0_{E_BASE}_1"))],
        role: Role::new(&format!("{IR_BASE}_0")),
    };

    graph.add_edge(initial, middle, start_label);
    graph.add_edge(middle, last, end_label);

    (graph, initial)
}

// consider a version where we change existing labels instead of adding new edges. still adding new edges for if, but not next if.
fn refinement_shape(level: usize, mut proto: Graph, initial: NodeId) -> (Graph, NodeId) {
    let terminal_nodes: Vec<_> = proto
        .node_indices()
        .filter(|node| proto.edges_directed(*node, Outgoing).count() == 0)
        .collect();
    let mut rng = rand::thread_rng();
    let index = terminal_nodes[rng.gen_range(0..terminal_nodes.len())];
    let reversed_graph = Reversed(&proto);
    let mut dfs = Dfs::new(&reversed_graph, index);
    let mut nodes_on_path = Vec::new();
    while let Some(node) = dfs.next(&reversed_graph) {
        nodes_on_path.push(node);
        if node == initial {
            break;
        }
    }
    // reverse so that index 0 is the initial node and index len-1 is the terminal node on the path
    nodes_on_path.reverse();

    let next_ir = format!("{IR_BASE}_{next_level}", next_level = level + 1);
    let next_if_label_0 = SwarmLabel {
        cmd: Command::new(&format!("{next_ir}_{CMD_BASE}_0")),
        log_type: vec![EventType::new(&format!("{next_ir}_{E_BASE}_0"))],
        role: Role::new(&next_ir),
    };
    let next_if_label_1 = SwarmLabel {
        cmd: Command::new(&format!("{next_ir}_{CMD_BASE}_1")),
        log_type: vec![EventType::new(&format!("{next_ir}_{E_BASE}_1"))],
        role: Role::new(&next_ir),
    };

    let index = rng.gen_range(0..nodes_on_path.len());
    let source_node = nodes_on_path[index];

    if index == nodes_on_path.len() - 1 {
        let next_if_middle = proto.add_node(State::new(&fresh_i().to_string()));
        let next_if_end = proto.add_node(State::new(&fresh_i().to_string()));
        proto.add_edge(source_node, next_if_middle, next_if_label_0);
        proto.add_edge(next_if_middle, next_if_end, next_if_label_1);
        nodes_on_path.push(next_if_middle);
        nodes_on_path.push(next_if_end);
    } else {
        let target_node = nodes_on_path[index + 1];
        let edge_to_remove = proto.find_edge(source_node, target_node).unwrap();
        let weight = proto[edge_to_remove].clone();
        proto.remove_edge(edge_to_remove);
        let next_if_start = proto.add_node(State::new(&fresh_i().to_string()));
        proto.add_edge(source_node, next_if_start, weight);
        let next_if_middle = proto.add_node(State::new(&fresh_i().to_string()));
        proto.add_edge(next_if_start, next_if_middle, next_if_label_0);
        proto.add_edge(next_if_middle, target_node, next_if_label_1);
        nodes_on_path = vec![
            nodes_on_path[..index + 1].to_vec(),
            vec![next_if_start, next_if_middle],
            nodes_on_path[index + 1..].to_vec(),
        ]
        .concat();
    };

    let ir = format!("{IR_BASE}_{level}");
    let if_label_0 = SwarmLabel {
        cmd: Command::new(&format!("{ir}_{CMD_BASE}_0")),
        log_type: vec![EventType::new(&format!("{ir}_{E_BASE}_0"))],
        role: Role::new(&ir),
    };
    let if_label_1 = SwarmLabel {
        cmd: Command::new(&format!("{ir}_{CMD_BASE}_1")),
        log_type: vec![EventType::new(&format!("{ir}_{E_BASE}_1"))],
        role: Role::new(&ir),
    };

    let new_initial = proto.add_node(State::new(&fresh_i().to_string()));
    let new_end = proto.add_node(State::new(&fresh_i().to_string()));
    proto.add_edge(new_initial, initial, if_label_0);
    proto.add_edge(nodes_on_path[nodes_on_path.len() - 1], new_end, if_label_1);

    (proto, new_initial)
}

// insert graph2 into graph1. that is, find some edge e in graph1.
// make e terminate at the initial node of graph2.
// insert all the edges outgoing from the node where e was incoming in the old graph
// as outgoing edges of some node in graph2.
// assume graph1 and graph2 have terminal nodes. Assume they both have at least one edge.
fn insert_into(graph1: (Graph, NodeId), graph2: (Graph, NodeId)) -> (Graph, NodeId) {
    let mut rng = rand::thread_rng();
    let (mut graph1, initial1) = graph1;
    let (graph2, initial2) = graph2;
    // map nodes in graph2 to nodes in graph1
    let mut node_map: BTreeMap<NodeId, NodeId> = BTreeMap::new();
    let mut graph2_terminals: Vec<NodeId> = vec![];

    // edge that we attach to initial of graph2 instead of its old target
    let connecting_edge = graph1.edge_references().choose(&mut rng).unwrap();
    let connecting_source = connecting_edge.source();
    let connecting_old_target = connecting_edge.target();
    let connecting_weight = connecting_edge.weight().clone();
    graph1.remove_edge(connecting_edge.id());

    // create a node in graph1 corresponding to initial of graph2. use insert_with to avoid https://stackoverflow.com/questions/60109843/entryor-insert-executes-despite-a-value-already-existing
    let inserted_initial = node_map
        .entry(initial2)
        .or_insert_with(|| graph1.add_node(State::new(&fresh_i().to_string())));
    graph1.add_edge(connecting_source, *inserted_initial, connecting_weight);

    let mut dfs = Dfs::new(&graph2, initial2);
    while let Some(node) = dfs.next(&graph2) {
        let node_in_graph1 = *node_map
            .entry(node)
            .or_insert_with(|| graph1.add_node(State::new(&fresh_i().to_string())));
        for e in graph2.edges_directed(node, Outgoing) {
            let target_in_graph1 = *node_map
                .entry(e.target())
                .or_insert_with(|| graph1.add_node(State::new(&fresh_i().to_string())));
            graph1.add_edge(node_in_graph1, target_in_graph1, e.weight().clone());
        }

        if graph2.edges_directed(node, Outgoing).count() == 0 {
            graph2_terminals.push(node);
        }
    }

    // select a terminal node in graph2. make all incoming point to connecting old target instead. remove this terminal node.
    let graph2_terminal = *graph2_terminals.choose(&mut rng).unwrap();
    let mut edges_to_remove: Vec<EdgeId> = vec![];
    let mut edges_to_add: Vec<(NodeId, NodeId, SwarmLabel)> = vec![];
    for e in graph1.edges_directed(node_map[&graph2_terminal], Incoming) {
        let source = e.source();
        let weight = e.weight();
        edges_to_remove.push(e.id());
        edges_to_add.push((source, connecting_old_target, weight.clone()));
    }
    for e_id in edges_to_remove {
        graph1.remove_edge(e_id);
    }
    for (source, target, weight) in edges_to_add {
        graph1.add_edge(source, target, weight);
    }
    graph1.remove_node(node_map[&graph2_terminal]);

    (graph1, initial1)
}

fn expand_graph(
    mut graph: Graph,
    initial: NodeId,
    mut swarm_labels: Vec<SwarmLabel>,
) -> (Graph, NodeId) {
    let mut nodes: Vec<NodeId> = graph.node_indices().into_iter().collect();
    let mut rng = rand::thread_rng();
    let b_dist = Bernoulli::new(0.2).unwrap(); // bernoulli distribution with propability 0.1 of success
    let b_dist_2 = Bernoulli::new(0.5).unwrap(); // bernoulli distribution with propability 0.5 of success
    let gen_state_name = || -> State { State::new(&fresh_i().to_string()) };

    while let Some(label) = swarm_labels.pop() {
        if b_dist_2.sample(&mut rng) {
            let source_node = if b_dist.sample(&mut rng) {
                nodes[rng.gen_range(0..nodes.len())]
            } else {
                // this whole thing was to have fewer branches... idk. loop will terminate because we always can reach 0?
                let mut source = nodes[rng.gen_range(0..nodes.len())];
                while graph.edges_directed(source, Outgoing).count() > 0 {
                    source = nodes[rng.gen_range(0..nodes.len())];
                }

                source
            };

            // if generated bool then select an existing node as target
            // otherwise generate a new node as target
            if b_dist.sample(&mut rng) && swarm_labels.len() > 0 {
                let index = rng.gen_range(0..nodes.len());
                let target_node = nodes[index];
                //nodes.push(graph.add_node(State::new(&graph.node_count().to_string())));
                graph.add_edge(source_node, target_node, label);
                // we should be able to reach a terminating node from all nodes.
                // we check that swarm_labels is not empty before entering this branch
                // so we should be able to generate new node and add and edge from
                // target node to this new node
                if !node_can_reach_zero(&graph, target_node) {
                    let new_target_node = graph.add_node(gen_state_name());
                    // consider not pushing?
                    nodes.push(new_target_node);
                    let new_weight = swarm_labels.pop().unwrap();
                    graph.add_edge(target_node, new_target_node, new_weight);
                }
            } else {
                let target_node = graph.add_node(gen_state_name());
                nodes.push(target_node);
                graph.add_edge(source_node, target_node, label);
            }
        } else {
            let connecting_edge = graph.edge_references().choose(&mut rng).unwrap();
            let connecting_source = connecting_edge.source();
            let connecting_old_target = connecting_edge.target();
            let connecting_weight = connecting_edge.weight().clone();
            graph.remove_edge(connecting_edge.id());

            let new_node = graph.add_node(gen_state_name());
            graph.add_edge(connecting_source, new_node, connecting_weight);
            graph.add_edge(new_node, connecting_old_target, label);
        }
    }

    (graph, initial)
}

/* // true if subs1 is a subset of subs2
fn is_sub_subscription(subs1: Subscriptions, subs2: Subscriptions) -> bool {
    if !subs1
        .keys()
        .cloned()
        .collect::<BTreeSet<Role>>()
        .is_subset(&subs2.keys().cloned().collect::<BTreeSet<Role>>())
    {
        return false;
    }

    for role in subs1.keys() {
        //println!("explicit size: {} implicit size: {}", subs1[role].len(), subs2[role].len());
        if !subs1[role].is_subset(&subs2[role]) {
            return false;
        }
    }

    true
} */

// test that we do not generate duplicate labels
proptest! {
    #[test]
    fn test_all_labels(mut labels in all_labels(10, 10)) {
        labels.sort();
        let mut labels2 = labels.clone().into_iter().collect::<BTreeSet<SwarmLabel>>().into_iter().collect::<Vec<_>>();
        labels2.sort();
        assert_eq!(labels, labels2);
    }
}

proptest! {
    #[test]
    fn test_labels_and_interface((labels, interfacing) in all_labels_and_if(10, 10)) {
        let interfacing_set = interfacing.clone().into_iter().collect::<BTreeSet<_>>();
        let labels_set = labels.into_iter().collect::<BTreeSet<_>>();
        assert!(interfacing_set.is_subset(&labels_set));
        let first = interfacing[0].clone();
        assert!(interfacing[1..].into_iter().all(|label| first.role == label.role));
    }
}

// test whether the approximated subscription for compositions
// is contained within the 'exact' subscription.
// i.e. is the approximation safe. max five protocols, max five roles
// in each, max five commands per role. relatively small.
proptest! {
    #[test]
    fn test_exact_1(vec in generate_interfacing_swarms(5, 5, 5, false)) {
        let protos = serde_json::to_string(&vec).unwrap();
        let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet::<EventType>>::new()).unwrap();
        let subscription = match serde_json::from_str(&exact_weak_well_formed_sub(protos.clone(), subs)).unwrap() {
            DataResult::<Subscriptions>::OK{data: subscriptions} => Some(subscriptions),
            DataResult::<Subscriptions>::ERROR{ .. } => None,
        };
        assert!(subscription.is_some());
        let subscription = subscription.unwrap();
        let subscription = serde_json::to_string(&subscription).unwrap();
        let errors = check_wwf_swarm(protos.clone(), subscription.clone());
        let errors = serde_json::from_str::<CheckResult>(&errors).unwrap();
        let ok = match errors {
            CheckResult::OK => true,
            CheckResult::ERROR { .. } => false

        };
        assert!(ok);
    }
}

// test whether the approximated subscription for compositions
// is contained within the 'exact' subscription.
// i.e. is the approximation safe. max five protocols, max five roles
// in each, max five commands per role. relatively small.
proptest! {
    #[test]
    fn test_overapproximated_1(vec in generate_interfacing_swarms(5, 5, 5, false)) {
        let protos = serde_json::to_string(&vec).unwrap();
        let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet::<EventType>>::new()).unwrap();
        let granularity = serde_json::to_string(&Granularity::Coarse).unwrap();
        let subscription = match serde_json::from_str(&overapproximated_weak_well_formed_sub(protos.clone(), subs, granularity)).unwrap() {
            DataResult::<Subscriptions>::OK{data: subscriptions} => Some(subscriptions),
            DataResult::<Subscriptions>::ERROR{ .. } => None,
        };
        assert!(subscription.is_some());
        let subscription = subscription.unwrap();
        let subscription = serde_json::to_string(&subscription).unwrap();
        let errors = check_wwf_swarm(protos.clone(), subscription.clone());
        let errors = serde_json::from_str::<CheckResult>(&errors).unwrap();
        let ok = match errors {
            CheckResult::OK => true,
            CheckResult::ERROR { errors: e } => {println!("{:?}", e); false}

        };
        assert!(ok);
    }
}

// same tests as above but with refinement pattern 1
proptest! {
    #[test]
    fn test_exact_2(vec in generate_interfacing_swarms_refinement(5, 5, 5)) {
        let protos = serde_json::to_string(&vec).unwrap();
        let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet::<EventType>>::new()).unwrap();
        let subscription = match serde_json::from_str(&exact_weak_well_formed_sub(protos.clone(), subs)).unwrap() {
            DataResult::<Subscriptions>::OK{data: subscriptions} => Some(subscriptions),
            DataResult::<Subscriptions>::ERROR{ .. } => None,
        };
        assert!(subscription.is_some());
        let subscription = subscription.unwrap();
        let subscription = serde_json::to_string(&subscription).unwrap();
        let errors = check_wwf_swarm(protos.clone(), subscription.clone());
        let errors = serde_json::from_str::<CheckResult>(&errors).unwrap();
        let ok = match errors {
            CheckResult::OK => true,
            CheckResult::ERROR { .. } => false

        };
        assert!(ok);
    }
}

proptest! {
    #[test]
    fn test_overapproximated_2(vec in generate_interfacing_swarms_refinement(5, 5, 5)) {
        let protos = serde_json::to_string(&vec).unwrap();
        let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet::<EventType>>::new()).unwrap();
        let granularity = serde_json::to_string(&Granularity::Coarse).unwrap();
        let subscription = match serde_json::from_str(&overapproximated_weak_well_formed_sub(protos.clone(), subs, granularity)).unwrap() {
            DataResult::<Subscriptions>::OK{data: subscriptions} => Some(subscriptions),
            DataResult::<Subscriptions>::ERROR{ .. } => None,
        };
        assert!(subscription.is_some());
        let subscription = subscription.unwrap();
        let subscription = serde_json::to_string(&subscription).unwrap();
        let errors = check_wwf_swarm(protos.clone(), subscription.clone());
        let errors = serde_json::from_str::<CheckResult>(&errors).unwrap();
        let ok = match errors {
            CheckResult::OK => true,
            CheckResult::ERROR { .. } => false

        };
        assert!(ok);
    }
}

// same tests as above but with refinement pattern 2 fewer protocols to not have to wait so long
proptest! {
    #[test]
    fn test_exact_3(vec in generate_interfacing_swarms_refinement_2(5, 5, 3)) {
        let protos = serde_json::to_string(&vec).unwrap();
        let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet::<EventType>>::new()).unwrap();
        let subscription = match serde_json::from_str(&exact_weak_well_formed_sub(protos.clone(), subs)).unwrap() {
            DataResult::<Subscriptions>::OK{data: subscriptions} => Some(subscriptions),
            DataResult::<Subscriptions>::ERROR{ .. } => None,
        };
        assert!(subscription.is_some());
        let subscription = subscription.unwrap();
        let subscription = serde_json::to_string(&subscription).unwrap();
        let errors = check_wwf_swarm(protos.clone(), subscription.clone());
        let errors = serde_json::from_str::<CheckResult>(&errors).unwrap();
        let ok = match errors {
            CheckResult::OK => true,
            CheckResult::ERROR { .. } => false

        };
        assert!(ok);
    }
}

proptest! {
    #[test]
    fn test_overapproximated_3(vec in generate_interfacing_swarms_refinement_2(5, 5, 3)) {
        let protos = serde_json::to_string(&vec).unwrap();
        let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet::<EventType>>::new()).unwrap();
        let granularity = serde_json::to_string(&Granularity::Coarse).unwrap();
        let subscription = match serde_json::from_str(&overapproximated_weak_well_formed_sub(protos.clone(), subs, granularity)).unwrap() {
            DataResult::<Subscriptions>::OK{data: subscriptions} => Some(subscriptions),
            DataResult::<Subscriptions>::ERROR{ .. } => None,
        };
        assert!(subscription.is_some());
        let subscription = subscription.unwrap();
        let subscription = serde_json::to_string(&subscription).unwrap();
        let errors = check_wwf_swarm(protos.clone(), subscription.clone());
        let errors = serde_json::from_str::<CheckResult>(&errors).unwrap();
        let ok = match errors {
            CheckResult::OK => true,
            CheckResult::ERROR { .. } => false

        };
        assert!(ok);
    }
}

proptest! {
    #[test]
    fn test_overapproximated_4(vec in generate_interfacing_swarms_refinement_2(5, 5, 3)) {
        let protos = serde_json::to_string(&vec).unwrap();
        let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet::<EventType>>::new()).unwrap();
        let granularity = serde_json::to_string(&Granularity::Medium).unwrap();
        let subscription = match serde_json::from_str(&overapproximated_weak_well_formed_sub(protos.clone(), subs, granularity)).unwrap() {
            DataResult::<Subscriptions>::OK{data: subscriptions} => Some(subscriptions),
            DataResult::<Subscriptions>::ERROR{ .. } => None,
        };
        assert!(subscription.is_some());
        let subscription = subscription.unwrap();
        let subscription = serde_json::to_string(&subscription).unwrap();
        let errors = check_wwf_swarm(protos.clone(), subscription.clone());
        let errors = serde_json::from_str::<CheckResult>(&errors).unwrap();
        let ok = match errors {
            CheckResult::OK => true,
            CheckResult::ERROR { .. } => false

        };
        assert!(ok);
    }
}

proptest! {
    #[test]
    fn test_overapproximated_5(vec in generate_interfacing_swarms_refinement_2(5, 5, 3)) {
        let protos = serde_json::to_string(&vec).unwrap();
        let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet::<EventType>>::new()).unwrap();
        let granularity = serde_json::to_string(&Granularity::Fine).unwrap();
        let subscription = match serde_json::from_str(&overapproximated_weak_well_formed_sub(protos.clone(), subs, granularity)).unwrap() {
            DataResult::<Subscriptions>::OK{data: subscriptions} => Some(subscriptions),
            DataResult::<Subscriptions>::ERROR{ .. } => None,
        };
        assert!(subscription.is_some());
        let subscription = subscription.unwrap();
        let subscription = serde_json::to_string(&subscription).unwrap();
        let errors = check_wwf_swarm(protos.clone(), subscription.clone());
        let errors = serde_json::from_str::<CheckResult>(&errors).unwrap();
        let ok = match errors {
            CheckResult::OK => true,
            CheckResult::ERROR { .. } => false

        };
        assert!(ok);
    }
}

proptest! {
    #[test]
    #[ignore]
    fn test_overapproximated_refinement_2_only_generate(vec in generate_interfacing_swarms_refinement_2(7, 7, 10)) {
        let protos = serde_json::to_string(&vec).unwrap();
        let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet::<EventType>>::new()).unwrap();
        let granularity = serde_json::to_string(&Granularity::Coarse).unwrap();
        let subscription = match serde_json::from_str(&overapproximated_weak_well_formed_sub(protos.clone(), subs, granularity)).unwrap() {
            DataResult::<Subscriptions>::OK{data: subscriptions} => Some(subscriptions),
            DataResult::<Subscriptions>::ERROR{ .. } => None,
        };
        assert!(subscription.is_some());
    }
}

fn avg_sub_size(subscriptions: &Subscriptions) -> f32 {
    let denominator = subscriptions.keys().len();
    let mut numerator = 0;
    for events in subscriptions.values() {
        numerator += events.len();
    }

    numerator as f32 / denominator as f32
}

proptest! {
    #[test]
    #[ignore]
    fn test_sub_sizes(vec in generate_interfacing_swarms_refinement_2(5, 5, 5)) {
        let protos = serde_json::to_string(&vec).unwrap();
        let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet::<EventType>>::new()).unwrap();
        let granularity = serde_json::to_string(&Granularity::Coarse).unwrap();
        let subscription = match serde_json::from_str(&overapproximated_weak_well_formed_sub(protos.clone(), subs.clone(), granularity)).unwrap() {
            DataResult::<Subscriptions>::OK{data: subscriptions} => Some(subscriptions),
            DataResult::<Subscriptions>::ERROR{ .. } => None,
        };
        assert!(subscription.is_some());
        let subscription1 = subscription.unwrap();
        /* let subscription = serde_json::to_string(&subscription1.clone()).unwrap();
        let errors = check_wwf_swarm(protos.clone(), subscription.clone());
        let errors = serde_json::from_str::<CheckResult>(&errors).unwrap();
        let ok = match errors {
            CheckResult::OK => true,
            CheckResult::ERROR { .. } => false

        };
        assert!(ok); */
        let subscription = match serde_json::from_str(&exact_weak_well_formed_sub(protos.clone(), subs.clone())).unwrap() {
            DataResult::<Subscriptions>::OK{data: subscriptions} => Some(subscriptions),
            DataResult::<Subscriptions>::ERROR{ .. } => None,
        };
        assert!(subscription.is_some());
        let subscription2 = subscription.unwrap();
        /* let subscription = serde_json::to_string(&subscription2.clone()).unwrap();
        let errors = check_wwf_swarm(protos.clone(), subscription.clone());
        let errors = serde_json::from_str::<CheckResult>(&errors).unwrap();
        let ok = match errors {
            CheckResult::OK => true,
            CheckResult::ERROR { .. } => false

        };
        assert!(ok); */

        println!("avg sub size approx: {}", avg_sub_size(&subscription1));
        println!("avg sub size exact: {}\n", avg_sub_size(&subscription2));
    }
}

proptest! {
    //#![proptest_config(ProptestConfig::with_cases(1))]
    #[test]
    #[ignore]
    fn test_combine_machines_prop(vec in generate_interfacing_swarms_refinement_2(5, 5, 3)) {
        let protos = serde_json::to_string(&vec).unwrap();
        let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet::<EventType>>::new()).unwrap();
        let granularity = serde_json::to_string(&Granularity::Medium).unwrap();
        let subscriptions = match serde_json::from_str(&overapproximated_weak_well_formed_sub(protos.clone(), subs, granularity)).unwrap() {
            DataResult::<Subscriptions>::OK{data: subscriptions} => Some(subscriptions),
            DataResult::<Subscriptions>::ERROR{ .. } => None,
        };
        let composition = match serde_json::from_str(&compose_protocols(protos.clone())).unwrap() {
            DataResult::<SwarmProtocol>::OK{data: composition} => Some(composition),
            DataResult::<SwarmProtocol>::ERROR{ .. } => None,
        };
        assert!(subscriptions.is_some());
        assert!(composition.is_some());
        let subscriptions = subscriptions.unwrap();
        let composition = composition.unwrap();
        //let composition = InterfacingSwarms::<Role>(vec![CompositionComponent{protocol: composition.unwrap(), interface: None}]);
        let sub_string = serde_json::to_string(&subscriptions).unwrap();
        let composition_string = serde_json::to_string(&composition.clone()).unwrap();
        for role in subscriptions.keys() {
            let role_string = role.to_string();
            let projection = match serde_json::from_str(&revised_projection(composition_string.clone(), sub_string.clone(), role_string.clone())).unwrap() {
                DataResult::<Machine>::OK{data: projection} => {
                Some(projection) },
                DataResult::<Machine>::ERROR{ .. } => None,
            };
            /* let projection_combined = match serde_json::from_str(&project_combine(protos.clone(), sub_string.clone(), role_string.clone())).unwrap() {
                DataResult::<Machine>::OK{data: projection} => {
                Some(projection) },
                DataResult::<Machine>::ERROR{ .. } => None,
            };
            assert!(projection_combined.is_some());
            let machine_string = serde_json::to_string(&projection_combined.unwrap()).unwrap(); */
            // we cant do this! because we cant pass explicit composition
            /* match serde_json::from_str(&check_composed_projection(composition_string.clone(), sub_string.clone(), role.to_string(), machine_string)).unwrap() {
                CheckResult::OK => assert!(true),
                CheckResult::ERROR {errors: e} => { println!("errors: {:?}", e); assert!(false) },
            } */
            assert!(projection.is_some());
            let machine_string = serde_json::to_string(&projection.clone().unwrap()).unwrap();
            // should work like this projecting over the explicit composition initially and comparing that with combined machines?
            match serde_json::from_str(&check_composed_projection(protos.clone(), sub_string.clone(), role.clone().to_string(), machine_string)).unwrap() {
                CheckResult::OK => (),
                CheckResult::ERROR {errors: e} => {
                    match serde_json::from_str(&project_combine(protos.clone(), sub_string.clone(), role.clone().to_string())).unwrap() {
                        DataResult::<Machine>::OK{data: projection1} => {
                            println!("machine combined: {}", serde_json::to_string_pretty(&projection1).unwrap());
                        },
                        DataResult::<Machine>::ERROR{ errors: e } => println!("errors combined: {:?}", e),
                    };
                    println!("machine: {}", serde_json::to_string_pretty(&projection.unwrap()).unwrap());
                    println!("composition: {}", serde_json::to_string_pretty(&composition).unwrap());
                    for v in &vec.0 {
                        println!("component: {}", serde_json::to_string_pretty(&v.protocol).unwrap());
                    }
                    println!("errors: {:?}", e); assert!(false)
                },
            }
        }
    }
}

/* proptest! {
    #![proptest_config(ProptestConfig::with_cases(1))]
    #[test]
    #[ignore]
    fn test_combine_machines_prop_1(vec in generate_interfacing_swarms_refinement_2(5, 5, 7)) {
        let protos = serde_json::to_string(&vec).unwrap();
        let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet::<EventType>>::new()).unwrap();
        let granularity = serde_json::to_string(&Granularity::Medium).unwrap();
        let subscriptions = match serde_json::from_str(&overapproximated_weak_well_formed_sub(protos.clone(), subs, granularity)).unwrap() {
            DataResult::<Subscriptions>::OK{data: subscriptions} => Some(subscriptions),
            DataResult::<Subscriptions>::ERROR{ .. } => None,
        };

        assert!(subscriptions.is_some());

        let subscriptions = subscriptions.unwrap();

        //let composition = InterfacingSwarms::<Role>(vec![CompositionComponent{protocol: composition.unwrap(), interface: None}]);
        let sub_string = serde_json::to_string(&subscriptions).unwrap();

        for role in subscriptions.keys() {
            let role_string = role.to_string();

            let projection_combined = match serde_json::from_str(&project_combine(protos.clone(), sub_string.clone(), role_string.clone())).unwrap() {
                DataResult::<Machine>::OK{data: projection} => {
                Some(projection) },
                DataResult::<Machine>::ERROR{ .. } => None,
            };
            assert!(projection_combined.is_some());
        }
        println!("done projecting");
        let composition = match serde_json::from_str(&compose_protocols(protos.clone())).unwrap() {
            DataResult::<SwarmProtocol>::OK{data: composition} => {
                Some(composition) },
            DataResult::<SwarmProtocol>::ERROR{ .. } => None,
        };
        println!("done composing");
        println!("size of composition state space: {}", composition.unwrap().transitions.into_iter().flat_map(|label| [label.source, label.target]).collect::<BTreeSet<State>>().len());

    }
} */

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1))]
    #[test]
    #[ignore]
    fn test_write_file(vec in generate_interfacing_swarms_refinement_2(4, 4, 3)) {
        let mut mut_guard = FILE_COUNTER_MAX.lock().unwrap();
        let i: u32 = *mut_guard;
        *mut_guard += 1;
        let parent_path = "/home/luc/Git/mtvsp-project/notes/trash/graph_visualization/protos_machines_dec3".to_string();
        let dir_name = format!("4_roles_4_commands_3_protos_{}_{}", i, i);
        create_directory(&parent_path, &dir_name);

        for (j, v) in vec.clone().0.into_iter().enumerate() {
            let out = serde_json::to_string(&v.protocol)?;
            let file_name = format!("{parent_path}/{dir_name}/component_{j}_max_4_roles_max_4.json");
            write_file(&file_name, out);
        }

        let protos = serde_json::to_string(&vec).unwrap();
        //let subscriptions = match serde_json::from_str(&overapproximated_weak_well_formed_sub(protos.clone())).unwrap() {
        let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet::<EventType>>::new()).unwrap();
        let subscriptions = match serde_json::from_str(&exact_weak_well_formed_sub(protos.clone(), subs)).unwrap() {
            DataResult::<Subscriptions>::OK{data: subscriptions} => {
                write_file(&format!("{parent_path}/{dir_name}/subscription.txt"), serde_json::to_string(&subscriptions).unwrap());
                Some(subscriptions) },
            DataResult::<Subscriptions>::ERROR{ .. } => None,
        };
        let composition = match serde_json::from_str(&compose_protocols(protos.clone())).unwrap() {
            DataResult::<SwarmProtocol>::OK{data: composition} => {
                write_file(&format!("{parent_path}/{dir_name}/composition.json"), serde_json::to_string(&composition).unwrap());
                Some(composition) },
            DataResult::<SwarmProtocol>::ERROR{ .. } => None,
        };
        assert!(subscriptions.is_some());
        assert!(composition.is_some());
        let subscriptions = subscriptions.unwrap();
        let composition = composition.unwrap();
        let sub_string = serde_json::to_string(&subscriptions).unwrap();
        let composition_string = serde_json::to_string(&composition).unwrap();
        let machine_dir = "machines".to_string();
        create_directory(&format!("{parent_path}/{dir_name}"), &machine_dir);
        for role in subscriptions.keys() {
            let role_string = role.to_string();
            let projection = match serde_json::from_str(&revised_projection(composition_string.clone(), sub_string.clone(), role_string.clone())).unwrap() {
                DataResult::<Machine>::OK{data: projection} => {
                write_file(&format!("{parent_path}/{dir_name}/{machine_dir}/{role_string}.json"), serde_json::to_string(&projection).unwrap());
                Some(projection) },
                DataResult::<Machine>::ERROR{ .. } => None,
            };
            assert!(projection.is_some());
        }
    }
}
fn print_type<T>(_: &T) {
    println!("{:?}", std::any::type_name::<T>());
}
fn create_directory(parent: &String, dir_name: &String) -> () {
    match create_dir(format!("{parent}/{dir_name}")) {
        Ok(_) => (),
        Err(ref e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(e) => {print_type(&e); panic!("couldn't create directory {}/{}: {}", parent, dir_name, e)},
    }
}

fn write_file(file_name: &String, content: String) -> () {
    let path = Path::new(&file_name);
    let display = path.display();

    // Open a file in write-only mode, returns `io::Result<File>`
    let mut file = match File::create(&path) {
        //Err(Error::)
        Err(why) => panic!("couldn't create {}: {}", display, why),
        Ok(file) => file,
    };

    match file.write_all(content.as_bytes()) {
        Err(why) => panic!("couldn't write to {}: {}", display, why),
        Ok(_) => ()
    }

}

/*
    THIS ONE HAS LOTS OF PRINTS KEEP BC. NICE FOR DEBUGGING
// test whether the approximated subscription for compositions
// is contained within the 'exact' subscription.
// i.e. is the approximation safe. max five protocols, max five roles
// in each, max five commands per role. relatively small.
proptest! {
    //#![proptest_config(ProptestConfig::with_cases(5))]
    #[test]
    fn test_exact_1(vec in generate_interfacing_swarms(5, 5, 5, false)) {
        let string = serde_json::to_string(&vec).unwrap();
        let subscription = match serde_json::from_str(&exact_weak_well_formed_sub(string.clone())).unwrap() {
            DataResult::<Subscriptions>::OK{data: subscriptions} => Some(subscriptions),
            DataResult::<Subscriptions>::ERROR{ .. } => None,
        };
        assert!(subscription.is_some());
        let subscription = subscription.unwrap();
        //println!("subs: {:?}", subscription);
        let subscription = serde_json::to_string(&subscription).unwrap();

        let errors = check_wwf_swarm(string.clone(), subscription.clone());
        //println!("errors: {:?}", errors);
        let errors = serde_json::from_str::<CheckResult>(&errors).unwrap();
        let ok = match errors {
            CheckResult::OK => true,
            CheckResult::ERROR { errors: e } => {
                println!("{:?}", e);
                println!("subs: {}", serde_json::to_string_pretty(&subscription).unwrap());
                for v in &vec.0 {
                    println!("component: {}", serde_json::to_string_pretty(&v.protocol).unwrap());
                }
                let c = compose_protocols(string);
                match serde_json::from_str(&c).unwrap() {
                    DataResult::<SwarmProtocol>::OK{data: protocol} => {println!("protocol: {}", serde_json::to_string_pretty(&protocol).unwrap())},
                    DataResult::<SwarmProtocol>::ERROR{..} => (),
                }
                false}

        };
        assert!(ok);
    }
}



*/

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BenchMarkInput  {
    pub state_space_size: usize,
    pub number_of_edges: usize,
    pub interfacing_swarms: InterfacingSwarms<Role>
}

fn wrap_and_write(interfacing_swarms: InterfacingSwarms<Role>, parent_path: String, dir_name: String) {
    let interfacing_swarms_string = serde_json::to_string(&interfacing_swarms).unwrap();
    let composition = match serde_json::from_str(&compose_protocols(interfacing_swarms_string.clone())).unwrap() {
        DataResult::<SwarmProtocol>::OK{data: composition} => {
            Some(composition) },
        DataResult::<SwarmProtocol>::ERROR{ .. } => None,
    };
    let composition = composition.unwrap();
    let state_space_size = composition.transitions.iter().flat_map(|label| [label.source.clone(), label.target.clone()]).collect::<BTreeSet<State>>().len();
    let number_of_edges = composition.transitions.iter().len();
    let benchmark_input = BenchMarkInput {state_space_size, number_of_edges, interfacing_swarms};
    let mut mut_guard = FILE_COUNTER_MAX.lock().unwrap();
    let i: u32 = *mut_guard;
    *mut_guard += 1;
    let file_name = format!("{parent_path}/{dir_name}/{:010}_{:010}_{dir_name}.json", state_space_size, i);
    let out = serde_json::to_string(&benchmark_input).unwrap();
    write_file(&file_name, out);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]
    #[test]
    #[ignore]
    fn write_bench_file_ref_1_5_1(interfacing_swarms in generate_interfacing_swarms_refinement(5, 5, 1)) {
        let parent_path = "benches/benchmark_data/refinement_pattern_1".to_string();
        let dir_name = format!("max_5_roles_max_5_commands_1_protos");
        create_directory(&parent_path, &dir_name);
        wrap_and_write(interfacing_swarms, parent_path, dir_name);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]
    #[test]
    #[ignore]
    fn write_bench_file_ref_1_5_2(interfacing_swarms in generate_interfacing_swarms_refinement(5, 5, 2)) {
        let parent_path = "benches/benchmark_data/refinement_pattern_1".to_string();
        let dir_name = format!("max_5_roles_max_5_commands_2_protos");
        create_directory(&parent_path, &dir_name);
        wrap_and_write(interfacing_swarms, parent_path, dir_name);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]
    #[test]
    #[ignore]
    fn write_bench_file_ref_1_5_3(interfacing_swarms in generate_interfacing_swarms_refinement(5, 5, 3)) {
        let parent_path = "benches/benchmark_data/refinement_pattern_1".to_string();
        let dir_name = format!("max_5_roles_max_5_commands_3_protos");
        create_directory(&parent_path, &dir_name);
        wrap_and_write(interfacing_swarms, parent_path, dir_name);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]
    #[test]
    #[ignore]
    fn write_bench_file_ref_1_5_4(interfacing_swarms in generate_interfacing_swarms_refinement(5, 5, 4)) {
        let parent_path = "benches/benchmark_data/refinement_pattern_1".to_string();
        let dir_name = format!("max_5_roles_max_5_commands_4_protos");
        create_directory(&parent_path, &dir_name);
        wrap_and_write(interfacing_swarms, parent_path, dir_name);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]
    #[test]
    #[ignore]
    fn write_bench_file_ref_1_5_5(interfacing_swarms in generate_interfacing_swarms_refinement(5, 5, 5)) {
        let parent_path = "benches/benchmark_data/refinement_pattern_1".to_string();
        let dir_name = format!("max_5_roles_max_5_commands_5_protos");
        create_directory(&parent_path, &dir_name);
        wrap_and_write(interfacing_swarms, parent_path, dir_name);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]
    #[test]
    #[ignore]
    fn write_bench_file_ref_1_5_6(interfacing_swarms in generate_interfacing_swarms_refinement(5, 5, 6)) {
        let parent_path = "benches/benchmark_data/refinement_pattern_1".to_string();
        let dir_name = format!("max_5_roles_max_5_commands_6_protos");
        create_directory(&parent_path, &dir_name);
        wrap_and_write(interfacing_swarms, parent_path, dir_name);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]
    #[test]
    #[ignore]
    fn write_bench_file_ref_1_5_7(interfacing_swarms in generate_interfacing_swarms_refinement(5, 5, 7)) {
        let parent_path = "benches/benchmark_data/refinement_pattern_1".to_string();
        let dir_name = format!("max_5_roles_max_5_commands_7_protos");
        create_directory(&parent_path, &dir_name);
        wrap_and_write(interfacing_swarms, parent_path, dir_name);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]
    #[test]
    #[ignore]
    fn write_bench_file_ref_1_5_8(interfacing_swarms in generate_interfacing_swarms_refinement(5, 5, 8)) {
        let parent_path = "benches/benchmark_data/refinement_pattern_1".to_string();
        let dir_name = format!("max_5_roles_max_5_commands_8_protos");
        create_directory(&parent_path, &dir_name);
        wrap_and_write(interfacing_swarms, parent_path, dir_name);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]
    #[test]
    #[ignore]
    fn write_bench_file_ref_1_5_9(interfacing_swarms in generate_interfacing_swarms_refinement(5, 5, 9)) {
        let parent_path = "benches/benchmark_data/refinement_pattern_1".to_string();
        let dir_name = format!("max_5_roles_max_5_commands_9_protos");
        create_directory(&parent_path, &dir_name);
        wrap_and_write(interfacing_swarms, parent_path, dir_name);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]
    #[test]
    #[ignore]
    fn write_bench_file_ref_1_5_10(interfacing_swarms in generate_interfacing_swarms_refinement(5, 5, 10)) {
        let parent_path = "benches/benchmark_data/refinement_pattern_1".to_string();
        let dir_name = format!("max_5_roles_max_5_commands_10_protos");
        create_directory(&parent_path, &dir_name);
        wrap_and_write(interfacing_swarms, parent_path, dir_name);
    }
}


proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]
    #[test]
    #[ignore]
    fn write_bench_file_ref_1_10_1(interfacing_swarms in generate_interfacing_swarms_refinement(10, 10, 1)) {
        let parent_path = "benches/benchmark_data/refinement_pattern_1".to_string();
        let dir_name = format!("max_10_roles_max_10_commands_1_protos");
        create_directory(&parent_path, &dir_name);
        wrap_and_write(interfacing_swarms, parent_path, dir_name);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]
    #[test]
    #[ignore]
    fn write_bench_file_ref_1_10_2(interfacing_swarms in generate_interfacing_swarms_refinement(10, 10, 2)) {
        let parent_path = "benches/benchmark_data/refinement_pattern_1".to_string();
        let dir_name = format!("max_10_roles_max_10_commands_2_protos");
        create_directory(&parent_path, &dir_name);
        wrap_and_write(interfacing_swarms, parent_path, dir_name);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]
    #[test]
    #[ignore]
    fn write_bench_file_ref_1_10_3(interfacing_swarms in generate_interfacing_swarms_refinement(10, 10, 3)) {
        let parent_path = "benches/benchmark_data/refinement_pattern_1".to_string();
        let dir_name = format!("max_10_roles_max_10_commands_3_protos");
        create_directory(&parent_path, &dir_name);
        wrap_and_write(interfacing_swarms, parent_path, dir_name);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]
    #[test]
    #[ignore]
    fn write_bench_file_ref_1_10_4(interfacing_swarms in generate_interfacing_swarms_refinement(10, 10, 4)) {
        let parent_path = "benches/benchmark_data/refinement_pattern_1".to_string();
        let dir_name = format!("max_10_roles_max_10_commands_4_protos");
        create_directory(&parent_path, &dir_name);
        wrap_and_write(interfacing_swarms, parent_path, dir_name);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]
    #[test]
    #[ignore]
    fn write_bench_file_ref_1_10_5(interfacing_swarms in generate_interfacing_swarms_refinement(10, 10, 5)) {
        let parent_path = "benches/benchmark_data/refinement_pattern_1".to_string();
        let dir_name = format!("max_10_roles_max_10_commands_5_protos");
        create_directory(&parent_path, &dir_name);
        wrap_and_write(interfacing_swarms, parent_path, dir_name);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]
    #[test]
    #[ignore]
    fn write_bench_file_ref_1_10_6(interfacing_swarms in generate_interfacing_swarms_refinement(10, 10, 6)) {
        let parent_path = "benches/benchmark_data/refinement_pattern_1".to_string();
        let dir_name = format!("max_10_roles_max_10_commands_6_protos");
        create_directory(&parent_path, &dir_name);
        wrap_and_write(interfacing_swarms, parent_path, dir_name);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]
    #[test]
    #[ignore]
    fn write_bench_file_ref_1_10_7(interfacing_swarms in generate_interfacing_swarms_refinement(10, 10, 7)) {
        let parent_path = "benches/benchmark_data/refinement_pattern_1".to_string();
        let dir_name = format!("max_10_roles_max_10_commands_7_protos");
        create_directory(&parent_path, &dir_name);
        wrap_and_write(interfacing_swarms, parent_path, dir_name);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]
    #[test]
    #[ignore]
    fn write_bench_file_ref_1_10_8(interfacing_swarms in generate_interfacing_swarms_refinement(10, 10, 8)) {
        let parent_path = "benches/benchmark_data/refinement_pattern_1".to_string();
        let dir_name = format!("max_10_roles_max_10_commands_8_protos");
        create_directory(&parent_path, &dir_name);
        wrap_and_write(interfacing_swarms, parent_path, dir_name);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]
    #[test]
    #[ignore]
    fn write_bench_file_ref_1_10_9(interfacing_swarms in generate_interfacing_swarms_refinement(10, 10, 9)) {
        let parent_path = "benches/benchmark_data/refinement_pattern_1".to_string();
        let dir_name = format!("max_10_roles_max_10_commands_9_protos");
        create_directory(&parent_path, &dir_name);
        wrap_and_write(interfacing_swarms, parent_path, dir_name);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]
    #[test]
    #[ignore]
    fn write_bench_file_ref_1_10_10(interfacing_swarms in generate_interfacing_swarms_refinement(10, 10, 10)) {
        let parent_path = "benches/benchmark_data/refinement_pattern_1".to_string();
        let dir_name = format!("max_10_roles_max_10_commands_10_protos");
        create_directory(&parent_path, &dir_name);
        wrap_and_write(interfacing_swarms, parent_path, dir_name);
    }
}

fn prepare_input(file_name: String) -> (usize, BenchMarkInput) {
    // Create a path to the desired file
    let path = Path::new(&file_name);
    let display = path.display();

    // Open the path in read-only mode, returns `io::Result<File>`
    let mut file = match File::open(&path) {
        Err(why) => panic!("couldn't open {}: {}", display, why),
        Ok(file) => file,
    };

    // Read the file contents into a string, returns `io::Result<usize>`
    let mut protos = String::new();
    match file.read_to_string(&mut protos) {
        Err(why) => panic!("couldn't read {}: {}", display, why),
        Ok(_) => (), //print!("{} contains:\n{}", display, protos),
    }
    let (state_space_size, interfacing_swarms) =
        match serde_json::from_str::<BenchMarkInput>(&protos) {
            Ok(input) => (input.state_space_size, input),
            Err(e) => panic!("error parsing input file: {}", e),
        };
    (
        state_space_size,
        interfacing_swarms//serde_json::to_string(&interfacing_swarms).unwrap(),
    )
}

fn prepare_files_in_directory(directory: String) -> Vec<(usize, BenchMarkInput)> {
    let mut inputs: Vec<(usize, BenchMarkInput)> = vec![];

    for entry in WalkDir::new(directory) {
        match entry {
            Ok(entry) => {
                if entry.file_type().is_file() {
                    println!("file: {}", entry.path().as_os_str().to_str().unwrap().to_string());
                    inputs.push(prepare_input(
                        entry.path().as_os_str().to_str().unwrap().to_string(),
                    ));
                }
            }
            Err(e) => panic!("error: {}", e),
        };
    }

    inputs
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BenchmarkSubSizeOutput  {
    pub state_space_size: usize,
    pub number_of_edges: usize,
    pub subscriptions: Subscriptions,
}

fn wrap_and_write_sub_out(bench_input: &BenchMarkInput, subscriptions: Subscriptions, granularity: String, parent_path: String) {
    let out = BenchmarkSubSizeOutput { state_space_size: bench_input.state_space_size, number_of_edges: bench_input.number_of_edges, subscriptions: subscriptions};
    let file_name = format!("{parent_path}/{:010}_{}.json", bench_input.state_space_size, granularity);
    let out = serde_json::to_string(&out).unwrap();
    write_file(&file_name, out);
}

#[test]
#[ignore]
fn bench_sub_sizes_refinement_1() {
    let mut interfacing_swarms_refinement_1 =
        prepare_files_in_directory(String::from("./benches/benchmark_data_selected/refinement_pattern_1/"));
    interfacing_swarms_refinement_1.sort_by(|(size1, _), (size2, _)| size1.cmp(size2));
    let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet<EventType>>::new()).unwrap();
    let coarse_granularity = serde_json::to_string(&Granularity::Coarse).unwrap();
    let medium_granularity = serde_json::to_string(&Granularity::Medium).unwrap();
    let fine_granularity = serde_json::to_string(&Granularity::Fine).unwrap();
    let granularities = vec![coarse_granularity, medium_granularity, fine_granularity];
    for (_, bi) in &interfacing_swarms_refinement_1 {
        //let bi = serde_json::from_str::<BenchMarkInput>(&bench_input).unwrap();
        let swarms = serde_json::to_string(&bi.interfacing_swarms).unwrap();
        for g in &granularities {

            let subscriptions = match serde_json::from_str(&overapproximated_weak_well_formed_sub(swarms.clone(), subs.clone(), g.clone())).unwrap() {
                DataResult::<Subscriptions>::OK{data: subscriptions} => Some(subscriptions),
                DataResult::<Subscriptions>::ERROR{ .. } => None,
            };

            wrap_and_write_sub_out(&bi, subscriptions.unwrap(), g.replace("\"", ""), String::from("./subscription_size_benchmarks/refinement_pattern_1_2"));

        }

        let subscriptions = match serde_json::from_str(&exact_weak_well_formed_sub(swarms.clone(), subs.clone())).unwrap() {
            DataResult::<Subscriptions>::OK{data: subscriptions} => {
                Some(subscriptions) },
            DataResult::<Subscriptions>::ERROR{ .. } => None,
        };


        wrap_and_write_sub_out(&bi, subscriptions.unwrap(), String::from("Exact"), String::from("./subscription_size_benchmarks/refinement_pattern_1_2"));

    }
}

#[test]
#[ignore]
fn bench_sub_sizes_refinement_2() {
    let mut interfacing_swarms_refinement_2 =
        prepare_files_in_directory(String::from("./benches/benchmark_data_selected/refinement_pattern_2/"));
    interfacing_swarms_refinement_2.sort_by(|(size1, _), (size2, _)| size1.cmp(size2));
    let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet<EventType>>::new()).unwrap();
    let coarse_granularity = serde_json::to_string(&Granularity::Coarse).unwrap();
    let medium_granularity = serde_json::to_string(&Granularity::Medium).unwrap();
    let fine_granularity = serde_json::to_string(&Granularity::Fine).unwrap();
    let granularities = vec![coarse_granularity, medium_granularity, fine_granularity];
    for (_, bi) in &interfacing_swarms_refinement_2 {
        //let bi = serde_json::from_str::<BenchMarkInput>(&bench_input).unwrap();
        let swarms = serde_json::to_string(&bi.interfacing_swarms).unwrap();
        for g in &granularities {

            let subscriptions = match serde_json::from_str(&overapproximated_weak_well_formed_sub(swarms.clone(), subs.clone(), g.clone())).unwrap() {
                DataResult::<Subscriptions>::OK{data: subscriptions} => Some(subscriptions),
                DataResult::<Subscriptions>::ERROR{ .. } => None,
            };

            wrap_and_write_sub_out(&bi, subscriptions.unwrap(), g.replace("\"", ""), String::from("./subscription_size_benchmarks/refinement_pattern_2_2"));

        }

        let subscriptions = match serde_json::from_str(&exact_weak_well_formed_sub(swarms.clone(), subs.clone())).unwrap() {
            DataResult::<Subscriptions>::OK{data: subscriptions} => {
                Some(subscriptions) },
            DataResult::<Subscriptions>::ERROR{ .. } => None,
        };


        wrap_and_write_sub_out(&bi, subscriptions.unwrap(), String::from("Exact"), String::from("./subscription_size_benchmarks/refinement_pattern_2_2"));

    }
}

#[test]
#[ignore]
fn bench_sub_sizes_random() {
    let mut interfacing_swarms_random =
        prepare_files_in_directory(String::from("./benches/benchmark_data_selected/random/"));
    interfacing_swarms_random.sort_by(|(size1, _), (size2, _)| size1.cmp(size2));
    let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet<EventType>>::new()).unwrap();
    let coarse_granularity = serde_json::to_string(&Granularity::Coarse).unwrap();
    let medium_granularity = serde_json::to_string(&Granularity::Medium).unwrap();
    let fine_granularity = serde_json::to_string(&Granularity::Fine).unwrap();
    let granularities = vec![coarse_granularity, medium_granularity, fine_granularity];
    for (_, bi) in &interfacing_swarms_random {
        let swarms = serde_json::to_string(&bi.interfacing_swarms).unwrap();
        for g in &granularities {

            let subscriptions = match serde_json::from_str(&overapproximated_weak_well_formed_sub(swarms.clone(), subs.clone(), g.clone())).unwrap() {
                DataResult::<Subscriptions>::OK{data: subscriptions} => Some(subscriptions),
                DataResult::<Subscriptions>::ERROR{ .. } => None,
            };

            wrap_and_write_sub_out(&bi, subscriptions.unwrap(), g.replace("\"", ""), String::from("./subscription_size_benchmarks/random_2"));

        }

        let subscriptions = match serde_json::from_str(&exact_weak_well_formed_sub(swarms.clone(), subs.clone())).unwrap() {
            DataResult::<Subscriptions>::OK{data: subscriptions} => {
                Some(subscriptions) },
            DataResult::<Subscriptions>::ERROR{ .. } => None,
        };


        wrap_and_write_sub_out(&bi, subscriptions.unwrap(), String::from("Exact"), String::from("./subscription_size_benchmarks/random_2"));

    }
}

#[test]
#[ignore]
fn print_sub_sizes_refinement_2() {
    let mut interfacing_swarms_refinement_2 =
        prepare_files_in_directory(String::from("./benches/benchmark_data/random/"));
    interfacing_swarms_refinement_2.sort_by(|(size1, _), (size2, _)| size1.cmp(size2));
    let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet<EventType>>::new()).unwrap();
    let fine_granularity = serde_json::to_string(&Granularity::Fine).unwrap();
    let mut i = 0;
    for (_, bi) in &interfacing_swarms_refinement_2 {
        //let bi = serde_json::from_str::<BenchMarkInput>(&bench_input).unwrap();
        let swarms = serde_json::to_string(&bi.interfacing_swarms).unwrap();
        let subscriptions_fine_approx = match serde_json::from_str(&overapproximated_weak_well_formed_sub(swarms.clone(), subs.clone(), fine_granularity.clone())).unwrap() {
                DataResult::<Subscriptions>::OK{data: subscriptions} => Some(subscriptions),
                DataResult::<Subscriptions>::ERROR{ .. } => None,
        };

        let subscriptions_exact = match serde_json::from_str(&exact_weak_well_formed_sub(swarms.clone(), subs.clone())).unwrap() {
            DataResult::<Subscriptions>::OK{data: subscriptions} => {
                Some(subscriptions) },
            DataResult::<Subscriptions>::ERROR{ .. } => None,
        };
        if subscriptions_fine_approx.clone().unwrap() != subscriptions_exact.clone().unwrap() {
            if i == 0 {
                println!("sub approx: {}", serde_json::to_string_pretty(&subscriptions_fine_approx).unwrap());
                println!("sub exact: {}", serde_json::to_string_pretty(&subscriptions_exact).unwrap());
                println!("swarms: {}", serde_json::to_string_pretty(&bi.interfacing_swarms).unwrap());
                let composition = match serde_json::from_str(&compose_protocols(swarms)).unwrap() {
                    DataResult::<SwarmProtocol>::OK{data: composition} => {
                        Some(composition) },
                    DataResult::<SwarmProtocol>::ERROR{ .. } => None,
                };
                println!("composition: {}", serde_json::to_string_pretty(&composition.unwrap()).unwrap());
                break;
            }
            i = i + 1;
        }
    }
}