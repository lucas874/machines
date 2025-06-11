use crate::{
    composition::{
        types1::{
            projection::{to_option_machine, MachineGraph, OptionGraph},
            proto_info::ProtoStruct,
            util::{gen_state_name, EventLabel},
        },
        util::compose::compose,
    },
    types::{EventType, MachineLabel, Role, State, StateName, SwarmLabel},
    NodeId, Subscriptions,
};
use itertools::Itertools;
use petgraph::{
    graph::EdgeReference,
    visit::{EdgeFiltered, EdgeRef, IntoEdgeReferences, IntoEdgesDirected, IntoNodeReferences},
    Direction::{Incoming, Outgoing},
};
use std::collections::{BTreeMap, BTreeSet};
type ERef<'a> = <&'a crate::Graph as IntoEdgeReferences>::EdgeRef;

// Similar to machine::project, except that transitions with event types
// not subscribed to by role are skipped.
pub fn project(
    swarm: &crate::Graph,
    initial: NodeId,
    subs: &Subscriptions,
    role: Role,
    minimize: bool,
) -> (MachineGraph, NodeId) {
    let _span = tracing::info_span!("project", %role).entered();
    let mut machine = MachineGraph::new();
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
        let (dfa, dfa_initial) = nfa_to_dfa(machine, m_nodes[initial.index()]); // make deterministic. slight deviation from projection operation formally.
        minimal_machine(&dfa, dfa_initial) // when minimizing we get a machine that is a little different but equivalent to the one prescribed by the projection operator formally
    } else {
        (machine, m_nodes[initial.index()])
    }
}

// precondition: the protocols interfaces on the supplied interfaces.
// precondition: the composition of the protocols in swarms is wwf w.r.t. subs.
pub fn project_combine(
    swarms: &Vec<ProtoStruct>,
    subs: &Subscriptions,
    role: Role,
    minimize: bool,
) -> (OptionGraph, Option<NodeId>) {
    let _span = tracing::info_span!("project_combine", %role).entered();
    // check this anyway
    if swarms.is_empty()
        || !swarms[0].interface.is_empty()
        || swarms[0].initial.is_none()
        || swarms[1..]
            .iter()
            .any(|p| p.interface.is_empty() || p.initial.is_none())
    {
        return (OptionGraph::new(), None);
    }

    let mapper = |p: &ProtoStruct| -> (MachineGraph, NodeId, BTreeSet<EventType>) {
        let (projection, projection_initial) =
            project(&p.graph, p.initial.unwrap(), subs, role.clone(), minimize);
        (projection, projection_initial, p.interface.clone())
    };

    let projections: Vec<_> = swarms.into_iter().map(mapper).collect();

    let (combined_projection, combined_initial) = combine_projs(projections, gen_state_name);

    //let (combined_projection, combined_initial) = minimal_machine(&combined_projection, combined_initial);
    // option because used in equivalent. Consider changing.
    (
        to_option_machine(&combined_projection),
        Some(combined_initial),
    )
}

pub(in crate::composition) fn combine_projs<N: Clone, E: Clone + EventLabel>(
    projections: Vec<(petgraph::Graph<N, E>, NodeId, BTreeSet<EventType>)>,
    gen_node: fn(&N, &N) -> N,
) -> (petgraph::Graph<N, E>, NodeId) {
    let _span = tracing::info_span!("combine_projs").entered();
    let (acc_machine, acc_initial, _) = projections[0].clone();
    let (combined_projection, combined_initial) = projections[1..].to_vec().into_iter().fold(
        (acc_machine, acc_initial),
        |(acc, acc_i), (m, i, interface)| compose(acc, acc_i, m, i, interface, gen_node),
    );
    (combined_projection, combined_initial)
}

// nfa to dfa using subset construction. Hopcroft, Motwani and Ullman section 2.3.5
fn nfa_to_dfa(nfa: MachineGraph, i: NodeId) -> (MachineGraph, NodeId) {
    let _span = tracing::info_span!("nfa_to_dfa").entered();
    let mut dfa = MachineGraph::new();
    // maps vectors of NodeIds from the nfa to a NodeId in the new dfa
    let mut dfa_nodes: BTreeMap<BTreeSet<NodeId>, NodeId> = BTreeMap::new();

    // push to and pop from in loop until empty and NFA has been turned into a dfa
    let mut stack: Vec<BTreeSet<NodeId>> = Vec::new();

    // [0, 1, 2] becomes Some(State("{0, 1, 2}"))
    let state_name = |nodes: &BTreeSet<NodeId>| -> State {
        let name = format!("{{ {} }}", nodes.iter().map(|n| nfa[*n].clone()).join(", "));
        State::new(&name)
    };

    // get all outgoing edges of the sources. turn into a map from machine labels to vectors of target states.
    let outgoing_map = |srcs: &BTreeSet<NodeId>| -> BTreeMap<MachineLabel, BTreeSet<NodeId>> {
        srcs.iter()
            .flat_map(|src| {
                nfa.edges_directed(*src, Outgoing)
                    .map(|e| (e.weight().clone(), e.target()))
            })
            .collect::<BTreeSet<(MachineLabel, NodeId)>>()
            .into_iter()
            .fold(BTreeMap::new(), |mut m, (edge_label, target)| {
                m.entry(edge_label)
                    .and_modify(|v: &mut BTreeSet<NodeId>| {
                        v.insert(target);
                    })
                    .or_insert_with(|| BTreeSet::from([target]));
                m
            })
    };

    // add initial state to dfa
    dfa_nodes.insert(
        BTreeSet::from([i]),
        dfa.add_node(state_name(&BTreeSet::from([i]))),
    );
    // add initial state to stack
    stack.push(BTreeSet::from([i]));

    while let Some(states) = stack.pop() {
        let map = outgoing_map(&states);
        for edge in map.keys() {
            if !dfa_nodes.contains_key(&map[edge]) {
                stack.push(map[edge].clone());
            }
            let target: NodeId = *dfa_nodes
                .entry(map[edge].clone())
                .or_insert_with(|| dfa.add_node(state_name(&map[edge])));
            let src: NodeId = *dfa_nodes.get(&states).unwrap();
            dfa.add_edge(src, target, edge.clone());
        }
    }

    (dfa, dfa_nodes[&BTreeSet::from([i])])
}

fn minimal_machine(graph: &MachineGraph, i: NodeId) -> (MachineGraph, NodeId) {
    let _span = tracing::info_span!("minimal_machine").entered();
    let partition = partition_refinement(graph);
    let mut minimal = MachineGraph::new();
    let mut node_to_partition = BTreeMap::new();
    let mut partition_to_minimal_graph_node = BTreeMap::new();
    let mut edges = BTreeSet::new();
    let state_name = |nodes: &BTreeSet<NodeId>| -> State {
        let name = format!(
            "{{ {} }}",
            nodes.iter().map(|n| graph[*n].clone()).join(", ")
        );
        State::new(&name)
    };

    for n in graph.node_indices() {
        node_to_partition.insert(
            n,
            partition.iter().find(|block| block.contains(&n)).unwrap(),
        );
    }

    for block in &partition {
        partition_to_minimal_graph_node.insert(block, minimal.add_node(state_name(block)));
    }
    for node in graph.node_indices() {
        for edge in graph.edges_directed(node, Outgoing) {
            let source = partition_to_minimal_graph_node[node_to_partition[&node]];
            let target = partition_to_minimal_graph_node[node_to_partition[&edge.target()]];
            if !edges.contains(&(source, edge.weight().clone(), target)) {
                minimal.add_edge(source, target, edge.weight().clone());
                edges.insert((source, edge.weight().clone(), target));
            }
        }
    }
    let initial = partition_to_minimal_graph_node[node_to_partition[&i]];
    (minimal, initial)
}

fn partition_refinement(graph: &MachineGraph) -> BTreeSet<BTreeSet<NodeId>> {
    let _span = tracing::info_span!("partition_refinement").entered();
    let mut partition_old = BTreeSet::new();
    let tmp: (BTreeSet<_>, BTreeSet<_>) = graph
        .node_indices()
        .partition(|n| graph.edges_directed(*n, Outgoing).count() == 0);
    let mut partition: BTreeSet<BTreeSet<NodeId>> = BTreeSet::from([tmp.0, tmp.1]);

    let pre_labels = |block: &BTreeSet<NodeId>| -> BTreeSet<MachineLabel> {
        block
            .iter()
            .flat_map(|n| {
                graph
                    .edges_directed(*n, Incoming)
                    .map(|e| e.weight().clone())
            })
            .collect()
    };

    while partition.len() != partition_old.len() {
        partition_old = partition.clone();
        for superblock in &partition_old {
            for label in pre_labels(superblock) {
                partition = refine_partition(graph, partition, superblock, &label);
            }
        }
    }

    partition
}

fn refine_partition(
    graph: &MachineGraph,
    partition: BTreeSet<BTreeSet<NodeId>>,
    superblock: &BTreeSet<NodeId>,
    label: &MachineLabel,
) -> BTreeSet<BTreeSet<NodeId>> {
    partition
        .iter()
        .flat_map(|block| refine_block(graph, block, superblock, label))
        .collect()
}

fn refine_block(
    graph: &MachineGraph,
    block: &BTreeSet<NodeId>,
    superblock: &BTreeSet<NodeId>,
    label: &MachineLabel,
) -> BTreeSet<BTreeSet<NodeId>> {
    let predicate = |node: &NodeId| -> bool {
        graph
            .edges_directed(*node, Outgoing)
            .any(|e| *e.weight() == *label && superblock.contains(&e.target()))
    };

    let tmp: (BTreeSet<_>, BTreeSet<_>) = block.iter().partition(|n| predicate(n));

    BTreeSet::from([tmp.0, tmp.1])
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect()
}
