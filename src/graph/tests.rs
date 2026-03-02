use itertools::Itertools;
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
            .iter_outgoing_edges(0.into())
            .map(|edge| edge.to())
            .collect::<Vec<_>>(),
        vec![(2.into())]
    );
    assert_eq!(
        graph
            .iter_outgoing_edges(1.into())
            .map(|edge| edge.to())
            .collect::<Vec<_>>(),
        vec![]
    );
    assert_eq!(
        graph
            .iter_outgoing_edges(2.into())
            .map(|edge| edge.to())
            .collect::<Vec<_>>(),
        vec![(4.into())]
    );
    assert_eq!(
        graph
            .iter_outgoing_edges(3.into())
            .map(|edge| edge.to())
            .collect::<Vec<_>>(),
        vec![(1.into())]
    );
    assert_eq!(
        graph
            .iter_outgoing_edges(4.into())
            .map(|edge| edge.to())
            .collect::<Vec<_>>(),
        vec![]
    );
    assert_eq!(
        graph
            .iter_outgoing_edges(5.into())
            .map(|edge| edge.to())
            .collect::<Vec<_>>(),
        vec![(3.into())]
    );
}

#[test]
fn test_remove_multiedges() {
    let nodes = vec![(), (), (), ()];
    let edges = vec![
        BidirectedEdge {
            from: 0.into(),
            from_forward: true,
            to: 1.into(),
            to_forward: true,
            data: 1,
        },
        BidirectedEdge {
            from: 1.into(),
            from_forward: false,
            to: 0.into(),
            to_forward: false,
            data: 2,
        },
        BidirectedEdge {
            from: 1.into(),
            from_forward: true,
            to: 2.into(),
            to_forward: true,
            data: 3,
        },
        BidirectedEdge {
            from: 1.into(),
            from_forward: false,
            to: 2.into(),
            to_forward: true,
            data: 4,
        },
        BidirectedEdge {
            from: 2.into(),
            from_forward: true,
            to: 3.into(),
            to_forward: true,
            data: 5,
        },
        BidirectedEdge {
            from: 2.into(),
            from_forward: true,
            to: 3.into(),
            to_forward: true,
            data: 6,
        },
    ];
    let mut graph = BidirectedAdjacencyArray::<u8, (), usize>::new(nodes.into(), edges.into());

    assert_eq!(
        graph
            .iter_outgoing_edges(0.into())
            .map(|edge| (edge.to, *graph.directed_edge_data(edge.index).data))
            .unique()
            .sorted()
            .collect_vec(),
        vec![(2.into(), 1), (2.into(), 2)],
    );
    assert_eq!(
        graph
            .iter_outgoing_edges(1.into())
            .map(|edge| (edge.to, *graph.directed_edge_data(edge.index).data))
            .unique()
            .sorted()
            .collect_vec(),
        vec![],
    );
    assert_eq!(
        graph
            .iter_outgoing_edges(2.into())
            .map(|edge| (edge.to, *graph.directed_edge_data(edge.index).data))
            .unique()
            .sorted()
            .collect_vec(),
        vec![(4.into(), 3),],
    );
    assert_eq!(
        graph
            .iter_outgoing_edges(3.into())
            .map(|edge| (edge.to, *graph.directed_edge_data(edge.index).data))
            .unique()
            .sorted()
            .collect_vec(),
        vec![(1.into(), 1), (1.into(), 2), (4.into(), 4)],
    );
    assert_eq!(
        graph
            .iter_outgoing_edges(4.into())
            .map(|edge| (edge.to, *graph.directed_edge_data(edge.index).data))
            .unique()
            .sorted()
            .collect_vec(),
        vec![(6.into(), 5), (6.into(), 6)],
    );
    assert_eq!(
        graph
            .iter_outgoing_edges(5.into())
            .map(|edge| (edge.to, *graph.directed_edge_data(edge.index).data))
            .unique()
            .sorted()
            .collect_vec(),
        vec![(2.into(), 4), (3.into(), 3)],
    );
    assert_eq!(
        graph
            .iter_outgoing_edges(6.into())
            .map(|edge| (edge.to, *graph.directed_edge_data(edge.index).data))
            .unique()
            .sorted()
            .collect_vec(),
        vec![],
    );
    assert_eq!(
        graph
            .iter_outgoing_edges(7.into())
            .map(|edge| (edge.to, *graph.directed_edge_data(edge.index).data))
            .unique()
            .sorted()
            .collect_vec(),
        vec![(5.into(), 5), (5.into(), 6)],
    );

    assert_eq!(
        graph
            .iter_incident_edges(0.into())
            .map(|edge| *graph.edge(edge).data())
            .sorted()
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert_eq!(
        graph
            .iter_incident_edges(1.into())
            .map(|edge| *graph.edge(edge).data())
            .sorted()
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4]
    );
    assert_eq!(
        graph
            .iter_incident_edges(2.into())
            .map(|edge| *graph.edge(edge).data())
            .sorted()
            .collect::<Vec<_>>(),
        vec![3, 4, 5, 6]
    );
    assert_eq!(
        graph
            .iter_incident_edges(3.into())
            .map(|edge| *graph.edge(edge).data())
            .sorted()
            .collect::<Vec<_>>(),
        vec![5, 6]
    );

    let removed = graph.remove_multiedges();
    assert_eq!(removed.len(), 2, "removed: {removed:?}");
    assert!(removed.contains(&0.into()) ^ removed.contains(&1.into()));
    assert!(removed.contains(&4.into()) ^ removed.contains(&5.into()));
    assert_eq!(graph.edge_count(), 4);

    assert_eq!(
        graph
            .iter_outgoing_edges(0.into())
            .map(|edge| edge.to)
            .unique()
            .sorted()
            .collect_vec(),
        vec![2.into()],
    );
    assert_eq!(
        graph
            .iter_outgoing_edges(1.into())
            .map(|edge| edge.to)
            .unique()
            .sorted()
            .collect_vec(),
        vec![],
    );
    assert_eq!(
        graph
            .iter_outgoing_edges(2.into())
            .map(|edge| edge.to)
            .unique()
            .sorted()
            .collect_vec(),
        vec![4.into()],
    );
    assert_eq!(
        graph
            .iter_outgoing_edges(3.into())
            .map(|edge| edge.to)
            .unique()
            .sorted()
            .collect_vec(),
        vec![1.into(), 4.into()],
    );
    assert_eq!(
        graph
            .iter_outgoing_edges(4.into())
            .map(|edge| edge.to)
            .unique()
            .sorted()
            .collect_vec(),
        vec![6.into()],
    );
    assert_eq!(
        graph
            .iter_outgoing_edges(5.into())
            .map(|edge| edge.to)
            .unique()
            .sorted()
            .collect_vec(),
        vec![2.into(), 3.into()],
    );
    assert_eq!(
        graph
            .iter_outgoing_edges(6.into())
            .map(|edge| edge.to)
            .unique()
            .sorted()
            .collect_vec(),
        vec![],
    );
    assert_eq!(
        graph
            .iter_outgoing_edges(7.into())
            .map(|edge| edge.to)
            .unique()
            .sorted()
            .collect_vec(),
        vec![5.into()],
    );
}
