use std::{
    collections::HashMap,
    fmt::Debug,
    io::{BufRead, BufReader, BufWriter, Read, Write},
};

use log::warn;
use tagged_vec::TaggedVec;

use crate::{
    graph::{BidirectedAdjacencyArray, BidirectedEdge},
    index::{EdgeIndex, GraphIndexInteger, NodeIndex},
};

pub struct GfaNodeData {
    name: String,
    sequence: String,
}

pub struct GfaEdgeData {
    overlap: u16,
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

pub fn read_gfa1<IndexType: GraphIndexInteger>(
    reader: &mut impl Read,
) -> Result<BidirectedAdjacencyArray<IndexType, GfaNodeData, GfaEdgeData>, GfaReadError> {
    let reader = BufReader::new(reader);
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
                let node = nodes.push(GfaNodeData {
                    name: name.clone(),
                    sequence,
                });
                node_name_to_node.insert(name.clone(), node);
            }

            "L" => {
                // Parse edge line.
                let from_name = line.get(1).ok_or(GfaReadError::LLineTooShort)?;
                let from = node_name_to_node
                    .get(*from_name)
                    .copied()
                    .ok_or_else(|| GfaReadError::UnknownNodeName(from_name.to_string()))?;
                let from_forward = match *line.get(2).ok_or(GfaReadError::LLineTooShort)? {
                    "+" => true,
                    "-" => false,
                    other => return Err(GfaReadError::UnknownGfaNodeSign(other.to_string())),
                };
                let to_name = line.get(3).ok_or(GfaReadError::LLineTooShort)?;
                let to = node_name_to_node
                    .get(*to_name)
                    .copied()
                    .ok_or_else(|| GfaReadError::UnknownNodeName(to_name.to_string()))?;
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

                edges.push(BidirectedEdge {
                    from,
                    from_forward,
                    to,
                    to_forward,
                    data: GfaEdgeData { overlap },
                });
            }

            other => {
                warn!("Unsupported GFA line type: {}", other);
            }
        }

        is_header_allowed = false;
    }

    todo!()
}

pub fn write_gfa1<IndexType: GraphIndexInteger, NodeData, EdgeData>(
    graph: &BidirectedAdjacencyArray<IndexType, NodeData, EdgeData>,
    writer: &mut impl Write,
) -> Result<(), std::io::Error>
where
    GfaNodeData: for<'a> From<&'a NodeData>,
    GfaEdgeData: for<'a> From<&'a EdgeData>,
{
    let mut writer = BufWriter::new(writer);

    // Write header.
    writeln!(writer, "H\tVN:Z:1.0")?;

    // Write nodes.
    for node in graph.iter_nodes() {
        let node_data = graph.node_data(node);
        let gfa_node_data: GfaNodeData = node_data.into();
        writeln!(
            writer,
            "S\t{}\t{}",
            gfa_node_data.name, gfa_node_data.sequence,
        )?;
    }

    // Write edges.
    for edge in graph.iter_edges() {
        let edge_data = graph.edge_data(edge);
        let gfa_edge_data: GfaEdgeData = edge_data.data().into();

        let from_node_name =
            &GfaNodeData::from(graph.node_data(edge_data.from().into_bidirected())).name;
        let to_node_name =
            &GfaNodeData::from(graph.node_data(edge_data.to().into_bidirected())).name;

        // In mathematical notation, traversing an edge from a to b means using edge (a, \hat{b}).
        // But in GFA1, this means using edge (a, b), where both signs are unchanged.
        let from_node_sign = if edge_data.from().is_forward() {
            "+"
        } else {
            "-"
        };
        let to_node_sign = if edge_data.to().is_reverse() {
            "+"
        } else {
            "-"
        };

        let overlap = gfa_edge_data.overlap;

        writeln!(
            writer,
            "L\t{from_node_name}\t{from_node_sign}\t{to_node_name}\t{to_node_sign}\t{overlap}M"
        )?;
    }

    Ok(())
}
