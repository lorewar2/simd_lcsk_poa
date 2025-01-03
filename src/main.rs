#![feature(portable_simd)]
#![allow(dead_code)]
mod poa;
mod pairwise;
use pairwise::*;
use poa::*;
use std::simd::{i32x4, Simd};

fn main() {
    let seq_x = vec![65, 65, 82];
    let seq_y = vec![65, 65, 82];
    let match_score = 1;
    let mismatch_score = -1;
    let gap_open_score = -2;
    let gap_extend_score = -1;
    let band_size = 10;
    let (result_vec, score) = pairwise(&seq_x, &seq_y, match_score, mismatch_score, gap_open_score, gap_extend_score, band_size);
    println!("{:?} {}", result_vec, score);
    /*
    // TEST SIMD Add two arrays of 4 elements each in parallel using SIMD
    // Create two arrays
    let a = [1, 2, 3, 4];
    let b = [5, 6, 7, 8];

    // Load the arrays into SIMD vectors (f32x4 can hold 4 floats in parallel)
    let simd_a = i32x4::from_array(a);
    let simd_b = i32x4::from_array(b);

    // Perform element-wise addition using SIMD
    let simd_result = simd_a + simd_b;

    // Extract the result back into a regular array
    let result: [i32; 4] = simd_result.to_array();

    // Print the result
    println!("Result: {:?}", result);
    
    // TEST POA
    let seqs = ["ACT", "AGT", "ACC", "ACT"];
    let mut seqs_bytes = vec![];
    for seq in seqs.iter() {
        seqs_bytes.push(seq.to_string().bytes().collect::<Vec<u8>>());
    }
    let mut aligner = Aligner::new(2, -2, -2, &seqs_bytes[0].to_vec(), 0, 0, 10 as i32);
    for seq in seqs_bytes.iter().skip(1) {
        aligner.global(seq).add_to_graph();
    }
    let consensus = aligner.consensus();
    println!("{:?}", consensus)
     */
}