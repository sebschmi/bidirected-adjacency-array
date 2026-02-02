use std::{
    fmt::{Debug, Display},
    hash::Hash,
};

use num_traits::PrimInt;
use optional_numeric_index::implement_generic_index;

pub trait GraphIndexInteger:
    PrimInt + Hash + Debug + Display + From<u8> + TryFrom<usize> + TryInto<usize>
{
}

impl<T: PrimInt + Hash + Debug + Display + From<u8> + TryFrom<usize> + TryInto<usize>>
    GraphIndexInteger for T
{
}

implement_generic_index!(pub NodeIndex, pub OptionalNodeIndex);
implement_generic_index!(pub EdgeIndex, pub OptionalEdgeIndex);

implement_generic_index!(pub DirectedNodeIndex, pub OptionalDirectedNodeIndex);
implement_generic_index!(pub DirectedEdgeIndex, pub OptionalDirectedEdgeIndex);

impl<IndexType: GraphIndexInteger> DirectedNodeIndex<IndexType> {
    pub fn from_bidirected(bidirected: NodeIndex<IndexType>, forward: bool) -> Self {
        let base = bidirected.0 * 2u8.into();
        if forward {
            DirectedNodeIndex(base)
        } else {
            DirectedNodeIndex(base + 1u8.into())
        }
    }

    pub fn into_bidirected(self) -> NodeIndex<IndexType> {
        NodeIndex(self.0 / 2u8.into())
    }

    pub fn invert(self) -> Self {
        DirectedNodeIndex(self.0 ^ 1u8.into())
    }

    pub fn is_forward(self) -> bool {
        (self.0 & 1u8.into()) == 0u8.into()
    }

    pub fn is_reverse(self) -> bool {
        !self.is_forward()
    }

    pub(crate) fn add(self, other: DirectedNodeIndex<IndexType>) -> DirectedNodeIndex<IndexType> {
        Self::new(self.0 + other.0)
    }
}

impl<IndexType: GraphIndexInteger> DirectedEdgeIndex<IndexType> {
    pub(crate) fn zero() -> Self {
        Self::new(0u8.into())
    }

    pub(crate) fn increment(&mut self) {
        *self = Self::new(self.0 + 1u8.into());
    }

    pub(crate) fn decrement(&mut self) {
        *self = Self::new(self.0 - 1u8.into());
    }

    pub(crate) fn add(self, other: DirectedEdgeIndex<IndexType>) -> DirectedEdgeIndex<IndexType> {
        Self::new(self.0 + other.0)
    }
}
