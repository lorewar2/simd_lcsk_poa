
use std::cmp::{max, Ordering};
use petgraph::graph::NodeIndex;
use petgraph::visit::Topo;
use petgraph::{Directed, Graph, Incoming};
pub const MIN_SCORE: i32 = -858_993_459; // negative infinity; see alignment/pairwise/mod.rs
pub type POAGraph = Graph<u8, i32, Directed, usize>;
use std::simd::{i16x8, Simd};
use std::simd::cmp::SimdOrd;
use std::collections::HashMap;

// Unlike with a total order we may have arbitrary successors in the
// traceback matrix. I have not yet figured out what the best level of
// detail to store is, so Match and Del operations remember In and Out
// nodes on the reference graph.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum AlignmentOperation {
    Match(Option<(usize, usize)>),
    Del(Option<(usize, usize)>),
    Ins(Option<usize>),
    Xclip(usize),
    Yclip(usize, usize), // to, from
}

#[derive(Default, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Alignment {
    pub score: i32,
    //    xstart: Edge,
    operations: Vec<AlignmentOperation>,
}

#[derive(Copy, Clone, Debug)]
pub struct TracebackCell {
    score: i32,
    op: AlignmentOperation,
}

impl Ord for TracebackCell {
    fn cmp(&self, other: &TracebackCell) -> Ordering {
        self.score.cmp(&other.score)
    }
}

impl PartialOrd for TracebackCell {
    fn partial_cmp(&self, other: &TracebackCell) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for TracebackCell {
    fn eq(&self, other: &TracebackCell) -> bool {
        self.score == other.score
    }
}

//impl Default for TracebackCell { }

impl Eq for TracebackCell {}

#[derive(Default, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct Traceback {
    rows: usize,
    cols: usize,

    // store the last visited node in topological order so that
    // we can index into the end of the alignment when we backtrack
    last: NodeIndex<usize>,
    matrix: Vec<(Vec<TracebackCell>, usize, usize)>,
}

impl Traceback {
    /// Create a Traceback matrix with given maximum sizes
    ///
    /// # Arguments
    ///
    /// * `m` - the number of nodes in the DAG
    /// * `n` - the length of the query sequence
    fn with_capacity(m: usize, n: usize) -> Self {
        // each row of matrix contain start end position and vec of traceback cells
        let matrix: Vec<(Vec<TracebackCell>, usize, usize)> = vec![(vec![], 0, n + 1); m + 1];
        Traceback {
            rows: m,
            cols: n,
            last: NodeIndex::new(0),
            matrix,
        }
    }
    /// Populate the first row of the traceback matrix
    fn initialize_scores(&mut self, gap_open: i32) {
        for j in 0..=self.cols {
            self.matrix[0].0.push(
                TracebackCell {
                    score: (j as i32) * gap_open,
                    op: AlignmentOperation::Ins(None),
                });
        }
        self.matrix[0].0[0] = TracebackCell {
            score: 0,
            op: AlignmentOperation::Match(None),
        };
    }

    fn new() -> Self {
        Traceback {
            rows: 0,
            cols: 0,
            last: NodeIndex::new(0),
            matrix: Vec::new(),
        }
    }

    // create a new row according to the parameters
    fn new_row(
        &mut self,
        row: usize,
        size: usize,
        gap_open: i32,
        start: usize,
        end: usize,
    ) {
        self.matrix[row].1 = start;
        self.matrix[row].2 = end;
        // when the row starts from the edge
        if start == 0 {
            self.matrix[row].0.push(
                TracebackCell {
                    score: (row as i32) * gap_open,
                    op: AlignmentOperation::Del(None),
                }
            );
        } else {
            self.matrix[row].0.push(TracebackCell {
                score: MIN_SCORE,
                op: AlignmentOperation::Match(None),
            });
        }
        for _ in 1..=size {
            self.matrix[row].0.push(TracebackCell {
                score: MIN_SCORE,
                op: AlignmentOperation::Match(None),
            });
        }
    }

    fn set(&mut self, i: usize, j: usize, cell: TracebackCell) {
        // set the matrix cell if in band range
        if !(self.matrix[i].1 > j || self.matrix[i].2 < j) {
            let real_position = j - self.matrix[i].1;
            self.matrix[i].0[real_position] = cell;
        }
    }

    fn get(&self, i: usize, j: usize) -> &TracebackCell {
        // get the matrix cell if in band range else return the appropriate values
        if !(self.matrix[i].1 > j || self.matrix[i].2 <= j || self.matrix[i].0.is_empty()) {
            let real_position = j - self.matrix[i].1;
            &self.matrix[i].0[real_position]
        }
        // behind the band, met the edge
        else if j == 0 {
            &TracebackCell {
                score: MIN_SCORE,
                op: AlignmentOperation::Del(None),
            }
        }
        // infront of the band
        else if j >= self.matrix[i].2 {
            &TracebackCell {
                score: MIN_SCORE,
                op: AlignmentOperation::Ins(None),
            }
        }
        // behind the band
        else {
            &TracebackCell {
                score: MIN_SCORE,
                op: AlignmentOperation::Match(None),
            }
        }
    }

    pub fn alignment(&self) -> Alignment {
        // optimal AlignmentOperation path
        let mut ops: Vec<AlignmentOperation> = vec![];

        // Now backtrack through the matrix to construct an optimal path
        let mut i = self.last.index() + 1;
        let mut j = self.cols;

        while i > 0 || j > 0 {
            // push operation and edge corresponding to (one of the) optimal
            // routes
            ops.push(self.get(i, j).op);
            match self.get(i, j).op {
                AlignmentOperation::Match(Some((p, _))) => {
                    i = p + 1;
                    j -= 1;
                    println!("Match");
                }
                AlignmentOperation::Del(Some((p, _))) => {
                    i = p + 1;
                    println!("Del");
                }
                AlignmentOperation::Ins(Some(p)) => {
                    i = p + 1;
                    j -= 1;
                    println!("Ins");
                }
                AlignmentOperation::Match(None) => {
                    i = 0;
                    j -= 1;
                    println!("Match Non");
                }
                AlignmentOperation::Del(None) => {
                    i -= 1;
                    println!("Del Non");
                }
                AlignmentOperation::Ins(None) => {
                    j -= 1;
                    println!("Ins Non");
                }
                AlignmentOperation::Xclip(r) => {
                    i = r;
                    println!("X clip");
                }
                AlignmentOperation::Yclip(r, _) => {
                    j = r;
                    println!("Y clip");
                }
            }
        }
        ops.reverse();

        Alignment {
            score: self.get(self.last.index() + 1, self.cols).score,
            operations: ops,
        }
    }
}

/// A partially ordered aligner builder
///
/// Uses consuming builder pattern for constructing partial order alignments with method chaining
#[derive(Default, Clone, Debug)]
pub struct Aligner {
    traceback: Traceback,
    query: Vec<u8>,
    poa: Poa,
}

impl Aligner {
    /// Create new instance.
    pub fn new(match_score: i32, mismatch_score: i32, gap_open_score: i32, reference: &Vec<u8>) -> Self {
        Aligner {
            traceback: Traceback::new(),
            query: reference.to_vec(),
            poa: Poa::from_string(match_score, mismatch_score, gap_open_score, reference),
        }
    }
    /// Add the alignment of the last query to the graph.
    pub fn add_to_graph(&mut self) -> &mut Self {
        let alignment = self.traceback.alignment();
        self.poa.add_alignment(&alignment, &self.query);
        self
    }

    /// Return alignment of last added query against the graph.
    pub fn alignment(&self) -> Alignment {
        self.traceback.alignment()
    }

    /// Globally align a given query against the graph.
    pub fn global(&mut self, query: &Vec<u8>) -> &mut Self {
        self.query = query.to_vec();
        self.traceback = self.poa.custom(query);
        self
    }
    pub fn global_simd(&mut self, query: &Vec<u8>) -> &mut Self {
        self.query = query.to_vec();
        self.poa.custom_simd(query);
        self
    }
    /// Return alignment graph.
    pub fn graph(&self) -> &POAGraph {
        &self.poa.graph
    }
    /// Return the consensus sequence generated from the POA graph.
    pub fn consensus(&self) -> Vec<u8> {
        let mut consensus: Vec<u8> = vec![];
        let max_index = self.poa.graph.node_count();
        let mut weight_score_next_vec: Vec<(i32, i32, usize)> = vec![(0, 0, 0); max_index + 1];
        let mut topo = Topo::new(&self.poa.graph);
        // go through the nodes topologically
        while let Some(node) = topo.next(&self.poa.graph) {
            let mut best_weight_score_next: (i32, i32, usize) = (0, 0, usize::MAX);
            let neighbour_nodes = self.poa.graph.neighbors_directed(node, Incoming);
            // go through the incoming neighbour nodes
            for neighbour_node in neighbour_nodes {
                let neighbour_index = neighbour_node.index();
                let neighbour_score = weight_score_next_vec[neighbour_index].1;
                let edges = self.poa.graph.edges_connecting(neighbour_node, node);
                let weight = edges.map(|edge| edge.weight()).sum();
                let current_node_score = weight + neighbour_score;
                // save the neighbour node with the highest weight and score as best
                if (weight, current_node_score, neighbour_index) > best_weight_score_next {
                    best_weight_score_next = (weight, current_node_score, neighbour_index);
                }
            }
            weight_score_next_vec[node.index()] = best_weight_score_next;
        }
        // get the index of the max scored node (end of consensus)
        let mut pos = weight_score_next_vec
            .iter()
            .enumerate()
            .max_by_key(|(_, &value)| value.1)
            .map(|(idx, _)| idx)
            .unwrap();
        // go through weight_score_next_vec appending to the consensus
        while pos != usize::MAX {
            consensus.push(self.poa.graph.raw_nodes()[pos].weight);
            pos = weight_score_next_vec[pos].2;
        }
        consensus.reverse();
        consensus
    }
}

/// A partially ordered alignment graph
///
/// A directed acyclic graph datastructure that represents the topology of a
/// traceback matrix.
#[derive(Default, Clone, Debug)]
pub struct Poa {
    match_score: i32,
    mismatch_score: i32,
    gap_open_score: i32,
    pub graph: POAGraph,
    pub memory_usage: usize,
}

impl Poa {
    /// Create a new POA graph from an initial reference sequence and alignment penalties.
    ///
    /// # Arguments
    ///
    /// * `scoring` - the score struct
    /// * `reference` - a reference TextSlice to populate the initial reference graph
    pub fn from_string(match_score: i32, mismatch_score: i32, gap_open_score: i32, seq: &Vec<u8>) -> Self {
        let mut graph: Graph<u8, i32, Directed, usize> =
            Graph::with_capacity(seq.len(), seq.len() - 1);
        let mut prev: NodeIndex<usize> = graph.add_node(seq[0]);
        let mut node: NodeIndex<usize>;
        for base in seq.iter().skip(1) {
            node = graph.add_node(*base);
            graph.add_edge(prev, node, 1);
            prev = node;
        }
        Poa { match_score: match_score, mismatch_score: mismatch_score, gap_open_score: gap_open_score, graph, memory_usage: 0}
    }

    pub fn profile_query (seq_y: &Vec<u8>, match_score: i32, mismatch_score: i32) -> Vec<Vec<i16x8>> {
        let num_seq_vec = seq_y.len() / 8;
        let mut MM_simd = vec![];
        // make 4 vectors for query
        let mut A_simd: Vec<i16x8> = vec![];
        let mut C_simd: Vec<i16x8> = vec![];
        let mut G_simd: Vec<i16x8> = vec![];
        let mut T_simd: Vec<i16x8> = vec![];
        // go through the query and populate the entries
        for index_simd in 0..num_seq_vec {
            let mut temp_A = vec![];
            let mut temp_C = vec![];
            let mut temp_G = vec![];
            let mut temp_T = vec![];
            let simd_seq = &seq_y[(index_simd * 8)..((index_simd + 1) * 8)];
            for base in simd_seq {
                if *base == 65 {
                    temp_A.push(match_score as i16);
                }
                else {
                    temp_A.push(mismatch_score as i16);
                }
                if *base == 67 {
                    temp_C.push(match_score as i16);
                }
                else {
                    temp_C.push(mismatch_score as i16);
                }
                if *base == 71 {
                    temp_G.push(match_score as i16);
                }
                else {
                    temp_G.push(mismatch_score as i16);
                }
                if *base == 84 {
                    temp_T.push(match_score as i16);
                }
                else {
                    temp_T.push(mismatch_score as i16);
                }
            }
            A_simd.push(i16x8::from_array(temp_A[0..8].try_into().expect("")));
            C_simd.push(i16x8::from_array(temp_C[0..8].try_into().expect("")));
            G_simd.push(i16x8::from_array(temp_G[0..8].try_into().expect("")));
            T_simd.push(i16x8::from_array(temp_T[0..8].try_into().expect("")));
            //println!("{:?}", simd_seq);
        }
        MM_simd.push(A_simd);
        MM_simd.push(C_simd);
        MM_simd.push(G_simd);
        MM_simd.push(T_simd);
        MM_simd
    }

    pub fn custom_simd(&mut self, query: &Vec<u8>) {
        println!("simd");
        // profile the query and what not
        let mut hash_table = HashMap::new();
        hash_table.insert(65, 0);
        hash_table.insert(67, 1);
        hash_table.insert(71, 2);
        hash_table.insert(84, 3);
        let MM_simd_full = Poa::profile_query(query, self.match_score, self.mismatch_score);
        // other simd stuff required
        let gap_open_score = self.gap_open_score as i16;
        let gap_open_8 = i16x8::from_array([gap_open_score, gap_open_score, gap_open_score, gap_open_score, gap_open_score, gap_open_score, gap_open_score, gap_open_score]);
        let left_mask_1 = i16x8::from_array([0, 1, 1, 1, 1, 1, 1, 1]);
        let left_mask_1_neg = i16x8::from_array([i16::MIN, 1, 1, 1, 1, 1, 1, 1]);
        let right_mask_7 = i16x8::from_array([1, 0, 0, 0, 0, 0, 0, 0]);
        assert!(self.graph.node_count() != 0);
        // dimensions of the traceback matrix
        let (m, n) = (self.graph.node_count(), query.len());
        println!("query.len() {}", query.len());
        let num_seq_vec = query.len() / 8;
        let mut HH: Vec<Vec<i16x8>> = vec![];
        //initialize HH with simd vecs, HH is used as traceback
        for i in 0..m {
            let mut temp_vec = vec![];
            for j in 0..num_seq_vec as i16 {
                if i == 0 {
                    let gap_open_multiplier = i16x8::from_array([(j * 8) + 1, (j * 8) + 2, (j * 8) + 3, (j * 8) + 4, (j * 8) + 5, (j * 8) + 6, (j * 8) + 7, (j * 8) + 8]) * -gap_open_8;
                    temp_vec.push(gap_open_multiplier);
                }
                else {
                    temp_vec.push(i16x8::from_array([i16::MIN, i16::MIN, i16::MIN, i16::MIN, i16::MIN, i16::MIN, i16::MIN, i16::MIN]));
                }
            }
            HH.push(temp_vec);
        }
        
        // construct the score matrix (O(n^2) space)
        let mut index = 0;
        let mut topo = Topo::new(&self.graph);
        while let Some(node) = topo.next(&self.graph) {
            
            let mut F = i16x8::from_array([0, 0, 0, 0, 0, 0, 0, (index + 1) * -gap_open_score]);
            
            // reference base and index
            let r = self.graph.raw_nodes()[node.index()].weight;
            let i = node.index(); // 0 index is for initialization so we start at 1
            let data_base_index = hash_table.get(&r).unwrap();
            // iterate over the predecessors of this node
            let mut prevs: Vec<NodeIndex<usize>> = self.graph.neighbors_directed(node, Incoming).collect();
            // add node index i (self referencing if no prev)
            if prevs.len() == 0 {
                prevs.push(node);
            }
            // vertical and diagonal
            for prev_node in &prevs {
                let i_p: usize = prev_node.index(); // index of previous node
                println!("S");
                let mut X = i16x8::from_array([(index) * -gap_open_score, 0, 0, 0, 0, 0, 0, 0]);
                for simd_index in 0..num_seq_vec {
                    let mut H_prev = HH[i_p][simd_index].clone();
                    println!("H_prev {:?}", H_prev);
                    let mut H_curr;
                    // when no prevs, start
                    if i_p == i {
                        H_curr = i16x8::from_array([i16::MIN, i16::MIN, i16::MIN, i16::MIN, i16::MIN, i16::MIN, i16::MIN, i16::MIN]);
                    }
                    else {
                        H_curr = HH[i][simd_index].clone();
                    }
                    let mut E = HH[i_p][simd_index].clone() - gap_open_8;
                    let MM_simd = MM_simd_full[*data_base_index][simd_index];
                    println!("MM simd {:?}", MM_simd);
                    // need to define T2 as H cannot be modified here
                    let T1 = H_prev.rotate_elements_left::<7>() * right_mask_7;
                    let mut T2 = (H_prev.rotate_elements_right::<1>() * left_mask_1) + X;
                    println!("X {:?}", X);
                    X = T1;
                    // match score added
                    println!("T2 {:?}", T2);
                    T2 = T2 + MM_simd;
                    
                    println!("E {:?}", E);
                    // diagonal or horizontal
                    H_curr = H_curr.simd_max(T2);
                    H_curr = H_curr.simd_max(E);
                    HH[i][simd_index] = H_curr;
                }
            }
            // horizontal NEeds fixing
            for simd_index in 0..num_seq_vec {
                let mut H = HH[i][simd_index].clone();
                println!("H after ver {:?}", H);
                F = F.rotate_elements_left::<7>() * right_mask_7;
                F = (H.rotate_elements_right::<1>() * left_mask_1) + F + gap_open_8;
                println!("F before {:?}", F);
                let mut T3 = F;
                for _iter in 0..8 {
                    // lshift 2 t2, for gap extends
                    T3 = T3.rotate_elements_right::<1>() * left_mask_1_neg;
                    F = F.simd_max(T3);
                }
                // make simd of gap open gap extend 
                
                F = F ;
                
                H = H.simd_max(F);
                F = H;
                HH[i][simd_index] = H;
                print!("{:?}", HH[i][simd_index]);
            }
            println!("");
            index += 1;
    }
}

    pub fn custom(&mut self, query: &Vec<u8>) -> Traceback {
        println!("Non simd");
        assert!(self.graph.node_count() != 0);
        // dimensions of the traceback matrix
        let (m, n) = (self.graph.node_count(), query.len());
        // save score location of the max scoring node for the query for suffix clipping
        let mut traceback = Traceback::with_capacity(m, n);
        traceback.initialize_scores(self.gap_open_score);
        // construct the score matrix (O(n^2) space)
        let mut topo = Topo::new(&self.graph);
        while let Some(node) = topo.next(&self.graph) {
            // reference base and index
            let r = self.graph.raw_nodes()[node.index()].weight; // reference base at previous index
            let i = node.index() + 1; // 0 index is for initialization so we start at 1
            traceback.last = node;
            // iterate over the predecessors of this node
            let prevs: Vec<NodeIndex<usize>> =
                self.graph.neighbors_directed(node, Incoming).collect();
            traceback.new_row(
                i,
                n + 1,
                self.gap_open_score,
                0,
                n + 1,
            );
            // query base and its index in the DAG (traceback matrix rows)
            for (query_index, query_base) in query.iter().enumerate() {
                let j = query_index + 1; // 0 index is initialized so we start at 1
                                         // match and deletion scores for the first reference base
                let max_cell = if prevs.is_empty() {
                    let temp_score;
                    if r == *query_base {
                        temp_score = self.match_score;
                    }
                    else {
                        temp_score = self.mismatch_score;
                    }
                    TracebackCell {
                        score: traceback.get(0, j - 1).score + temp_score,
                        op: AlignmentOperation::Match(None),
                    }
                } else {
                    let mut max_cell = 
                        TracebackCell {
                            score: MIN_SCORE,
                            op: AlignmentOperation::Match(None),
                        };
                    for prev_node in &prevs {
                        let i_p: usize = prev_node.index() + 1; // index of previous node
                        let temp_score;
                        if r == *query_base {
                            temp_score = self.match_score;
                        }
                        else {
                            temp_score = self.mismatch_score;
                        }
                        max_cell = max(
                            max_cell,
                            max(
                                TracebackCell {
                                    score: traceback.get(i_p, j - 1).score
                                        + temp_score,
                                    op: AlignmentOperation::Match(Some((i_p - 1, i - 1))),
                                },
                                TracebackCell {
                                    score: traceback.get(i_p, j).score + self.gap_open_score,
                                    op: AlignmentOperation::Del(Some((i_p - 1, i))),
                                },
                            ),
                        );
                    }
                    max_cell
                };
                let score = max(
                    max_cell,
                    TracebackCell {
                        score: traceback.get(i, j - 1).score + self.gap_open_score,
                        op: AlignmentOperation::Ins(Some(i - 1)),
                    },
                );
                traceback.set(i, j, score);
            }
        }
        // print the matrix here
        for i in 0..m {
            for j in 0..n {
                print!(" {}", traceback.get(i, j). score);
            }
            println!("");
        }
        //println!("Total {}KB", (total_cell_usage * mem::size_of::<TracebackCell>()) / 1024);
        traceback
    }
    /// Incorporate a new sequence into a graph from an alignment
    ///
    /// # Arguments
    ///
    /// * `aln` - The alignment of the new sequence to the graph
    /// * `seq` - The sequence being incorporated
    pub fn add_alignment(&mut self, aln: &Alignment, seq: &Vec<u8>) {
        let head = Topo::new(&self.graph).next(&self.graph).unwrap();
        let mut prev: NodeIndex<usize> = NodeIndex::new(head.index());
        let mut i: usize = 0;
        let mut edge_not_connected: bool = false;
        for op in aln.operations.iter() {
            match op {
                AlignmentOperation::Match(None) => {
                    let node: NodeIndex<usize> = NodeIndex::new(head.index());
                    if (seq[i] != self.graph.raw_nodes()[head.index()].weight) && (seq[i] != b'X') {
                        let node = self.graph.add_node(seq[i]);
                        if edge_not_connected {
                            self.graph.add_edge(prev, node, 1);
                        }
                        edge_not_connected = false;
                        prev = node;
                    }
                    if edge_not_connected {
                        self.graph.add_edge(prev, node, 1);
                        prev = node;
                        edge_not_connected = false;
                    }
                    i += 1;
                }
                AlignmentOperation::Match(Some((_, p))) => {
                    let node = NodeIndex::new(*p);
                    if (seq[i] != self.graph.raw_nodes()[*p].weight) && (seq[i] != b'X') {
                        let node = self.graph.add_node(seq[i]);
                        self.graph.add_edge(prev, node, 1);
                        prev = node;
                    } else {
                        // increment node weight
                        match self.graph.find_edge(prev, node) {
                            Some(edge) => {
                                *self.graph.edge_weight_mut(edge).unwrap() += 1;
                            }
                            None => {
                                if prev.index() != head.index() && prev.index() != node.index() {
                                    self.graph.add_edge(prev, node, 1);
                                }
                            }
                        }
                        prev = NodeIndex::new(*p);
                    }
                    i += 1;
                }
                AlignmentOperation::Ins(None) => {
                    let node = self.graph.add_node(seq[i]);
                    if edge_not_connected {
                        self.graph.add_edge(prev, node, 1);
                    }
                    prev = node;
                    edge_not_connected = true;
                    i += 1;
                }
                AlignmentOperation::Ins(Some(_)) => {
                    let node = self.graph.add_node(seq[i]);
                    self.graph.add_edge(prev, node, 1);
                    prev = node;
                    i += 1;
                }
                AlignmentOperation::Del(_) => {} // we should only have to skip over deleted nodes and xclip
                AlignmentOperation::Xclip(_) => {}
                AlignmentOperation::Yclip(_, r) => {
                    i = *r;
                }
            }
        }
    }
}