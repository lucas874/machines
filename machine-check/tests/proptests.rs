use machine_check::{
    check_swarm, composition::{check_composed_projection, check_composed_swarm, compose_protocols, composition_types::{CompositionComponent, Granularity, InterfacingProtocols, InterfacingSwarms},
    exact_well_formed_sub, overapproximated_well_formed_sub, project_combine, revised_projection}, types::{CheckResult, Command, DataResult, EventType, MachineLabel, Role, State, StateName, SwarmLabel, Transition}, well_formed_sub, EdgeId, Graph, MachineType, NodeId, Subscriptions, SwarmProtocolType
};
use petgraph::{
    graph::EdgeReference,
    visit::{Dfs, EdgeRef, Reversed, Walker},
    Direction::{Incoming, Outgoing},
};
use proptest::prelude::*;
use rand::{distributions::Bernoulli, prelude::*};
use std::{
    cmp, collections::{BTreeMap, BTreeSet}, iter::zip, sync::Mutex
};
use tracing_subscriber::{fmt, fmt::format::FmtSpan, EnvFilter};

// reimplemented here because we need to Deserialize. To not change in types.rs
/* #[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum CheckResult {
    OK,
    ERROR { errors: Vec<String> },
} */

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


fn setup_logger() {
    fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
        .try_init()
        .ok();
}

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
    let b_dist = Bernoulli::new(0.1).unwrap(); // bernoulli distribution with propability 0.1 of success
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

pub fn to_swarm_json(graph: crate::Graph, initial: NodeId) -> SwarmProtocolType {
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

    SwarmProtocolType {
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
    fn protos_refinement_2(max_roles: usize, max_events: usize, num_protos: usize)
                (labels in all_labels_2((0..num_protos).into_iter().map(|i| Role::new(&format!("{IR_BASE}_{i}"))).collect(), cmp::max(0, max_roles-1), max_events))
                -> Vec<((Graph, NodeId), Vec<SwarmLabel>)> {
        labels.into_iter().map(|(ir_labels, labels)| (random_graph(None, ir_labels.into_iter().rev().collect()), labels)).collect()
    }
}

// Aka general pattern
prop_compose! {
    fn generate_interfacing_swarms_refinement_2(max_roles: usize, max_events: usize, num_protos: usize)
                (protos in protos_refinement_2(max_roles, max_events, num_protos))
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
    let b_dist = Bernoulli::new(0.1).unwrap(); // bernoulli distribution with propability 0.1 of success
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

fn to_interfacing_protocols(interfacing_swarms: InterfacingSwarms<Role>) -> InterfacingProtocols {
    InterfacingProtocols(interfacing_swarms
        .0
        .into_iter()
        .map(|cc| cc.protocol)
        .collect())
}

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
    fn test_exact_1(protos in generate_interfacing_swarms(5, 5, 5, false)) {
        setup_logger();
        let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet::<EventType>>::new()).unwrap();
        let subscription: Option<Subscriptions> = match exact_well_formed_sub(to_interfacing_protocols(protos.clone()), subs) {
            DataResult::OK{data: subscriptions} => Some(subscriptions),
            DataResult::ERROR{ .. } => None,
        };
        assert!(subscription.is_some());
        let subscription = subscription.unwrap();
        let subscription = serde_json::to_string(&subscription).unwrap();
        let errors = check_composed_swarm(to_interfacing_protocols(protos.clone()), subscription.clone());
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
    fn test_overapproximated_1(protos in generate_interfacing_swarms(5, 5, 5, false)) {
        setup_logger();
        let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet::<EventType>>::new()).unwrap();
        let granularity = Granularity::Coarse;
        let subscription: Option<Subscriptions> = match overapproximated_well_formed_sub(to_interfacing_protocols(protos.clone()), subs, granularity) {
            DataResult::OK{data: subscriptions} => Some(subscriptions),
            DataResult::ERROR{ .. } => None,
        };
        assert!(subscription.is_some());
        let subscription = subscription.unwrap();
        let subscription = serde_json::to_string(&subscription).unwrap();
        let errors = check_composed_swarm(to_interfacing_protocols(protos.clone()), subscription.clone());
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
    fn test_exact_2(protos in generate_interfacing_swarms_refinement(5, 5, 5)) {
        setup_logger();
        let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet::<EventType>>::new()).unwrap();
        let subscription: Option<Subscriptions> = match exact_well_formed_sub(to_interfacing_protocols(protos.clone()), subs) {
            DataResult::OK{data: subscriptions} => Some(subscriptions),
            DataResult::ERROR{ .. } => None,
        };
        assert!(subscription.is_some());
        let subscription = subscription.unwrap();
        let subscription = serde_json::to_string(&subscription).unwrap();
        let errors = check_composed_swarm(to_interfacing_protocols(protos.clone()), subscription.clone());
        let ok = match errors {
            CheckResult::OK => true,
            CheckResult::ERROR { .. } => false
        };
        assert!(ok);
    }
}

proptest! {
    #[test]
    fn test_overapproximated_2(protos in generate_interfacing_swarms_refinement(5, 5, 5)) {
        setup_logger();
        let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet::<EventType>>::new()).unwrap();
        let granularity = Granularity::Coarse;
        let subscription: Option<Subscriptions> = match overapproximated_well_formed_sub(to_interfacing_protocols(protos.clone()), subs, granularity) {
            DataResult::OK{data: subscriptions} => Some(subscriptions),
            DataResult::ERROR{ .. } => None,
        };
        assert!(subscription.is_some());
        let subscription = subscription.unwrap();
        let subscription = serde_json::to_string(&subscription).unwrap();
        let errors = check_composed_swarm(to_interfacing_protocols(protos.clone()), subscription.clone());
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
    fn test_exact_3(protos in generate_interfacing_swarms_refinement_2(5, 5, 3)) {
        setup_logger();
        let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet::<EventType>>::new()).unwrap();
        let subscription: Option<Subscriptions> = match exact_well_formed_sub(to_interfacing_protocols(protos.clone()), subs) {
            DataResult::OK{data: subscriptions} => Some(subscriptions),
            DataResult::ERROR{ .. } => None,
        };
        assert!(subscription.is_some());
        let subscription = subscription.unwrap();
        let subscription = serde_json::to_string(&subscription).unwrap();
        let errors = check_composed_swarm(to_interfacing_protocols(protos.clone()), subscription.clone());
        let ok = match errors {
            CheckResult::OK => true,
            CheckResult::ERROR { .. } => false
        };
        assert!(ok);
    }
}

proptest! {
    #[test]
    fn test_overapproximated_3(protos in generate_interfacing_swarms_refinement_2(5, 5, 3)) {
        setup_logger();
        let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet::<EventType>>::new()).unwrap();
        let granularity = Granularity::Coarse;
        let subscription: Option<Subscriptions> = match overapproximated_well_formed_sub(to_interfacing_protocols(protos.clone()), subs, granularity) {
            DataResult::OK{data: subscriptions} => Some(subscriptions),
            DataResult::ERROR{ .. } => None,
        };
        assert!(subscription.is_some());
        let subscription = subscription.unwrap();
        let subscription = serde_json::to_string(&subscription).unwrap();
        let errors = check_composed_swarm(to_interfacing_protocols(protos.clone()), subscription.clone());
        let ok = match errors {
            CheckResult::OK => true,
            CheckResult::ERROR { .. } => false
        };
        assert!(ok);
    }
}

proptest! {
    #[test]
    fn test_overapproximated_4(protos in generate_interfacing_swarms_refinement_2(5, 5, 3)) {
        setup_logger();
        let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet::<EventType>>::new()).unwrap();
        let granularity = Granularity::Medium;
        let subscription: Option<Subscriptions> = match overapproximated_well_formed_sub(to_interfacing_protocols(protos.clone()), subs, granularity) {
            DataResult::OK{data: subscriptions} => Some(subscriptions),
            DataResult::ERROR{ .. } => None,
        };
        assert!(subscription.is_some());
        let subscription = subscription.unwrap();
        let subscription = serde_json::to_string(&subscription).unwrap();
        let errors = check_composed_swarm(to_interfacing_protocols(protos.clone()), subscription.clone());
        let ok = match errors {
            CheckResult::OK => true,
            CheckResult::ERROR { .. } => false
        };
        assert!(ok);
    }
}

proptest! {
    #[test]
    fn test_overapproximated_5(protos in generate_interfacing_swarms_refinement_2(5, 5, 3)) {
        setup_logger();
        let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet::<EventType>>::new()).unwrap();
        let granularity = Granularity::Fine;
        let subscription: Option<Subscriptions> = match overapproximated_well_formed_sub(to_interfacing_protocols(protos.clone()), subs, granularity) {
            DataResult::OK{data: subscriptions} => Some(subscriptions),
            DataResult::ERROR{ .. } => None,
        };
        assert!(subscription.is_some());
        let subscription = subscription.unwrap();
        let subscription = serde_json::to_string(&subscription).unwrap();
        let errors = check_composed_swarm(to_interfacing_protocols(protos.clone()), subscription.clone());
        let ok = match errors {
            CheckResult::OK => true,
            CheckResult::ERROR { .. } => false
        };
        assert!(ok);
    }
}

proptest! {
    #[test]
    fn test_overapproximated_6(protos in generate_interfacing_swarms_refinement_2(5, 5, 3)) {
        setup_logger();
        let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet::<EventType>>::new()).unwrap();
        let granularity = Granularity::TwoStep;
        let subscription: Option<Subscriptions> = match overapproximated_well_formed_sub(to_interfacing_protocols(protos.clone()), subs, granularity) {
            DataResult::OK{data: subscriptions} => Some(subscriptions),
            DataResult::ERROR{ .. } => None,
        };
        assert!(subscription.is_some());
        let subscription = subscription.unwrap();
        let subscription = serde_json::to_string(&subscription).unwrap();
        let errors = check_composed_swarm(to_interfacing_protocols(protos.clone()), subscription.clone());
        let ok = match errors {
            CheckResult::OK => true,
            CheckResult::ERROR { .. } => false
        };
        assert!(ok);
    }
}

proptest! {
    #[test]
    fn test_overapproximated_7(protos in generate_interfacing_swarms_refinement(5, 5, 5)) {
        setup_logger();
        let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet::<EventType>>::new()).unwrap();
        let granularity = Granularity::TwoStep;
        let subscription: Option<Subscriptions> = match overapproximated_well_formed_sub(to_interfacing_protocols(protos.clone()), subs, granularity) {
            DataResult::OK{data: subscriptions} => Some(subscriptions),
            DataResult::ERROR{ .. } => None,
        };
        assert!(subscription.is_some());
        let subscription = subscription.unwrap();
        let subscription = serde_json::to_string(&subscription).unwrap();
        let errors = check_composed_swarm(to_interfacing_protocols(protos.clone()), subscription.clone());
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
    fn test_overapproximated_refinement_2_only_generate(protos in generate_interfacing_swarms_refinement_2(7, 7, 10)) {
        setup_logger();
        let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet::<EventType>>::new()).unwrap();
        let granularity = Granularity::Coarse;
        let subscription: Option<Subscriptions> = match overapproximated_well_formed_sub(to_interfacing_protocols(protos.clone()), subs, granularity) {
            DataResult::OK{data: subscriptions} => Some(subscriptions),
            DataResult::ERROR{ .. } => None,
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
    fn test_sub_sizes(protos in generate_interfacing_swarms_refinement_2(5, 5, 5)) {
        setup_logger();
        let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet::<EventType>>::new()).unwrap();
        let granularity = Granularity::Coarse;
        let subscription: Option<Subscriptions> = match overapproximated_well_formed_sub(to_interfacing_protocols(protos.clone()), subs.clone(), granularity) {
            DataResult::OK{data: subscriptions} => Some(subscriptions),
            DataResult::ERROR{ .. } => None,
        };
        assert!(subscription.is_some());
        let subscription1 = subscription.unwrap();
        /* let subscription = serde_json::to_string(&subscription1.clone()).unwrap();
        let errors = check_wwf_swarm(to_interfacing_protocols(protos.clone()), subscription.clone());
        let errors = serde_json::from_str::<CheckResult>(&errors).unwrap();
        let ok = match errors {
            CheckResult::OK => true,
            CheckResult::ERROR { .. } => false

        };
        assert!(ok); */
        let subscription: Option<Subscriptions> = match exact_well_formed_sub(to_interfacing_protocols(protos.clone()), subs.clone()) {
            DataResult::OK{data: subscriptions} => Some(subscriptions),
            DataResult::ERROR{ .. } => None,
        };
        assert!(subscription.is_some());
        let subscription2 = subscription.unwrap();
        /* let subscription = serde_json::to_string(&subscription2.clone()).unwrap();
        let errors = check_wwf_swarm(to_interfacing_protocols(protos.clone()), subscription.clone());
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

fn num_states(proto: &MachineType) -> usize {
    let mut states = BTreeSet::new();
    states.insert(proto.initial.clone());
    for t in &proto.transitions {
        states.insert(t.source.clone());
        states.insert(t.target.clone());
    }
    states.len()
}

fn num_transitions(proto: &MachineType) -> usize {
    let mut transitions = BTreeSet::new();
    for t in &proto.transitions {
        transitions.insert(t.clone());
    }
    transitions.len()
}

fn num_terminal(proto: &MachineType) -> usize {
    let mut states = BTreeSet::new();
    states.insert(proto.initial.clone());
    let mut states_to_outgoing = BTreeMap::new();
    let mut terminal_states = BTreeSet::new();
    for t in &proto.transitions {
        states.insert(t.source.clone());
        states.insert(t.target.clone());
        states_to_outgoing.entry(t.source.clone()).and_modify(|ts: &mut BTreeSet<Transition<MachineLabel>>| {ts.insert(t.clone());}).or_insert_with(|| BTreeSet::new());
    }
    for state in &states {
        if !states_to_outgoing.contains_key(state) {
            terminal_states.insert(state.clone());
        }
    }
    terminal_states.len()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    #[test]
    //#[ignore]
    fn test_combine_machines_prop(protos in generate_interfacing_swarms_refinement_2(5, 5, 3)) {
        setup_logger();
        let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet::<EventType>>::new()).unwrap();
        let granularity = Granularity::TwoStep;
        let subscriptions: Option<Subscriptions> = match overapproximated_well_formed_sub(to_interfacing_protocols(protos.clone()), subs.clone(), granularity) {
            DataResult::OK{data: subscriptions} => Some(subscriptions),
            DataResult::ERROR{ .. } => None,
        };
        let composition: Option<SwarmProtocolType> = match compose_protocols(to_interfacing_protocols(protos.clone())) {
            DataResult::OK{data: composition} => Some(composition),
            DataResult::ERROR{ .. } => None,
        };
        assert!(subscriptions.is_some());
        assert!(composition.is_some());
        let subscriptions = subscriptions.unwrap();
        let composition = composition.unwrap();
        //let composition = InterfacingSwarms::<Role>(vec![CompositionComponent{protocol: composition.unwrap(), interface: None}]);
        let sub_string = serde_json::to_string(&subscriptions).unwrap();
        for role in subscriptions.keys() {
            let projection: Option<MachineType> = match revised_projection(composition.clone(), sub_string.clone(), role.clone(), true) {
                DataResult::OK{data: projection} => {
                Some(projection) },
                DataResult::ERROR{ .. } => None,
            };

            assert!(projection.is_some());
            // should work like this projecting over the explicit composition initially and comparing that with combined machines?
            match check_composed_projection(to_interfacing_protocols(protos.clone()), sub_string.clone(), role.clone(), projection.clone().unwrap()) {
                CheckResult::OK => (),
                CheckResult::ERROR {errors: e} => {
                    match project_combine(to_interfacing_protocols(protos.clone()), sub_string.clone(), role.clone(), false) {
                        DataResult::OK{data: projection1} => {
                            println!("machine combined: {}", serde_json::to_string_pretty::<MachineType>(&projection1).unwrap());
                        },
                        DataResult::ERROR{ errors: e } => println!("errors combined: {:?}", e),
                    };
                    println!("machine: {}", serde_json::to_string_pretty(&projection.unwrap()).unwrap());
                    println!("composition: {}", serde_json::to_string_pretty(&composition).unwrap());
                    for v in &protos.0 {
                        println!("component: {}", serde_json::to_string_pretty(&v.protocol).unwrap());
                    }
                    println!("errors: {:?}", e); assert!(false)
                },
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    #[test]
    #[ignore]
    fn test_combine_machines_prop_more_verbose(protos in generate_interfacing_swarms_refinement_2(5, 5, 3)) {
        setup_logger();
        let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet::<EventType>>::new()).unwrap();
        let granularity = Granularity::TwoStep;
        let subscriptions: Option<Subscriptions> = match overapproximated_well_formed_sub(to_interfacing_protocols(protos.clone()), subs, granularity) {
            DataResult::OK{data: subscriptions} => Some(subscriptions),
            DataResult::ERROR{ .. } => None,
        };
        let composition: Option<SwarmProtocolType> = match compose_protocols(to_interfacing_protocols(protos.clone())) {
            DataResult::OK{data: composition} => Some(composition),
            DataResult::ERROR{ .. } => None,
        };
        assert!(subscriptions.is_some());
        assert!(composition.is_some());
        let subscriptions = subscriptions.unwrap();
        let composition = composition.unwrap();
        //let composition = InterfacingSwarms::<Role>(vec![CompositionComponent{protocol: composition.unwrap(), interface: None}]);
        let sub_string = serde_json::to_string(&subscriptions).unwrap();
        for role in subscriptions.keys() {
            let projection: Option<MachineType> = match revised_projection(composition.clone(), sub_string.clone(), role.clone(), true) {
                DataResult::OK{data: projection} => {
                Some(projection) },
                DataResult::ERROR{ .. } => None,
            };
            assert!(projection.is_some());
            // should work like this projecting over the explicit composition initially and comparing that with combined machines?
            match check_composed_projection(to_interfacing_protocols(protos.clone()), sub_string.clone(), role.clone(), projection.clone().unwrap()) {
                CheckResult::OK => {
                    let combined: Option<MachineType> = match project_combine(to_interfacing_protocols(protos.clone()), sub_string.clone(), role.clone(), false) {
                        DataResult::OK{data: combined} => {
                        Some(combined) },
                        DataResult::ERROR{ .. } => None,
                    };
                    println!("combined: num states: {}, num transitions: {}, num terminal: {}", num_states(&combined.clone().unwrap()), num_transitions(&combined.clone().unwrap()), num_terminal(&combined.clone().unwrap()));
                    println!("expanded: num states: {}, num transitions: {}, num terminal: {}", num_states(&projection.clone().unwrap()), num_transitions(&projection.clone().unwrap()), num_terminal(&projection.clone().unwrap()));
                    println!("|combined states| - |expanded states|: {}", num_states(&combined.clone().unwrap()) - num_states(&projection.clone().unwrap()));
                    println!("|combined states| - |expanded states| = |combined terminal| - |expanded terminal|: {}", (num_states(&combined.clone().unwrap()) - num_states(&projection.clone().unwrap())) == (num_terminal(&combined.clone().unwrap()) - num_terminal(&projection.clone().unwrap())));
                    println!("");
                },//(),
                CheckResult::ERROR {errors: e} => {
                    match project_combine(to_interfacing_protocols(protos.clone()), sub_string.clone(), role.clone(), false) {
                        DataResult::OK{data: projection1} => {
                            println!("machine combined: {}", serde_json::to_string_pretty::<MachineType>(&projection1).unwrap());
                        },
                        DataResult::ERROR{ errors: e } => println!("errors combined: {:?}", e),
                    };
                    println!("machine: {}", serde_json::to_string_pretty(&projection.unwrap()).unwrap());
                    /* println!("composition: {}", serde_json::to_string_pretty(&composition).unwrap());
                    for v in &vec.0 {
                        println!("component: {}", serde_json::to_string_pretty(&v.protocol).unwrap());
                    } */
                    println!("errors: {:?}", e); assert!(false)
                },
            }
        }
    }
}

proptest! {
    #[test]
    fn test_well_formed_from_23(protos in generate_interfacing_swarms_refinement_2(8, 8, 8)) {
        setup_logger();
        let protocols: Vec<SwarmProtocolType> = protos.0.into_iter().map(|component| component.protocol).collect();
        for proto in &protocols {
            let subs = serde_json::to_string(&BTreeMap::<Role, BTreeSet::<EventType>>::new()).unwrap();
            let subscription: Option<Subscriptions> = match well_formed_sub(proto.clone(), subs) {
                DataResult::OK{data: subscriptions} => Some(subscriptions),
                DataResult::ERROR{ .. } => None,
            };
            assert!(subscription.is_some());
            let subscription = subscription.unwrap();

            // Check that removing any event type from generated subscription makes wf-check ('Behavioural Types for Local-First Software'-definition) fail.
            for (role, event_types) in &subscription {
                for event_type in event_types {
                    let mut subscription_should_fail = subscription.clone();
                    subscription_should_fail.entry(role.clone()).and_modify(|ets| { ets.remove(event_type); });
                    let errors = check_swarm(proto.clone(), serde_json::to_string(&subscription_should_fail).unwrap());
                    let ok = match errors {
                        CheckResult::OK => true,
                        CheckResult::ERROR { .. } => false //{ println!("{:?}", e); false }
                    };
                    //println!("hihi in test_well_formed_from_23 inner inner inner loop");
                    assert!(!ok);
                }
            }
            // Check if generated subscription is well-formed according to the 'Behavioural Types for Local-First Software'-definition
            let subscription = serde_json::to_string(&subscription).unwrap();
            let errors = check_swarm(proto.clone(), subscription.clone());
            let ok = match errors {
                CheckResult::OK => true,
                CheckResult::ERROR { .. } => false
            };
            assert!(ok);
        }
    }
}