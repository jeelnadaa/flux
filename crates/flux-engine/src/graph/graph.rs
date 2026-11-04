//! Audio processing graph with DAG topological execution and PDC.

use super::node::{EngineNode, NodeId};
use super::pdc::PluginDelayCompensation;
use flux_dsp::delay::DelayLine;
use flux_dsp::node::DspNode;
use std::collections::{HashMap, HashSet};

/// Error encountered in audio graph topology or execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
    /// Graph contains a cyclic feedback loop.
    CycleDetected,
    /// Specified node ID does not exist in graph.
    NodeNotFound(NodeId),
}

/// Directed acyclic audio processing graph.
pub struct AudioGraph {
    nodes: HashMap<NodeId, EngineNode>,
    edges: Vec<(NodeId, NodeId)>,
    execution_order: Vec<NodeId>,
    next_node_id: usize,
    sample_rate: f32,
    max_block_size: usize,
    // Preallocated node output buffers for zero-allocation real-time execution: NodeId -> Vec<Vec<f32>>
    node_buffers: HashMap<NodeId, Vec<Vec<f32>>>,
    // Delay compensation lines for PDC: NodeId -> DelayLine
    pdc_delays: HashMap<NodeId, DelayLine>,
    terminal_nodes: Vec<NodeId>,
    // Precomputed map of incoming parent nodes for each node: NodeId -> list of parent NodeIds
    predecessors: HashMap<NodeId, Vec<NodeId>>,
    // Reusable scratch input buffer per quantum (8 channels x max_block_size)
    input_scratch: Vec<Vec<f32>>,
    // Reusable scratch output buffer per quantum (8 channels x max_block_size)
    output_scratch: Vec<Vec<f32>>,
}

impl AudioGraph {
    /// Creates a new empty audio graph.
    pub fn new(sample_rate: f32, max_block_size: usize) -> Self {
        let max_bs = max_block_size.max(128);
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            execution_order: Vec::new(),
            next_node_id: 1,
            sample_rate,
            max_block_size: max_bs,
            node_buffers: HashMap::new(),
            pdc_delays: HashMap::new(),
            terminal_nodes: Vec::new(),
            predecessors: HashMap::new(),
            input_scratch: vec![vec![0.0f32; max_bs]; 8],
            output_scratch: vec![vec![0.0f32; max_bs]; 8],
        }
    }

    /// Adds a DSP node to the graph and returns its unique `NodeId`.
    pub fn add_node(
        &mut self,
        name: impl Into<String>,
        dsp: Box<dyn DspNode>,
        channels: usize,
    ) -> NodeId {
        let id = NodeId(self.next_node_id);
        self.next_node_id += 1;

        let mut node = EngineNode::new(id, name, dsp, channels);
        node.prepare(self.sample_rate);

        // Preallocate output scratch buffers
        let buffers = vec![vec![0.0f32; self.max_block_size]; channels];
        self.node_buffers.insert(id, buffers);
        self.nodes.insert(id, node);

        id
    }

    /// Removes a node and its connections from the graph.
    pub fn remove_node(&mut self, id: NodeId) -> Result<(), GraphError> {
        if self.nodes.remove(&id).is_none() {
            return Err(GraphError::NodeNotFound(id));
        }
        self.edges.retain(|&(from, to)| from != id && to != id);
        self.node_buffers.remove(&id);
        self.pdc_delays.remove(&id);
        self.update_topology()?;
        Ok(())
    }

    /// Connects the output of `from` to the input of `to`.
    pub fn connect(&mut self, from: NodeId, to: NodeId) -> Result<(), GraphError> {
        if !self.nodes.contains_key(&from) {
            return Err(GraphError::NodeNotFound(from));
        }
        if !self.nodes.contains_key(&to) {
            return Err(GraphError::NodeNotFound(to));
        }

        if !self.edges.contains(&(from, to)) {
            self.edges.push((from, to));
        }

        self.update_topology()
    }

    /// Recomputes topological sort and plugin delay compensation.
    pub fn update_topology(&mut self) -> Result<(), GraphError> {
        // Kahn's algorithm for topological sorting
        let mut in_degree: HashMap<NodeId, usize> = HashMap::new();
        let mut adj: HashMap<NodeId, Vec<NodeId>> = HashMap::new();

        for &id in self.nodes.keys() {
            in_degree.insert(id, 0);
        }

        for &(from, to) in &self.edges {
            adj.entry(from).or_default().push(to);
            *in_degree.entry(to).or_insert(0) += 1;
        }

        let mut queue: Vec<NodeId> = in_degree
            .iter()
            .filter(|&(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut order = Vec::with_capacity(self.nodes.len());

        while let Some(u) = queue.pop() {
            order.push(u);
            if let Some(neighbors) = adj.get(&u) {
                for &v in neighbors {
                    if let Some(deg) = in_degree.get_mut(&v) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(v);
                        }
                    }
                }
            }
        }

        if order.len() != self.nodes.len() {
            return Err(GraphError::CycleDetected);
        }

        self.execution_order = order;

        // Determine terminal nodes (out_degree == 0)
        let outgoing: HashSet<NodeId> = self.edges.iter().map(|&(from, _)| from).collect();
        self.terminal_nodes = self
            .nodes
            .keys()
            .copied()
            .filter(|id| !outgoing.contains(id))
            .collect();

        // Precompute incoming predecessor parent nodes
        let mut predecessors: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for &(from, to) in &self.edges {
            predecessors.entry(to).or_default().push(from);
        }
        self.predecessors = predecessors;

        // Update PDC compensation
        let mut latencies = HashMap::new();
        for (&id, node) in &self.nodes {
            latencies.insert(id, node.dsp.latency_samples());
        }

        let compensations = PluginDelayCompensation::calculate_compensation(
            &latencies,
            &self.edges,
            &self.terminal_nodes,
        );

        for (&id, &delay_samples) in &compensations {
            if delay_samples > 0 {
                let delay = self
                    .pdc_delays
                    .entry(id)
                    .or_insert_with(|| DelayLine::new(delay_samples + 256));
                delay.set_delay(delay_samples as f32);
            } else if let Some(delay) = self.pdc_delays.get_mut(&id) {
                delay.set_delay(0.0);
            }
        }

        Ok(())
    }

    /// Returns the node by id.
    pub fn get_node(&self, id: NodeId) -> Option<&EngineNode> {
        self.nodes.get(&id)
    }

    /// Returns mutable node by id.
    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut EngineNode> {
        self.nodes.get_mut(&id)
    }

    /// Resets all nodes in the graph.
    pub fn reset(&mut self) {
        for node in self.nodes.values_mut() {
            node.dsp.reset();
        }
        for delay in self.pdc_delays.values_mut() {
            delay.reset();
        }
        for buffers in self.node_buffers.values_mut() {
            for ch in buffers {
                ch.fill(0.0);
            }
        }
    }

    /// Executes one render quantum across the graph in topological order.
    ///
    /// # Safety and Real-Time Guarantees
    /// This method makes ZERO heap allocations and acquires NO locks during execution.
    pub fn process(&mut self, input: &[&[f32]], output: &mut [&mut [f32]]) {
        if self.execution_order.is_empty() || output.is_empty() {
            return;
        }

        let num_frames = output[0].len().min(self.max_block_size);

        // Clear output
        for ch in output.iter_mut() {
            ch[..num_frames].fill(0.0);
        }

        for &node_id in &self.execution_order {
            let (in_channels, out_channels) = {
                let node = self.nodes.get(&node_id).unwrap();
                (node.input_channels.min(8), node.output_channels.min(8))
            };

            // Prepare inputs for this node
            if let Some(parents) = self.predecessors.get(&node_id).filter(|p| !p.is_empty()) {
                // Sum outputs from all parent nodes
                for ch in 0..in_channels {
                    self.input_scratch[ch][..num_frames].fill(0.0);
                }

                for &parent_id in parents {
                    let parent_buf = self.node_buffers.get(&parent_id).unwrap();
                    let p_chs = parent_buf.len().min(in_channels);
                    for ch in 0..p_chs {
                        for i in 0..num_frames {
                            self.input_scratch[ch][i] += parent_buf[ch][i];
                        }
                    }
                }
            } else {
                // Root node takes external graph input
                for ch in 0..in_channels {
                    if ch < input.len() {
                        let frames = input[ch].len().min(num_frames);
                        self.input_scratch[ch][..frames].copy_from_slice(&input[ch][..frames]);
                        if frames < num_frames {
                            self.input_scratch[ch][frames..num_frames].fill(0.0);
                        }
                    } else {
                        self.input_scratch[ch][..num_frames].fill(0.0);
                    }
                }
            }

            // Process node into preallocated scratch without heap allocation
            {
                let node = self.nodes.get_mut(&node_id).unwrap();

                let mut in_refs: [&[f32]; 8] = [&[]; 8];
                for ch in 0..in_channels {
                    in_refs[ch] = &self.input_scratch[ch][..num_frames];
                }

                let mut out_refs: [&mut [f32]; 8] = Default::default();
                for (ch, buf) in self.output_scratch.iter_mut().take(out_channels).enumerate() {
                    buf[..num_frames].fill(0.0);
                    out_refs[ch] = &mut buf[..num_frames];
                }

                node.dsp
                    .process(&in_refs[..in_channels], &mut out_refs[..out_channels]);

                // Copy to node's output buffer and apply PDC delay if present
                let node_buf = self.node_buffers.get_mut(&node_id).unwrap();
                for ch in 0..out_channels {
                    node_buf[ch][..num_frames]
                        .copy_from_slice(&self.output_scratch[ch][..num_frames]);
                }

                if let Some(delay) = self.pdc_delays.get_mut(&node_id) {
                    if delay.delay() > 0.0 {
                        for ch in node_buf.iter_mut().take(out_channels) {
                            for sample in ch.iter_mut().take(num_frames) {
                                *sample = delay.process_sample(*sample);
                            }
                        }
                    }
                }
            }
        }

        // Sum terminal nodes to master output
        for &t_id in &self.terminal_nodes {
            let t_buf = self.node_buffers.get(&t_id).unwrap();
            let chs = t_buf.len().min(output.len());
            for ch in 0..chs {
                for i in 0..num_frames {
                    output[ch][i] += t_buf[ch][i];
                }
            }
        }
    }
}
