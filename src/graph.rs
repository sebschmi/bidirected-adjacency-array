use std::{collections::BTreeSet, iter};

use bitvec::vec::BitVec;
use permutation::Permutation;
use rustc_hash::FxHashSet;
use tagged_vec::TaggedVec;

use crate::{
    index::{
        DirectedEdgeIndex, DirectedNodeIndex, EdgeIndex, GraphIndexInteger, NodeIndex,
        OptionalEdgeIndex,
    },
    io::gfa1::PlainGfaEdgeData,
};

static DEBUG_REMOVE_MULTIEDGES: bool = false;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BidirectedAdjacencyArray<IndexType: GraphIndexInteger, NodeData, EdgeData> {
    /// Maps directed nodes to their edge lists.
    ///
    /// Each bidirected node is represented by two consecutive directed nodes.
    /// The forward side is identified by [`DirectedNodeIndex::is_forward`].
    ///
    /// The last element is a sentinel value to simplify edge list iteration.
    pub(crate) node_array: TaggedVec<DirectedNodeIndex<IndexType>, DirectedEdgeIndex<IndexType>>,

    /// The edge lists for all directed nodes.
    ///
    /// Each bidirected edge is represented by two reverse-complemental directed edges.
    /// Even ++ and -- self loops are represented by two distinct but same directed edges.
    pub(crate) edge_array: TaggedVec<DirectedEdgeIndex<IndexType>, DirectedNodeIndex<IndexType>>,

    /// Data associated with the nodes.
    ///
    /// Since each bidirected node is represented by two directed nodes,
    /// the data for both directed nodes is stored at the same index.
    /// Hence, the data of a directed node `n` is stored at index `n / 2`.
    pub(crate) node_data: TaggedVec<NodeIndex<IndexType>, NodeData>,

    /// Keys for finding the data associated with the edges.
    ///
    /// Each bidirected edge is represented by two directed edges,
    /// and both directed edges share the same data.
    /// However, we treat one directed edge as the "forward" direction and the other as the "reverse" direction.
    /// We only store the data for the "forward" direction.
    pub(crate) edge_data_keys: TaggedVec<DirectedEdgeIndex<IndexType>, EdgeDataKey<IndexType>>,

    /// The actual edge data.
    ///
    /// This should be accessed via the `edge_data_keys`.
    pub(crate) edge_data: TaggedVec<EdgeIndex<IndexType>, BidirectedEdgeData<IndexType, EdgeData>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct EdgeDataKey<IndexType: GraphIndexInteger> {
    inverse: DirectedEdgeIndex<IndexType>,
    data_index: OptionalEdgeIndex<IndexType>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct BidirectedEdgeData<IndexType, EdgeData> {
    forward: DirectedEdgeIndex<IndexType>,
    reverse: DirectedEdgeIndex<IndexType>,
    data: EdgeData,
}

pub struct DirectedEdge<IndexType> {
    from: DirectedNodeIndex<IndexType>,
    to: DirectedNodeIndex<IndexType>,
    index: DirectedEdgeIndex<IndexType>,
}

pub struct DirectedEdgeDataView<'a, IndexType, EdgeData> {
    is_forward: bool,
    edge: EdgeIndex<IndexType>,
    data: &'a EdgeData,
}

pub struct EdgeView<'a, IndexType, EdgeData> {
    from: DirectedNodeIndex<IndexType>,
    to: DirectedNodeIndex<IndexType>,
    forward: DirectedEdgeIndex<IndexType>,
    reverse: DirectedEdgeIndex<IndexType>,
    data: &'a EdgeData,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct BidirectedEdge<IndexType, EdgeData> {
    pub from: NodeIndex<IndexType>,
    /// True if this edge originates from the forward side of the `from` node.
    pub from_forward: bool,
    pub to: NodeIndex<IndexType>,
    /// True if this edge terminates at the forward side of the `to` node.
    pub to_forward: bool,
    pub data: EdgeData,
}

impl<IndexType: GraphIndexInteger, NodeData, EdgeData>
    BidirectedAdjacencyArray<IndexType, NodeData, EdgeData>
{
    /// Creates a bidirected adjacency array with the given bidirected nodes and edges.
    ///
    /// # Example
    ///
    /// ```rust
    /// use bidirected_adjacency_array::graph::BidirectedAdjacencyArray;
    /// use bidirected_adjacency_array::index::{NodeIndex, EdgeIndex};
    /// use bidirected_adjacency_array::graph::BidirectedEdge;
    /// use tagged_vec::TaggedVec;
    ///
    /// let nodes = TaggedVec::from(vec!['A', 'B', 'C']);
    /// let edges = TaggedVec::from(vec![
    ///         BidirectedEdge::new(NodeIndex::from_usize(0).into_directed_forward(), NodeIndex::from_usize(1).into_directed_forward(), 1), // A+ -> B+
    ///         BidirectedEdge::new(NodeIndex::from_usize(1).into_directed_reverse(), NodeIndex::from_usize(2).into_directed_forward(), 2), // B- -> C+
    /// ]);
    /// let graph = BidirectedAdjacencyArray::<u8, _, _>::new(nodes, edges);
    /// ```
    pub fn new(
        nodes: TaggedVec<NodeIndex<IndexType>, NodeData>,
        edges: TaggedVec<EdgeIndex<IndexType>, BidirectedEdge<IndexType, EdgeData>>,
    ) -> Self {
        let mut node_array = TaggedVec::from_iter(iter::repeat_n(
            DirectedEdgeIndex::from_usize(0),
            nodes.len() * 2 + 1,
        ));

        // Count the number of outgoing edges for each directed node.
        for edge in edges.iter_values() {
            let from_directed_forward =
                DirectedNodeIndex::from_bidirected(edge.from, edge.from_forward);
            node_array[from_directed_forward].increment();
            let from_directed_reverse =
                DirectedNodeIndex::from_bidirected(edge.to, edge.to_forward).invert();
            node_array[from_directed_reverse].increment();
        }

        // Convert counts to edge list limits by computing the prefix sum.
        let directed_edge_count =
            node_array
                .iter_values_mut()
                .fold(DirectedEdgeIndex::zero(), |sum, element| {
                    let sum = sum.add(*element);
                    *element = sum;
                    sum
                });
        assert_eq!(
            directed_edge_count,
            node_array.iter_values().last().copied().unwrap(),
        );

        // Create edge data structures.
        let mut edge_array = TaggedVec::from_iter(iter::repeat_n(
            DirectedNodeIndex::from_usize(0),
            directed_edge_count.into_usize(),
        ));
        let mut edge_data_keys = TaggedVec::from_iter(iter::repeat_n(
            EdgeDataKey {
                inverse: DirectedEdgeIndex::zero(),
                data_index: OptionalEdgeIndex::new_none(),
            },
            directed_edge_count.into_usize(),
        ));
        let mut edge_data = TaggedVec::new();

        // Now add edges by counting down the edge list limits.
        // Afterwards, the node array will contain the correct edge list offsets.
        for (edge_index, edge) in edges.into_iter(..) {
            let from_directed_forward =
                DirectedNodeIndex::from_bidirected(edge.from, edge.from_forward);
            let to_directed_forward = DirectedNodeIndex::from_bidirected(edge.to, edge.to_forward);
            let edge_index_forward = {
                node_array[from_directed_forward].decrement();
                node_array[from_directed_forward]
            };

            let from_directed_reverse = to_directed_forward.invert();
            let to_directed_reverse = from_directed_forward.invert();
            let edge_index_reverse = {
                node_array[from_directed_reverse].decrement();
                node_array[from_directed_reverse]
            };

            edge_array[edge_index_forward] = to_directed_forward;
            edge_array[edge_index_reverse] = to_directed_reverse;

            edge_data_keys[edge_index_forward] = EdgeDataKey {
                inverse: edge_index_reverse,
                data_index: edge_index.into(),
            };
            edge_data_keys[edge_index_reverse] = EdgeDataKey {
                inverse: edge_index_forward,
                data_index: OptionalEdgeIndex::new_none(),
            };

            let data_index = edge_data.push(BidirectedEdgeData {
                forward: edge_index_forward,
                reverse: edge_index_reverse,
                data: edge.data,
            });
            assert_eq!(edge_index, data_index);
        }

        Self {
            node_array,
            edge_array,
            node_data: nodes,
            edge_data_keys,
            edge_data,
        }
    }

    /// Reorder the edges of the given node according to the given comparator.
    ///
    /// The comparator receives two different to-nodes, representing the two edges from the given node to these to-nodes.
    pub fn reorder_edges(
        &mut self,
        node: DirectedNodeIndex<IndexType>,
        comparator: impl FnMut(
            DirectedNodeIndex<IndexType>,
            DirectedNodeIndex<IndexType>,
        ) -> std::cmp::Ordering,
    ) {
        self.reorder_edges_with_buffers(
            node,
            comparator,
            &mut Permutation::one(0),
            &mut BitVec::new(),
        );
    }

    fn reorder_edges_with_buffers(
        &mut self,
        node: DirectedNodeIndex<IndexType>,
        mut comparator: impl FnMut(
            DirectedNodeIndex<IndexType>,
            DirectedNodeIndex<IndexType>,
        ) -> std::cmp::Ordering,
        permutation: &mut Permutation,
        bitvec: &mut BitVec,
    ) {
        let offset = self.node_array[node].into_usize();
        let limit = self.node_array[node.add(DirectedNodeIndex::from_usize(1))].into_usize();

        // TODO allow for different length, and construct permutation forwards maybe?
        permutation.assign_from_sort_by(
            &self.edge_array.as_untagged_slice()[offset..limit],
            |a, b| comparator(*a, *b),
        );

        // Update edge array.
        permutation
            .apply_slice_in_place(&mut self.edge_array.as_untagged_mut_slice()[offset..limit]);

        // Update edge data keys.
        bitvec.clear();
        bitvec.resize(limit - offset, false);
        for edge_index in offset..limit {
            if bitvec[edge_index - offset] {
                continue;
            }

            let permuted_edge_index =
                DirectedEdgeIndex::from_usize(permutation.apply_idx(edge_index - offset) + offset);
            let edge_index = DirectedEdgeIndex::from_usize(edge_index);
            let inverse_edge_index = self.edge_data_keys[edge_index].inverse;

            self.edge_data_keys[inverse_edge_index].inverse = permuted_edge_index;

            if (offset..limit).contains(&inverse_edge_index.into_usize()) {
                // The inverse edge is from the same directed node, so we mark it as permuted, such that we don't permute it twice.
                // This can happen for self loops.
                bitvec.set(inverse_edge_index.into_usize() - offset, true);

                let permuted_inverse_edge_index = DirectedEdgeIndex::from_usize(
                    permutation.apply_idx(inverse_edge_index.into_usize() - offset) + offset,
                );
                self.edge_data_keys[edge_index].inverse = permuted_inverse_edge_index;
            }
        }

        permutation
            .apply_slice_in_place(&mut self.edge_data_keys.as_untagged_mut_slice()[offset..limit]);

        // Update edge data.
        bitvec.clear();
        bitvec.resize(limit - offset, false);
        for edge_index in offset..limit {
            if bitvec[edge_index - offset] {
                continue;
            }

            let edge_index = DirectedEdgeIndex::from_usize(edge_index);
            let inverse_edge_index = self.edge_data_keys[edge_index].inverse;

            let edge_data_key = self.edge_data_keys[edge_index]
                .data_index
                .into_option()
                .xor(
                    self.edge_data_keys[inverse_edge_index]
                        .data_index
                        .into_option(),
                )
                .unwrap();
            let edge_data = &mut self.edge_data[edge_data_key];

            if (offset..limit).contains(&edge_data.forward.into_usize()) {
                edge_data.forward = DirectedEdgeIndex::from_usize(
                    permutation.apply_idx(edge_data.forward.into_usize() - offset) + offset,
                );
            }

            if (offset..limit).contains(&edge_data.reverse.into_usize()) {
                edge_data.reverse = DirectedEdgeIndex::from_usize(
                    permutation.apply_idx(edge_data.reverse.into_usize() - offset) + offset,
                );
            }

            if (offset..limit).contains(&inverse_edge_index.into_usize()) {
                // The inverse edge is from the same directed node, so we mark it as permuted, such that we don't permute its data twice.
                // This can happen for self loops.
                bitvec.set(inverse_edge_index.into_usize() - offset, true);
            }
        }
    }

    /// Removes multiedges between the same pair of directed nodes, keeping only one of them.
    ///
    /// **Warning:** This does not check if the edge data of the multiedges is actually the same, and simply keeps one of them.
    /// It is the caller's responsibility to ensure that this is the case if they want to keep the data consistent.
    ///
    /// Returns the indices of the removed edges.
    /// Note that this method shifts the edge indices, and hence the returned indices now respond to different edges in the graph, or possibly no edge at all.
    pub fn remove_multiedges(&mut self) -> BTreeSet<EdgeIndex<IndexType>> {
        let mut deleted_edges = BTreeSet::default();
        let mut existing_edges = FxHashSet::default();
        let mut directed_edge_decrement = DirectedEdgeIndex::zero();

        for from_node in self
            .node_array
            .iter_indices(..DirectedNodeIndex::from_usize(self.node_array.len() - 1))
        {
            if DEBUG_REMOVE_MULTIEDGES {
                println!("from_node: {from_node}");
            }

            let edge_offset = self.node_array[from_node];
            let edge_limit = self.node_array[from_node.add(1.into())];

            // Update node array offset.
            self.node_array[from_node] = self.node_array[from_node].sub(directed_edge_decrement);

            existing_edges.clear();
            for directed_edge in self.edge_array.iter_indices(edge_offset..edge_limit) {
                if DEBUG_REMOVE_MULTIEDGES {
                    println!("  directed_edge: {directed_edge}");
                }

                let to_node = self.edge_array[directed_edge];
                if DEBUG_REMOVE_MULTIEDGES {
                    println!("  to_node: {to_node}");
                }
                let decremented_directed_edge = directed_edge.sub(directed_edge_decrement);
                if DEBUG_REMOVE_MULTIEDGES {
                    println!("  decremented_directed_edge: {decremented_directed_edge}");
                }
                let inverse_directed_edge = self.edge_data_keys[directed_edge].inverse;
                if DEBUG_REMOVE_MULTIEDGES {
                    println!("  inverse_directed_edge: {inverse_directed_edge}");
                }

                if existing_edges.insert(to_node) {
                    // First occurrence of this directed edge.
                    // We decrement it only.

                    self.edge_array[decremented_directed_edge] = self.edge_array[directed_edge];

                    self.edge_data_keys[decremented_directed_edge] =
                        self.edge_data_keys[directed_edge];
                    self.edge_data_keys[inverse_directed_edge].inverse = decremented_directed_edge;

                    let edge = self.edge_data_keys[directed_edge]
                        .data_index
                        .into_option()
                        .or(self.edge_data_keys[inverse_directed_edge]
                            .data_index
                            .into_option())
                        .unwrap();
                    if DEBUG_REMOVE_MULTIEDGES {
                        println!("  edge: {edge}");
                    }

                    if self.edge_data[edge].forward == directed_edge {
                        self.edge_data[edge].forward = decremented_directed_edge;
                    } else if self.edge_data[edge].reverse == directed_edge {
                        self.edge_data[edge].reverse = decremented_directed_edge;
                    }
                } else {
                    // This directed edge is repeated and should be deleted.
                    if DEBUG_REMOVE_MULTIEDGES {
                        println!("  DELETE");
                    }
                    if let Some(edge) = self.edge_data_keys[directed_edge].data_index.into_option()
                    {
                        if DEBUG_REMOVE_MULTIEDGES {
                            println!("  edge: {edge}");
                        }
                        deleted_edges.insert(edge);
                    }
                    directed_edge_decrement.increment();
                }
            }
        }

        let deleted_edges = deleted_edges;

        // Remove unused entries at the end of the edge array and edge data keys.
        self.edge_array.splice(
            DirectedEdgeIndex::from_usize(self.edge_array.len()).sub(directed_edge_decrement)..,
            iter::empty(),
        );
        self.edge_data_keys.splice(
            DirectedEdgeIndex::from_usize(self.edge_data_keys.len()).sub(directed_edge_decrement)..,
            iter::empty(),
        );

        let mut deleted_edges_iter = deleted_edges.iter().copied().peekable();
        let mut edge_decrement = EdgeIndex::from_usize(0);
        for edge in self.edge_data.iter_indices(..) {
            if Some(&edge) == deleted_edges_iter.peek() {
                // Edge is deleted.
                deleted_edges_iter.next();
                edge_decrement.increment();
            } else {
                // Edge is not deleted, update indices.
                let decremented_edge = edge.sub(edge_decrement);

                if self.edge_data_keys[self.edge_data[edge].forward]
                    .data_index
                    .is_some()
                {
                    self.edge_data_keys[self.edge_data[edge].forward].data_index =
                        decremented_edge.into();
                } else if self.edge_data_keys[self.edge_data[edge].reverse]
                    .data_index
                    .is_some()
                {
                    self.edge_data_keys[self.edge_data[edge].reverse].data_index =
                        decremented_edge.into();
                }
            }
        }

        // Remove unused entries in the edge data.
        let mut deleted_edges_iter = deleted_edges.iter().copied().peekable();
        let mut current_index = EdgeIndex::from_usize(0);
        self.edge_data.retain(|_| {
            if Some(&current_index) == deleted_edges_iter.peek() {
                // Edge is deleted.
                deleted_edges_iter.next();
                current_index.increment();
                false
            } else {
                // Edge is not deleted.
                current_index.increment();
                true
            }
        });

        deleted_edges
    }

    pub fn node_count(&self) -> usize {
        self.node_data.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edge_data.len()
    }

    pub fn iter_nodes(&self) -> impl Iterator<Item = NodeIndex<IndexType>> {
        self.node_data.iter_indices(..)
    }

    pub fn iter_edges(&self) -> impl Iterator<Item = EdgeIndex<IndexType>> {
        self.edge_data.iter_indices(..)
    }

    pub fn iter_outgoing_edges(
        &self,
        node: DirectedNodeIndex<IndexType>,
    ) -> impl Iterator<Item = DirectedEdge<IndexType>> {
        let start = self.node_array[node];
        let end = self.node_array[node.add(DirectedNodeIndex::from_usize(1))];
        self.edge_array
            .iter(start..end)
            .map(move |(edge_index, &to_node)| DirectedEdge {
                from: node,
                to: to_node,
                index: edge_index,
            })
    }

    /// Iterate over the bidirected edges incident to the given bidirected node.
    pub fn iter_incident_edges(
        &self,
        node: NodeIndex<IndexType>,
    ) -> impl Iterator<Item = EdgeIndex<IndexType>> {
        let forward_node = DirectedNodeIndex::from_bidirected(node, true);
        let reverse_node = DirectedNodeIndex::from_bidirected(node, false);
        self.iter_outgoing_edges(forward_node)
            .chain(self.iter_outgoing_edges(reverse_node))
            .filter_map(|directed_edge| {
                let directed_edge_data = self.directed_edge_data(directed_edge.index());
                if directed_edge.from() == directed_edge.to()
                    || directed_edge.from() == directed_edge.to().invert()
                {
                    directed_edge_data
                        .is_forward()
                        .then_some(directed_edge_data.edge())
                } else {
                    Some(directed_edge_data.edge())
                }
            })
    }

    pub fn node_data(&self, node: NodeIndex<IndexType>) -> &NodeData {
        &self.node_data[node]
    }

    pub fn edge(&self, edge: EdgeIndex<IndexType>) -> EdgeView<'_, IndexType, EdgeData> {
        let bidirected_edge_data = &self.edge_data[edge];

        let forward_to = self.edge_array[bidirected_edge_data.forward];
        let reverse_to = self.edge_array[bidirected_edge_data.reverse];

        if forward_to == reverse_to {
            // ++ or -- self loop case: both directed edges go from node to its reverse.
            let from = forward_to.invert();
            let to = forward_to;
            EdgeView {
                from,
                to,
                forward: bidirected_edge_data.forward,
                reverse: bidirected_edge_data.reverse,
                data: &bidirected_edge_data.data,
            }
        } else if forward_to.invert() == reverse_to {
            // +- or -+ self loop case: directed edges are self loops.
            let from = forward_to;
            let to = forward_to;
            EdgeView {
                from,
                to,
                forward: bidirected_edge_data.forward,
                reverse: bidirected_edge_data.reverse,
                data: &bidirected_edge_data.data,
            }
        } else {
            // Normal case: directed edges go between two different nodes.
            let from = reverse_to.invert();
            let to = forward_to;
            EdgeView {
                from,
                to,
                forward: bidirected_edge_data.forward,
                reverse: bidirected_edge_data.reverse,
                data: &bidirected_edge_data.data,
            }
        }
    }

    pub fn directed_edge_data<'this>(
        &'this self,
        directed_edge: DirectedEdgeIndex<IndexType>,
    ) -> DirectedEdgeDataView<'this, IndexType, EdgeData> {
        let key = &self.edge_data_keys[directed_edge];
        if let Some(edge) = key.data_index.into_option() {
            DirectedEdgeDataView {
                is_forward: true,
                edge,
                data: &self.edge_data[edge].data,
            }
        } else {
            let inverse_key = &self.edge_data_keys[key.inverse];
            let Some(edge) = inverse_key.data_index.into_option() else {
                panic!(
                    "Edge data for edge {:?} and its inverse {:?} are both missing",
                    directed_edge, key.inverse
                );
            };
            DirectedEdgeDataView {
                is_forward: false,
                edge,
                data: &self.edge_data[edge].data,
            }
        }
    }

    pub fn directed_edge_into_bidirected(
        &self,
        directed_edge: DirectedEdgeIndex<IndexType>,
    ) -> EdgeIndex<IndexType> {
        let key = &self.edge_data_keys[directed_edge];
        if let Some(edge) = key.data_index.into_option() {
            edge
        } else {
            let inverse_key = &self.edge_data_keys[key.inverse];
            inverse_key
                .data_index
                .expect("Edge data for directed edge and its inverse are both missing")
        }
    }
}

impl<IndexType> DirectedEdge<IndexType> {
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

    pub fn index(&self) -> DirectedEdgeIndex<IndexType>
    where
        IndexType: Copy,
    {
        self.index
    }
}

impl<'a, IndexType, EdgeData> DirectedEdgeDataView<'a, IndexType, EdgeData> {
    pub fn is_forward(&self) -> bool {
        self.is_forward
    }

    pub fn edge(&self) -> EdgeIndex<IndexType>
    where
        IndexType: Copy,
    {
        self.edge
    }

    pub fn data(&self) -> &EdgeData {
        self.data
    }
}

impl<'a, IndexType, EdgeData> EdgeView<'a, IndexType, EdgeData> {
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

    pub fn forward(&self) -> DirectedEdgeIndex<IndexType>
    where
        IndexType: Copy,
    {
        self.forward
    }

    pub fn reverse(&self) -> DirectedEdgeIndex<IndexType>
    where
        IndexType: Copy,
    {
        self.reverse
    }

    pub fn data(&self) -> &EdgeData {
        self.data
    }
}

impl<IndexType: GraphIndexInteger, EdgeData> BidirectedEdge<IndexType, EdgeData> {
    pub fn new(
        from: DirectedNodeIndex<IndexType>,
        to: DirectedNodeIndex<IndexType>,
        data: EdgeData,
    ) -> Self {
        Self {
            from: from.into_bidirected(),
            from_forward: from.is_forward(),
            to: to.into_bidirected(),
            to_forward: to.is_forward(),
            data,
        }
    }
}

impl<IndexType: GraphIndexInteger> BidirectedEdge<IndexType, PlainGfaEdgeData> {
    pub fn new_gfa(
        from: DirectedNodeIndex<IndexType>,
        to: DirectedNodeIndex<IndexType>,
        overlap: u16,
    ) -> Self {
        Self {
            from: from.into_bidirected(),
            from_forward: from.is_forward(),
            to: to.into_bidirected(),
            to_forward: to.is_forward(),
            data: PlainGfaEdgeData::new(overlap),
        }
    }
}
