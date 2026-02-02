use thiserror::Error;

use crate::{
    graph::BidirectedAdjacencyArray,
    index::{EdgeIndex, GraphIndexInteger, NodeIndex},
};

#[derive(Debug, Error)]
pub enum GraphComparisonError<IndexType> {
    #[error("the number of nodes in the graphs differ")]
    NodeCountMismatch,

    #[error("the number of edges in the graphs differ")]
    EdgeCountMismatch,

    #[error("node data mismatch at node index {0}")]
    NodeDataMismatch(NodeIndex<IndexType>),

    #[error("edge data mismatch at edge index {0}")]
    EdgeDataMismatch(EdgeIndex<IndexType>),

    #[error("edge endpoint mismatch at edge index {0}")]
    EdgeEndpointMismatch(EdgeIndex<IndexType>),
}

impl<IndexType: GraphIndexInteger, NodeData, EdgeData>
    BidirectedAdjacencyArray<IndexType, NodeData, EdgeData>
{
    /// Compares this graph to another graph.
    ///
    /// The comparison returns `Ok` if all nodes and edges are identical in both graphs.
    /// Otherwise, it returns an `Err` describing the differences.
    pub fn compare(&self, other: &Self) -> Result<(), GraphComparisonError<IndexType>>
    where
        NodeData: Eq,
        EdgeData: Eq,
    {
        if self.node_count() != other.node_count() {
            return Err(GraphComparisonError::NodeCountMismatch);
        }

        if self.edge_count() != other.edge_count() {
            return Err(GraphComparisonError::EdgeCountMismatch);
        }

        for node_index in self.iter_nodes() {
            let self_node_data = self.node_data(node_index);
            let other_node_data = other.node_data(node_index);

            if self_node_data != other_node_data {
                return Err(GraphComparisonError::NodeDataMismatch(node_index));
            }
        }

        for edge_index in self.iter_edges() {
            let self_edge = self.edge(edge_index);
            let other_edge = other.edge(edge_index);

            if self_edge.data() != other_edge.data() {
                return Err(GraphComparisonError::EdgeDataMismatch(edge_index));
            }

            if self_edge.from() != other_edge.from() || self_edge.to() != other_edge.to() {
                return Err(GraphComparisonError::EdgeEndpointMismatch(edge_index));
            }
        }

        Ok(())
    }

    #[cfg(test)]
    pub fn expect_equal(&self, other: &Self)
    where
        NodeData: Eq,
        EdgeData: Eq,
    {
        match self.compare(other) {
            Ok(()) => { /* All good. */ }
            Err(error @ GraphComparisonError::EdgeEndpointMismatch(edge_index)) => {
                let expected_edge = self.edge(edge_index);
                let actual_edge = other.edge(edge_index);

                let expected_from = expected_edge.from();
                let expected_to = expected_edge.to();
                let actual_from = actual_edge.from();
                let actual_to = actual_edge.to();

                panic!(
                    "{error}:\nExpected ({}, {})\nActual   ({}, {})",
                    expected_from, expected_to, actual_from, actual_to,
                );
            }
            Err(e) => panic!("Graphs differ: {:?}", e),
        }
    }
}
