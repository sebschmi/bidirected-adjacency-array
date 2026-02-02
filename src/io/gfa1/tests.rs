use rand::{
    SeedableRng,
    distr::{SampleString, slice::Choose},
    rngs::SmallRng,
};

use crate::{
    graph::{BidirectedAdjacencyArray, BidirectedEdge},
    io::gfa1::{PlainGfaEdgeData, PlainGfaNodeData, read_gfa1, write_gfa1},
};

#[test]
fn test_write_read_triangle() {
    let nodes = vec![
        PlainGfaNodeData {
            name: "N0".into(),
            sequence: "000".into(),
        },
        PlainGfaNodeData {
            name: "N1".into(),
            sequence: "111".into(),
        },
        PlainGfaNodeData {
            name: "N2".into(),
            sequence: "222".into(),
        },
    ];
    let edges = vec![
        BidirectedEdge {
            from: 0.into(),
            from_forward: true,
            to: 1.into(),
            to_forward: true,
            data: PlainGfaEdgeData { overlap: 0 },
        },
        BidirectedEdge {
            from: 1.into(),
            from_forward: true,
            to: 2.into(),
            to_forward: true,
            data: PlainGfaEdgeData { overlap: 1 },
        },
        BidirectedEdge {
            from: 2.into(),
            from_forward: true,
            to: 0.into(),
            to_forward: true,
            data: PlainGfaEdgeData { overlap: 2 },
        },
    ];

    let expected_graph = BidirectedAdjacencyArray::<u16, _, _>::new(nodes.into(), edges.into());

    let mut buffer = Vec::new();
    write_gfa1(&expected_graph, &mut buffer).unwrap();
    let actual_gfa = std::str::from_utf8(&buffer).unwrap().trim();
    println!("GFA:\n{}", std::str::from_utf8(&buffer).unwrap());
    let actual_graph = read_gfa1::<u16>(&mut buffer.as_slice()).unwrap();

    let expected_gfa = "H\tVN:Z:1.0\nS\tN0\t000\nS\tN1\t111\nS\tN2\t222\nL\tN0\t+\tN1\t+\t0M\nL\tN1\t+\tN2\t+\t1M\nL\tN2\t+\tN0\t+\t2M";

    expected_graph.expect_equal(&actual_graph);
    assert_eq!(expected_gfa, actual_gfa);
}

#[test]
fn test_write_read_large() {
    let mut rng = SmallRng::seed_from_u64(0);
    let dna_characters = Choose::new(&['A', 'C', 'G', 'T']).unwrap();

    for _ in 0..1000 {
        let expected_graph = BidirectedAdjacencyArray::<u16, _, _>::generate_random_graph(
            10,
            100,
            |node_index, rng| PlainGfaNodeData {
                name: format!("node{node_index}"),
                sequence: dna_characters.sample_string(rng, 10),
            },
            |_| PlainGfaEdgeData { overlap: 0 },
            &mut rng,
        )
        .unwrap();

        let mut buffer = Vec::new();
        write_gfa1(&expected_graph, &mut buffer).unwrap();
        let actual_graph = read_gfa1::<u16>(&mut buffer.as_slice()).unwrap();

        expected_graph.expect_equal(&actual_graph);
    }
}
