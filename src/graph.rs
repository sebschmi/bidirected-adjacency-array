use tagged_vec::TaggedVec;

use crate::index::{
    DirectedEdgeIndex, DirectedNodeIndex, EdgeIndex, GraphIndexInteger, NodeIndex,
    OptionalEdgeIndex,
};

pub struct BidirectedAdjacencyArray<IndexType, NodeData, EdgeData> {
    /// Maps directed nodes to their edge lists.
    ///
    /// Each bidirected node is represented by two consecutive directed nodes.
    /// The forward side is identified by [`DirectedNodeIndex::is_forward`].
    node_array: TaggedVec<DirectedNodeIndex<IndexType>, DirectedEdgeIndex<IndexType>>,

    /// The edge lists for all directed nodes.
    ///
    /// Each bidirected edge is represented by two reverse-complemental directed edges.
    /// Even ++ and -- self loops are represented by two distinct but same directed edges.
    edge_array: TaggedVec<DirectedEdgeIndex<IndexType>, DirectedNodeIndex<IndexType>>,

    /// Data associated with the nodes.
    ///
    /// Since each bidirected node is represented by two directed nodes,
    /// the data for both directed nodes is stored at the same index.
    /// Hence, the data of a directed node `n` is stored at index `n / 2`.
    node_data: TaggedVec<NodeIndex<IndexType>, NodeData>,

    /// Keys for finding the data associated with the edges.
    ///
    /// Each bidirected edge is represented by two directed edges,
    /// and both directed edges share the same data.
    /// However, we treat one directed edge as the "forward" direction and the other as the "reverse" direction.
    /// We only store the data for the "forward" direction.
    edge_data_keys: TaggedVec<DirectedEdgeIndex<IndexType>, EdgeDataKey<IndexType>>,

    /// The actual edge data.
    ///
    /// This should be accessed via the `edge_data_keys`.
    edge_data: TaggedVec<EdgeIndex<IndexType>, BidirectedEdgeData<IndexType, EdgeData>>,
}

struct EdgeDataKey<IndexType> {
    inverse: DirectedEdgeIndex<IndexType>,
    data_index: OptionalEdgeIndex<IndexType>,
}

struct BidirectedEdgeData<IndexType, EdgeData> {
    forward: DirectedEdgeIndex<IndexType>,
    reverse: DirectedEdgeIndex<IndexType>,
    data: EdgeData,
}

pub struct DirectedEdgeDataView<'a, EdgeData> {
    forward: bool,
    data: &'a EdgeData,
}

pub struct EdgeDataView<'a, IndexType, EdgeData> {
    from: DirectedNodeIndex<IndexType>,
    to: DirectedNodeIndex<IndexType>,
    data: &'a EdgeData,
}

pub struct BidirectedEdge<IndexType, EdgeData> {
    pub from: NodeIndex<IndexType>,
    pub from_forward: bool,
    pub to: NodeIndex<IndexType>,
    pub to_forward: bool,
    pub data: EdgeData,
}

impl<IndexType: GraphIndexInteger, NodeData, EdgeData>
    BidirectedAdjacencyArray<IndexType, NodeData, EdgeData>
{
    pub fn new(
        nodes: TaggedVec<NodeIndex<IndexType>, NodeData>,
        edges: TaggedVec<EdgeIndex<IndexType>, BidirectedEdge<IndexType, EdgeData>>,
    ) -> Self {
        let mut node_array =
            TaggedVec::<DirectedNodeIndex<IndexType>, DirectedEdgeIndex<IndexType>>::new();
        let mut edge_array =
            TaggedVec::<DirectedEdgeIndex<IndexType>, DirectedNodeIndex<IndexType>>::new();

        Self {
            node_array,
            edge_array,
            node_data: nodes,
            edge_data_keys: todo!(),
            edge_data: todo!(),
        }
    }

    pub fn node_count(&self) -> usize {
        self.node_data.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edge_data.len()
    }

    pub fn iter_nodes(&self) -> impl Iterator<Item = NodeIndex<IndexType>> {
        self.node_data.iter_indices()
    }

    pub fn iter_edges(&self) -> impl Iterator<Item = EdgeIndex<IndexType>> {
        self.edge_data.iter_indices()
    }

    pub fn node_data(&self, node: NodeIndex<IndexType>) -> &NodeData {
        &self.node_data[node]
    }

    pub fn edge_data(&self, edge: EdgeIndex<IndexType>) -> EdgeDataView<'_, IndexType, EdgeData> {
        let bidirected_edge_data = &self.edge_data[edge];

        let forward_to = self.edge_array[bidirected_edge_data.forward];
        let reverse_to = self.edge_array[bidirected_edge_data.reverse];

        if forward_to == reverse_to {
            // ++ or -- self loop case: both directed edges go from node to its reverse.
            let from = forward_to.invert();
            let to = forward_to;
            EdgeDataView {
                from,
                to,
                data: &bidirected_edge_data.data,
            }
        } else if forward_to.invert() == reverse_to {
            // +- or -+ self loop case: directed edges are self loops.
            let from = forward_to;
            let to = forward_to;
            EdgeDataView {
                from,
                to,
                data: &bidirected_edge_data.data,
            }
        } else {
            // Normal case: directed edges go between two different nodes.
            let from = reverse_to.invert();
            let to = forward_to;
            EdgeDataView {
                from,
                to,
                data: &bidirected_edge_data.data,
            }
        }
    }

    pub fn directed_edge_data<'this>(
        &'this self,
        edge: DirectedEdgeIndex<IndexType>,
    ) -> DirectedEdgeDataView<'this, EdgeData> {
        let key = &self.edge_data_keys[edge];
        if let Some(data_index) = Option::<EdgeIndex<IndexType>>::from(key.data_index) {
            DirectedEdgeDataView {
                forward: true,
                data: &self.edge_data[data_index].data,
            }
        } else {
            let inverse_key = &self.edge_data_keys[key.inverse];
            let Some(inverse_data_index) =
                Option::<EdgeIndex<IndexType>>::from(inverse_key.data_index)
            else {
                panic!(
                    "Edge data for edge {:?} and its inverse {:?} are both missing",
                    edge, key.inverse
                );
            };
            DirectedEdgeDataView {
                forward: false,
                data: &self.edge_data[inverse_data_index].data,
            }
        }
    }
}

impl<'a, EdgeData> DirectedEdgeDataView<'a, EdgeData> {
    pub fn is_forward(&self) -> bool {
        self.forward
    }

    pub fn data(&self) -> &EdgeData {
        self.data
    }
}

impl<'a, IndexType, EdgeData> EdgeDataView<'a, IndexType, EdgeData> {
    pub fn from(&self) -> DirectedNodeIndex<IndexType>
    where
        IndexType: Copy,
    {
        self.from
    }

    pub fn to(&self) -> DirectedNodeIndex<IndexType>
    where
        IndexType: Copy,
    {
        self.to
    }

    pub fn data(&self) -> &EdgeData {
        self.data
    }
}
