use tagged_vec::TaggedVec;

use crate::graph::{BidirectedAdjacencyArray, BidirectedEdge};

#[test]
fn test_empty_construction() {
    let graph = BidirectedAdjacencyArray::<u8, (), ()>::new(TaggedVec::new(), TaggedVec::new());
    assert_eq!(graph.node_count(), 0);
    assert_eq!(graph.edge_count(), 0);
}

#[test]
fn test_single_node_no_edges_construction() {
    let nodes = TaggedVec::from(vec![()]);
    let edges = TaggedVec::new();
    let graph = BidirectedAdjacencyArray::<u8, (), ()>::new(nodes, edges);
    assert_eq!(graph.node_count(), 1);
    assert_eq!(graph.edge_count(), 0);
}

#[test]
fn test_path_construction() {
    let nodes = vec![(), (), ()];
    let edges = vec![
        BidirectedEdge {
            from: 0.into(),
            from_forward: true,
            to: 1.into(),
            to_forward: true,
            data: (),
        },
        BidirectedEdge {
            from: 1.into(),
            from_forward: true,
            to: 2.into(),
            to_forward: true,
            data: (),
        },
    ];
    let graph = BidirectedAdjacencyArray::<u8, (), ()>::new(nodes.into(), edges.into());
    assert_eq!(graph.node_count(), 3);
    assert_eq!(graph.edge_count(), 2);

    assert_eq!(
        graph
            .iter_successors(0.into())
            .map(|(_, node_index)| node_index)
            .collect::<Vec<_>>(),
        vec![(2.into())]
    );
    assert_eq!(
        graph
            .iter_successors(1.into())
            .map(|(_, node_index)| node_index)
            .collect::<Vec<_>>(),
        vec![]
    );
    assert_eq!(
        graph
            .iter_successors(2.into())
            .map(|(_, node_index)| node_index)
            .collect::<Vec<_>>(),
        vec![(4.into())]
    );
    assert_eq!(
        graph
            .iter_successors(3.into())
            .map(|(_, node_index)| node_index)
            .collect::<Vec<_>>(),
        vec![(1.into())]
    );
    assert_eq!(
        graph
            .iter_successors(4.into())
            .map(|(_, node_index)| node_index)
            .collect::<Vec<_>>(),
        vec![]
    );
    assert_eq!(
        graph
            .iter_successors(5.into())
            .map(|(_, node_index)| node_index)
            .collect::<Vec<_>>(),
        vec![(3.into())]
    );
}
