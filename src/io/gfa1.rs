use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::Debug,
    io::{BufRead, Write},
};

use log::warn;
use tagged_vec::TaggedVec;

use crate::{
    graph::{BidirectedAdjacencyArray, BidirectedEdge},
    index::{EdgeIndex, GraphIndexInteger, NodeIndex},
};

#[cfg(test)]
mod tests;

pub trait GfaNodeData {
    fn name(&'_ self) -> Cow<'_, str>;
    fn sequence(&'_ self) -> Cow<'_, str>;
}

pub trait GfaEdgeData {
    fn overlap(&self) -> u16;
}

#[derive(thiserror::Error, Debug)]
pub enum GfaReadError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("a header line was found after other lines")]
    WronglyPositionedHeader,

    #[error("an S line is missing the sequence name")]
    MissingSequenceNameInSLine,

    #[error("an L line is missing the four fields specifying the edge endpoints")]
    LLineTooShort,

    #[error("unknown node name '{0}' in an L line")]
    UnknownNodeName(String),

    #[error("unknown sign '{0}' in an L line")]
    UnknownGfaNodeSign(String),
}

struct UnresolvedBidirectedEdge {
    from: String,
    from_forward: bool,
    to: String,
    to_forward: bool,
    data: PlainGfaEdgeData,
}

impl<
    IndexType: GraphIndexInteger,
    NodeData: From<PlainGfaNodeData>,
    EdgeData: From<PlainGfaEdgeData>,
> BidirectedAdjacencyArray<IndexType, NodeData, EdgeData>
{
    pub fn read_gfa1(
        reader: impl BufRead,
    ) -> Result<BidirectedAdjacencyArray<IndexType, NodeData, EdgeData>, GfaReadError> {
        let mut node_name_to_node = HashMap::new();
        let mut nodes = TaggedVec::<NodeIndex<IndexType>, _>::new();
        let mut edges = TaggedVec::<EdgeIndex<IndexType>, _>::new();
        let mut is_header_allowed = true;

        for line in reader.lines() {
            let line = line?;
            let line = line.trim().split('\t').collect::<Vec<_>>();

            match line[0] {
                "H" => {
                    if is_header_allowed {
                        if line.get(1) != Some(&"VN:Z:1.0") {
                            warn!("Unsupported GFA version");
                        }
                    } else {
                        return Err(GfaReadError::WronglyPositionedHeader);
                    }
                }

                "S" => {
                    let name = line
                        .get(1)
                        .ok_or(GfaReadError::MissingSequenceNameInSLine)?
                        .to_string();
                    let sequence = line.get(2).unwrap_or(&"").to_string();
                    let node = nodes.push(
                        PlainGfaNodeData {
                            name: name.clone(),
                            sequence,
                        }
                        .into(),
                    );
                    node_name_to_node.insert(name.clone(), node);
                }

                "L" => {
                    // Parse edge line.
                    let from = line.get(1).ok_or(GfaReadError::LLineTooShort)?.to_string();
                    let from_forward = match *line.get(2).ok_or(GfaReadError::LLineTooShort)? {
                        "+" => true,
                        "-" => false,
                        other => return Err(GfaReadError::UnknownGfaNodeSign(other.to_string())),
                    };
                    let to = line.get(3).ok_or(GfaReadError::LLineTooShort)?.to_string();
                    let to_forward = match *line.get(4).ok_or(GfaReadError::LLineTooShort)? {
                        "+" => true,
                        "-" => false,
                        other => return Err(GfaReadError::UnknownGfaNodeSign(other.to_string())),
                    };
                    let overlap_str = line.get(5).unwrap_or(&"0M");
                    let overlap = overlap_str
                        .trim_end_matches('M')
                        .parse::<u16>()
                        .unwrap_or(0);

                    edges.push(UnresolvedBidirectedEdge {
                        from,
                        from_forward,
                        to,
                        to_forward,
                        data: PlainGfaEdgeData { overlap },
                    });
                }

                other => {
                    warn!("Unsupported GFA line type: {}", other);
                }
            }

            is_header_allowed = false;
        }

        let edges = edges
            .into_values_iter()
            .map(|edge| {
                let from = node_name_to_node
                    .get(&edge.from)
                    .copied()
                    .ok_or(GfaReadError::UnknownNodeName(edge.from))?;
                let to = node_name_to_node
                    .get(&edge.to)
                    .copied()
                    .ok_or(GfaReadError::UnknownNodeName(edge.to))?;

                let from_forward = edge.from_forward;
                let to_forward = edge.to_forward;
                let data = EdgeData::from(edge.data);

                Result::<_, GfaReadError>::Ok(BidirectedEdge {
                    from,
                    from_forward,
                    to,
                    to_forward,
                    data,
                })
            })
            .collect::<Result<Vec<_>, GfaReadError>>()?;

        // Drop name index before constructing graph to save RAM.
        drop(node_name_to_node);
        Ok(BidirectedAdjacencyArray::new(nodes, edges.into()))
    }
}

impl<IndexType: GraphIndexInteger, NodeData: GfaNodeData, EdgeData: GfaEdgeData>
    BidirectedAdjacencyArray<IndexType, NodeData, EdgeData>
{
    pub fn write_gfa1(&self, mut writer: impl Write) -> Result<(), std::io::Error> {
        // Write header.
        writeln!(writer, "H\tVN:Z:1.0")?;

        // Write nodes.
        for node in self.iter_nodes() {
            let node_data = self.node_data(node);
            writeln!(writer, "S\t{}\t{}", node_data.name(), node_data.sequence())?;
        }

        // Write edges.
        for edge in self.iter_edges() {
            let edge_data = self.edge(edge);

            let from_node_name = self.node_data(edge_data.from().into_bidirected()).name();
            let to_node_name = self.node_data(edge_data.to().into_bidirected()).name();

            // In mathematical notation, traversing an edge from a to b means using edge (a, \hat{b}).
            // But in GFA1, this means using edge (a, b), where both signs are unchanged.
            let from_node_sign = if edge_data.from().is_forward() {
                "+"
            } else {
                "-"
            };
            let to_node_sign = if edge_data.to().is_forward() {
                "+"
            } else {
                "-"
            };

            let overlap = edge_data.data().overlap();

            writeln!(
                writer,
                "L\t{from_node_name}\t{from_node_sign}\t{to_node_name}\t{to_node_sign}\t{overlap}M",
            )?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlainGfaNodeData {
    name: String,
    sequence: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlainGfaEdgeData {
    overlap: u16,
}

impl PlainGfaNodeData {
    pub fn new(name: impl ToString, sequence: impl ToString) -> Self {
        Self {
            name: name.to_string(),
            sequence: sequence.to_string(),
        }
    }
}

impl GfaNodeData for PlainGfaNodeData {
    fn name(&'_ self) -> Cow<'_, str> {
        Cow::Borrowed(&self.name)
    }

    fn sequence(&'_ self) -> Cow<'_, str> {
        Cow::Borrowed(&self.sequence)
    }
}

impl PlainGfaEdgeData {
    pub fn new(overlap: u16) -> Self {
        Self { overlap }
    }
}

impl GfaEdgeData for PlainGfaEdgeData {
    fn overlap(&self) -> u16 {
        self.overlap
    }
}
