use std::io::Read;

use tagged_vec::TaggedVec;

use crate::{graph::BidirectedAdjacencyArray, index::GraphIndexInteger};

impl<IndexType: GraphIndexInteger, NodeData: Copy, EdgeData: Copy>
    BidirectedAdjacencyArray<IndexType, NodeData, EdgeData>
{
    /// Reads a bidirected adjacency array from a platform-dependent binary format.
    pub fn read_binary(mut reader: impl Read) -> std::io::Result<Self> {
        Ok(Self {
            node_array: TaggedVec::read_binary(&mut reader)?,
            edge_array: TaggedVec::read_binary(&mut reader)?,
            node_data: TaggedVec::read_binary(&mut reader)?,
            edge_data_keys: TaggedVec::read_binary(&mut reader)?,
            edge_data: TaggedVec::read_binary(&mut reader)?,
        })
    }

    /// Writes the bidirected adjacency array into a platform-dependent binary format.
    pub fn write_binary(&self, mut writer: impl std::io::Write) -> std::io::Result<()> {
        self.node_array.write_binary(&mut writer)?;
        self.edge_array.write_binary(&mut writer)?;
        self.node_data.write_binary(&mut writer)?;
        self.edge_data_keys.write_binary(&mut writer)?;
        self.edge_data.write_binary(&mut writer)?;
        Ok(())
    }
}
