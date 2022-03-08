use std::{cmp, collections::HashMap};
use std::fs::File;
use std::io::{prelude::*, BufWriter};

pub fn local_alignment(s1: &Vec<char>, s2: &Vec<char>, matrix: &HashMap<(char, char), i32>) {
    let mut a = vec![vec![0; s2.len()]; s1.len()];
    let mut path =vec![vec!['x';s2.len()];s1.len()];
    
    for row in 0..s1.len() {
        for col in 0..s2.len() {
            match (row, col) {
                (_, 0)|(0, _) => {
                    a[row][col] = 0;
                    path[row][col] = 'O';
                },
                _ => {
                    let d = a[row-1][col-1] + matrix.get(&(s1[row], s2[col])).unwrap();
                    let l = a[row-1][col] + matrix.get(&(s1[row], '-')).unwrap();
                    let u = a[row][col-1] + matrix.get(&('-', s2[col])).unwrap();

                    if d < 0 && l < 0 && u < 0 {
                        a[row][col] = 0;
                        path[row][col] = 'O';
                    }
                    match d.cmp(&l) {
                       cmp::Ordering::Less => {
                           match l.cmp(&u) {
                               cmp::Ordering::Less => {
                                   a[row][col] = u;
                                   path[row][col] = 'U'
                               },
                               _ => {
                                   a[row][col] = l;
                                   path[row][col] = 'L'
                               },
                           }
                       },
                       _ => {
                           match d.cmp(&u) {
                               cmp::Ordering::Less => {
                                   a[row][col] = u;
                                   path[row][col] = 'U'
                               },
                               _ => {
                                   a[row][col] = d;
                                   if s1[row] == s2[col] {
                                       path[row][col] = 'D'
                                   } else {
                                       path[row][col] = 'd'
                                   }

                               }
                           }
                       } 
                    }
                }
            }
        }
    }
    let (max_row, max_col) = get_max(&a);
    println!(
        "Local Alignement: {}",
        a[max_row][max_col]
    );
    write_alignment(
        &path,
        max_row,
        max_col,
        s1,
        s2,
    )
}

fn get_max(a: &[Vec<i32>]) -> (usize, usize) {
    let mut max_row = 0;
    let mut max_col = 0;

    for i in 0..a.len() {
        for j in 0..a[0].len() {
            if a[i][j] >= a[max_row][max_col] {
                max_row = i;
                max_col = j;
            }
        }
    }
    (max_row, max_col)
}

fn write_alignment(
    path: &[Vec<char>],
    mut row: usize,
    mut col: usize,
    s1: &[char],
    s2: &[char],
) {
    let mut s1_align = String::new();
    let mut s2_align = String::new();
    let mut alignment_moves = String::new();

    while path[row][col] != 'O' {
        
        match path[row][col] {
            'D' => {
                s1_align.push(s1[row]);
                s2_align.push(s2[col]);
                alignment_moves.push('|');
                row -= 1;
                col -= 1;
            }
            'd' => {
                s1_align.push(s1[row]);
                s2_align.push(s2[col]);
                alignment_moves.push('.');
                col -= 1;
                row -= 1;
            }
            'U' => {
                s1_align.push(s1[row]);
                s2_align.push('-');
                alignment_moves.push(' ');
                row -= 1;
            }
            'L' => {
                s1_align.push('-');
                s2_align.push(s2[col]);
                alignment_moves.push(' ');
                col -= 1;
            }
            _ => panic!("ampl_is_enough panic"),
        }
    }

    s1_align = s1_align.chars().rev().collect();
    alignment_moves = alignment_moves.chars().rev().collect();
    s2_align = s2_align.chars().rev().collect();
    let file_name = "local_alignment.txt";

    let path = project_root::get_project_root().unwrap().join(file_name);
    let f = File::create(path).expect("unable to create file");
    let mut f = BufWriter::new(f);

    writeln!(f, "{}", s1_align).expect("unable to write");
    writeln!(f, "{}", alignment_moves).expect("unable to write");
    writeln!(f, "{}", s2_align).expect("unable to write");
}