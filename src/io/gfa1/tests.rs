use std::iter;

use rand::{
    RngExt, SeedableRng,
    distr::{SampleString, slice::Choose},
    rngs::SmallRng,
};

use crate::{
    graph::{BidirectedAdjacencyArray, BidirectedEdge},
    io::gfa1::{PlainGfaEdgeData, PlainGfaNodeData},
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
    expected_graph.write_gfa1(&mut buffer).unwrap();
    let actual_gfa = std::str::from_utf8(&buffer).unwrap().trim();
    println!("GFA:\n{}", std::str::from_utf8(&buffer).unwrap());
    let actual_graph =
        BidirectedAdjacencyArray::<u16, PlainGfaNodeData, PlainGfaEdgeData>::read_gfa1(
            &mut buffer.as_slice(),
        )
        .unwrap();

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
        expected_graph.write_gfa1(&mut buffer).unwrap();
        let actual_graph =
            BidirectedAdjacencyArray::<u16, PlainGfaNodeData, PlainGfaEdgeData>::read_gfa1(
                &mut buffer.as_slice(),
            )
            .unwrap();

        expected_graph.expect_equal(&actual_graph);
    }
}

#[test]
fn test_read_random_order_lines() {
    let mut rng = SmallRng::seed_from_u64(0);
    let dna_characters = Choose::new(&['A', 'C', 'G', 'T']).unwrap();

    let expected_graph = BidirectedAdjacencyArray::<u16, _, _>::generate_random_graph(
        4,
        8,
        |node_index, rng| PlainGfaNodeData {
            name: format!("node{node_index}"),
            sequence: dna_characters.sample_string(rng, 10),
        },
        |_| PlainGfaEdgeData { overlap: 0 },
        &mut rng,
    )
    .unwrap();

    let mut buffer = Vec::new();
    expected_graph.write_gfa1(&mut buffer).unwrap();

    let lines: Vec<_> = str::from_utf8(&buffer).unwrap().lines().collect();
    let first_l_line_index = lines.iter().position(|line| line.starts_with('L')).unwrap();
    let mut s_lines: Vec<_> = lines[1..first_l_line_index].iter().copied().rev().collect();
    let mut l_lines: Vec<_> = lines[first_l_line_index..].iter().copied().rev().collect();

    let mut lines = vec![lines[0]]; // Keep header line at the beginning.
    while s_lines.len() + l_lines.len() > 0 {
        if rng.random_ratio(
            s_lines.len().try_into().unwrap(),
            (s_lines.len() + l_lines.len()).try_into().unwrap(),
        ) {
            lines.push(s_lines.pop().unwrap());
        } else {
            lines.push(l_lines.pop().unwrap());
        }
    }

    assert!(s_lines.is_empty());
    assert!(l_lines.is_empty());
    let first_l_line_index = lines.iter().position(|line| line.starts_with('L')).unwrap();
    let last_s_line_index = lines
        .iter()
        .rposition(|line| line.starts_with('S'))
        .unwrap();
    assert!(first_l_line_index < last_s_line_index, "Test setup failure");

    let buffer = lines.join("\n").into_bytes();

    let actual_graph =
        BidirectedAdjacencyArray::<u16, PlainGfaNodeData, PlainGfaEdgeData>::read_gfa1(
            &mut buffer.as_slice(),
        )
        .unwrap();

    expected_graph.expect_equal(&actual_graph);
}

#[test]
fn test_read_inverse_order_lines() {
    let mut rng = SmallRng::seed_from_u64(0);
    let dna_characters = Choose::new(&['A', 'C', 'G', 'T']).unwrap();

    let expected_graph = BidirectedAdjacencyArray::<u16, _, _>::generate_random_graph(
        4,
        8,
        |node_index, rng| PlainGfaNodeData {
            name: format!("node{node_index}"),
            sequence: dna_characters.sample_string(rng, 10),
        },
        |_| PlainGfaEdgeData { overlap: 0 },
        &mut rng,
    )
    .unwrap();

    let mut buffer = Vec::new();
    expected_graph.write_gfa1(&mut buffer).unwrap();

    let lines: Vec<_> = str::from_utf8(&buffer).unwrap().lines().collect();
    let first_l_line_index = lines.iter().position(|line| line.starts_with('L')).unwrap();
    let lines = iter::once(&lines[0]) // Keep header line at the beginning
        .chain(lines[first_l_line_index..].iter()) // L lines
        .chain(lines[1..first_l_line_index].iter()) // S lines
        .copied()
        .collect::<Vec<_>>();

    let buffer = lines.join("\n").into_bytes();

    let actual_graph =
        BidirectedAdjacencyArray::<u16, PlainGfaNodeData, PlainGfaEdgeData>::read_gfa1(
            &mut buffer.as_slice(),
        )
        .unwrap();

    expected_graph.expect_equal(&actual_graph);
}
