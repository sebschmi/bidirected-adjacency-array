use std::{collections::HashSet, hash::Hash};

use rand::Rng;
use tagged_vec::TaggedVec;
use thiserror::Error;

use crate::{
    graph::{BidirectedAdjacencyArray, BidirectedEdge},
    index::{GraphIndexInteger, NodeIndex},
};

#[derive(Debug, Error)]
pub enum RandomGraphError<IndexType: GraphIndexInteger, NodeData, EdgeData> {
    #[error(
        "the random edge generator repeatedly produced edges that were already present in the graph"
    )]
    RandomGenerationStalled(BidirectedAdjacencyArray<IndexType, NodeData, EdgeData>),
}

impl<IndexType: GraphIndexInteger, NodeData, EdgeData>
    BidirectedAdjacencyArray<IndexType, NodeData, EdgeData>
{
    /// Generates a random bidirected graph with the specified number of nodes and edges.
    ///
    /// If the edge generator repeatedly produces an edge that is already present in the graph,
    /// then the generation is aborted with an error.
    pub fn generate_random_graph<Random: Rng>(
        num_nodes: usize,
        num_edges: usize,
        mut node_data_generator: impl FnMut(NodeIndex<IndexType>, &mut Random) -> NodeData,
        mut edge_data_generator: impl FnMut(&mut Random) -> EdgeData,
        rng: &mut Random,
    ) -> Result<
        BidirectedAdjacencyArray<IndexType, NodeData, EdgeData>,
        RandomGraphError<IndexType, NodeData, EdgeData>,
    >
    where
        EdgeData: Eq + Hash,
    {
        let mut nodes = TaggedVec::with_capacity(num_nodes);
        for node_index in 0..num_nodes {
            nodes.push(node_data_generator(NodeIndex::from_usize(node_index), rng));
        }

        let mut edges = HashSet::new();
        let mut stall_counter = 0;

        while edges.len() < num_edges {
            let from = rng.random_range(0..num_nodes);
            let to = rng.random_range(0..num_nodes);
            let from_forward = rng.random_bool(0.5);
            let to_forward = rng.random_bool(0.5);

            let edge = BidirectedEdge {
                from: NodeIndex::from_usize(from),
                from_forward,
                to: NodeIndex::from_usize(to),
                to_forward,
                data: edge_data_generator(rng),
            };

            let was_modified = edges.insert(edge);
            if was_modified {
                stall_counter = 0;
            } else {
                stall_counter += 1;
                if stall_counter > 10 {
                    return Err(RandomGraphError::RandomGenerationStalled(
                        BidirectedAdjacencyArray::new(nodes, edges.into_iter().collect()),
                    ));
                }
            }
        }

        Ok(BidirectedAdjacencyArray::new(
            nodes,
            edges.into_iter().collect(),
        ))
    }
}
