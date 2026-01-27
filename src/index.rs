use std::fmt::Debug;

use num_traits::PrimInt;
use optional_numeric_index::implement_generic_index;

pub trait GraphIndexInteger: PrimInt + Debug + From<u8> + TryFrom<usize> + TryInto<usize> {}

implement_generic_index!(pub NodeIndex, pub OptionalNodeIndex);
implement_generic_index!(pub EdgeIndex, pub OptionalEdgeIndex);

implement_generic_index!(pub DirectedNodeIndex, pub OptionalDirectedNodeIndex);
implement_generic_index!(pub DirectedEdgeIndex, pub OptionalDirectedEdgeIndex);

impl<IndexType: GraphIndexInteger> DirectedNodeIndex<IndexType> {
    pub fn invert(self) -> Self {
        DirectedNodeIndex(self.0 ^ 1u8.into())
    }

    pub fn is_forward(self) -> bool {
        (self.0 & 1u8.into()) == 0u8.into()
    }

    pub fn is_reverse(self) -> bool {
        !self.is_forward()
    }

    pub fn into_bidirected(self) -> NodeIndex<IndexType> {
        NodeIndex(self.0 / 2u8.into())
    }
}
