use crate::types::{
    proto_graph::NodeId,
    typescript_types::{EventLabel, EventType, StateName},
};
use petgraph::{Direction::Outgoing, graph::EdgeReference, visit::EdgeRef};
use std::collections::{BTreeMap, BTreeSet};

// Compose graph1 and graph2 (representing either protocols or machines)
// synchronizing on the event types in interface.
pub(crate) fn compose<N, E: EventLabel>(
    graph1: petgraph::Graph<N, E>,
    initial1: NodeId,
    graph2: petgraph::Graph<N, E>,
    initial2: NodeId,
    interface: BTreeSet<EventType>,
    gen_node: fn(&N, &N) -> N,
) -> (petgraph::Graph<N, E>, NodeId) {
    let _span = tracing::info_span!("compose").entered();
    let mut graph = petgraph::Graph::<N, E>::new();
    let mut node_map: BTreeMap<(NodeId, NodeId), NodeId> = BTreeMap::new();

    let weight_target_mapper = |e: EdgeReference<'_, E>| (e.weight().clone(), e.target());

    let outgoing_map = |m: &petgraph::Graph<N, E>, src: NodeId| -> BTreeMap<E, NodeId> {
        m.edges_directed(src, Outgoing)
            .map(weight_target_mapper)
            .collect()
    };

    // take the outgoing edges of a node an split into two vectors: one for the edges involving interfacing events and one for the edges that do not
    let partitioned = |g: &petgraph::Graph<N, E>, node: NodeId| -> (Vec<E>, Vec<E>) {
        g.edges_directed(node, Outgoing)
            .map(|e| e.weight().clone())
            .partition(|e| interface.contains(&e.get_event_type()))
    };

    let outgoing_to_visit = |g1: &petgraph::Graph<N, E>,
                             s1: NodeId,
                             g2: &petgraph::Graph<N, E>,
                             s2: NodeId|
     -> Vec<E> {
        let (interfacing1, non_interfacing1) = partitioned(g1, s1);
        let (interfacing2, non_interfacing2) = partitioned(g2, s2);

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

    let combined_initial = graph.add_node(gen_node(&graph1[initial1], &graph2[initial2]));
    node_map.insert((initial1, initial2), combined_initial);
    let mut worklist = vec![(combined_initial, (initial1, initial2))];

    while let Some((src, (old_src1, old_src2))) = worklist.pop() {
        let map1 = outgoing_map(&graph1, old_src1);
        let map2 = outgoing_map(&graph2, old_src2);
        let outgoing_edges = outgoing_to_visit(&graph1, old_src1, &graph2, old_src2);

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
                graph.add_edge(src, *dst, e);
            } else {
                let new_dst = graph.add_node(gen_node(&graph1[dst1], &graph2[dst2]));
                graph.add_edge(src, new_dst, e);
                node_map.insert((dst1, dst2), new_dst);
                worklist.push((new_dst, (dst1, dst2)));
            }
        }
    }

    (graph, combined_initial)
}

pub(crate) fn gen_state_name<N: StateName + From<String>>(n1: &N, n2: &N) -> N {
    let name = format!("{} || {}", n1.state_name(), n2.state_name());
    N::from(name)
}
