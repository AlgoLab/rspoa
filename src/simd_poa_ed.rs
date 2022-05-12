use std::{arch::x86_64::*, cmp};

use crate::graph::LnzGraph;
pub fn exec_no_simd(read: &Vec<u8>, graph: &LnzGraph) -> f32 {
    let mut m: Vec<Vec<f32>> = vec![vec![0f32; read.len()]; graph.lnz.len()];
    let mut path: Vec<Vec<f32>> = vec![vec![0f32; read.len()]; graph.lnz.len()];
    for i in 1..graph.lnz.len() - 1 {
        if !graph.nwp[i] {
            m[i][0] = m[i - 1][0] + 1f32;
            path[i][0] = (i - 1) as f32 + 0.2;
        } else {
            let pred = graph.pred_hash.get(&i).unwrap();
            let best_p = pred.iter().min().unwrap();
            m[i][0] = m[*best_p][0] + 1f32;
            path[i][0] = *best_p as f32 + 0.2;
        }
    }
    for j in 1..read.len() {
        m[0][j] = j as f32;
        path[0][j] = 0.3;
    }
    for i in 1..graph.lnz.len() - 1 {
        for j in 1..read.len() {
            if !graph.nwp[i] {
                let l = m[i][j - 1] + 1f32;
                let u = m[i - 1][j] + 1f32;
                let d = m[i - 1][j - 1]
                    + if read[j] == graph.lnz[i] as u8 {
                        0f32
                    } else {
                        1f32
                    };

                m[i][j] = [l, u, d].into_iter().reduce(f32::min).unwrap();
                if m[i][j] == d {
                    path[i][j] = (i - 1) as f32 + 0.1;
                } else if m[i][j] == u {
                    path[i][j] = (i - 1) as f32 + 0.2;
                } else {
                    path[i][j] = i as f32 + 0.3;
                }
            } else {
                let mut u = 0f32;
                let mut u_pred = 0;
                let mut d = 0f32;
                let mut d_pred = 0;
                let mut first = true;
                for p in graph.pred_hash.get(&i).unwrap() {
                    if first {
                        u = m[*p][j];
                        d = m[*p][j - 1];
                        u_pred = *p;
                        d_pred = *p;
                        first = false
                    }
                    if m[*p][j] < u {
                        u = m[*p][j];
                        u_pred = *p;
                    }
                    if m[*p][j - 1] < d {
                        d = m[*p][j - 1];
                        d_pred = *p;
                    }
                }
                u += 1f32;
                d += if read[j] == graph.lnz[i] as u8 {
                    0f32
                } else {
                    1f32
                };
                let l = m[i][j - 1] + 1f32;

                m[i][j] = [l, u, d].into_iter().reduce(f32::min).unwrap();

                if m[i][j] == d {
                    path[i][j] = d_pred as f32 + 0.1;
                } else if m[i][j] == u {
                    path[i][j] = u_pred as f32 + 0.2;
                } else {
                    path[i][j] = i as f32 + 0.3;
                }
            }
        }
    }
    let mut best_result = 0f32;
    let mut first = true;
    for p in graph.pred_hash.get(&(m.len() - 1)).unwrap().iter() {
        if first {
            best_result = m[*p][read.len() - 1];
            first = false;
        }
        if m[*p][read.len() - 1] < best_result {
            best_result = m[*p][read.len() - 1];
        }
    }
    best_result
}

#[target_feature(enable = "avx2")]
pub unsafe fn exec(read: &Vec<u8>, graph: &LnzGraph) -> f32 {
    let mut m: Vec<Vec<f32>> = vec![vec![0f32; read.len()]; graph.lnz.len()];
    let mut path: Vec<Vec<f32>> = vec![vec![0f32; read.len()]; graph.lnz.len()];
    for i in 1..graph.lnz.len() - 1 {
        if !graph.nwp[i] {
            m[i][0] = m[i - 1][0] + 1f32;
            path[i][0] = (i - 1) as f32 + 0.2;
        } else {
            let pred = graph.pred_hash.get(&i).unwrap();
            let best_p = pred.iter().min().unwrap();
            m[i][0] = m[*best_p][0] + 1f32;
            path[i][0] = *best_p as f32 + 0.2;
        }
    }
    for j in 1..read.len() {
        m[0][j] = j as f32;
        path[0][j] = 0.3;
    }

    let max_multiple = (read.len() / 8) * 8;
    let read_f32 = &read[0..max_multiple + 1]
        .iter()
        .map(|c| *c as f32)
        .collect::<Vec<f32>>();
    let one_simd = _mm256_set1_ps(1.0);

    for i in 1..graph.lnz.len() - 1 {
        for j in (1..max_multiple + 1).step_by(8) {
            if !graph.nwp[i] {
                let us = _mm256_add_ps(_mm256_loadu_ps(m[i - 1].get_unchecked(j)), one_simd);

                let eq_char = _mm256_cmp_ps(
                    _mm256_loadu_ps(read_f32.get_unchecked(j)), //read chars simd
                    _mm256_set1_ps(graph.lnz[i] as u8 as f32),  // reference char simd
                    _CMP_EQ_OS,
                );
                let neq_ds =
                    _mm256_add_ps(_mm256_loadu_ps(m[i - 1].get_unchecked(j - 1)), one_simd);
                let eq_ds = _mm256_loadu_ps(m[i - 1].get_unchecked(j - 1));
                let ds = _mm256_blendv_ps(neq_ds, eq_ds, eq_char);

                let best_choice = _mm256_cmp_ps(ds, us, _CMP_LT_OS);
                let result = _mm256_blendv_ps(us, ds, best_choice);

                _mm256_storeu_ps(m[i].get_unchecked_mut(j), result);

                //path update
                
                let dir_result = _mm256_blendv_ps(_mm256_set1_ps(0.2), _mm256_set1_ps(0.1), best_choice);
                let path_update = _mm256_add_ps(_mm256_set1_ps((i - 1) as f32), dir_result);
                _mm256_storeu_ps(path[i].get_unchecked_mut(j), path_update);

            // TODO: update path for multiple predecessor
            } else {
                let preds = graph.pred_hash.get(&i).unwrap();
                let mut best_ds = _mm256_set1_ps(0f32);
                let mut best_us = _mm256_set1_ps(0f32);
                let mut pred_best_us = _mm256_set1_ps(0f32);
                let mut pred_best_ds = _mm256_set1_ps(0f32);
                let mut first = true;
                for p in preds {
                    let us = _mm256_loadu_ps(m[*p].get_unchecked(j));

                    let ds = _mm256_loadu_ps(m[*p].get_unchecked(j - 1));
                    let preds = _mm256_set1_ps(*p as f32);
                    if first {
                        first = false;
                        best_us = us;
                        best_ds = ds;
                        pred_best_us = preds;
                        pred_best_ds = preds;
                    } else {
                        let best_us_choices = _mm256_cmp_ps(us, best_us, _CMP_LT_OS);
                        best_us = _mm256_blendv_ps(best_us, us, best_us_choices);
                        pred_best_us = _mm256_blendv_ps(pred_best_us, preds, best_us_choices);

                        let best_ds_choices = _mm256_cmp_ps(ds, best_ds, _CMP_LT_OS);
                        best_ds = _mm256_blendv_ps(best_ds, ds, best_ds_choices);
                        pred_best_ds = _mm256_blendv_ps(pred_best_ds, preds, best_ds_choices);
                    }
                }
                best_us = _mm256_add_ps(best_us, one_simd);

                let eq_char = _mm256_cmp_ps(
                    _mm256_loadu_ps(read_f32.get_unchecked(j)), //read chars simd
                    _mm256_set1_ps(graph.lnz[i] as u8 as f32),  // reference char simd
                    _CMP_EQ_OS,
                );
                let neq_ds = _mm256_add_ps(best_ds, one_simd);
                best_ds = _mm256_blendv_ps(neq_ds, best_ds, eq_char);

                let best_choice = _mm256_cmp_ps(best_ds, best_us, _CMP_LT_OS);
                let result = _mm256_blendv_ps(best_us, best_ds, best_choice);
                _mm256_storeu_ps(m[i].get_unchecked_mut(j), result);

                pred_best_ds = _mm256_add_ps(pred_best_ds, _mm256_set1_ps(0.1));
                pred_best_us = _mm256_add_ps(pred_best_us, _mm256_set1_ps(0.2));

                let dir_result = _mm256_blendv_ps(pred_best_us, pred_best_ds, best_choice);
                _mm256_storeu_ps(path[i].get_unchecked_mut(j), dir_result);
            }

            // update with l for each one
            for idx in j..cmp::min(j + 8, read.len()) {
                if m[i][idx - 1] + 1f32 < m[i][idx] {
                    m[i][idx] = m[i][idx - 1] + 1f32;
                    path[i][idx] = i as f32 + 0.3;
                }
            }
        }
        for j in max_multiple + 1..read.len() {
            if !graph.nwp[i] {
                let l = m[i][j - 1] + 1f32;
                let u = m[i - 1][j] + 1f32;
                let d = m[i - 1][j - 1]
                    + if read[j] == graph.lnz[i] as u8 {
                        0f32
                    } else {
                        1f32
                    };

                m[i][j] = [l, u, d].into_iter().reduce(f32::min).unwrap();
                if m[i][j] == d {
                    path[i][j] = (i - 1) as f32 + 0.1;
                } else if m[i][j] == u {
                    path[i][j] = (i - 1) as f32 + 0.2;
                } else {
                    path[i][j] = i as f32 + 0.3;
                }
            } else {
                let mut u = 0f32;
                let mut u_pred = 0;
                let mut d = 0f32;
                let mut d_pred = 0;
                let mut first = true;
                for p in graph.pred_hash.get(&i).unwrap() {
                    if first {
                        u = m[*p][j];
                        d = m[*p][j - 1];
                        u_pred = *p;
                        d_pred = *p;
                        first = false
                    }
                    if m[*p][j] < u {
                        u = m[*p][j];
                        u_pred = *p;
                    }
                    if m[*p][j - 1] < d {
                        d = m[*p][j - 1];
                        d_pred = *p;
                    }
                }
                u += 1f32;
                d += if read[j] == graph.lnz[i] as u8 {
                    0f32
                } else {
                    1f32
                };
                let l = m[i][j - 1] + 1f32;

                m[i][j] = [l, u, d].into_iter().reduce(f32::min).unwrap();

                if m[i][j] == d {
                    path[i][j] = d_pred as f32 + 0.1;
                } else if m[i][j] == u {
                    path[i][j] = u_pred as f32 + 0.2;
                } else {
                    path[i][j] = i as f32 + 0.3;
                }
            }
        }
    }
    let mut best_result = 0f32;
    let mut first = true;
    for p in graph.pred_hash.get(&(m.len() - 1)).unwrap().iter() {
        if first {
            best_result = m[*p][read.len() - 1];
            first = false;
        }
        if m[*p][read.len() - 1] < best_result {
            best_result = m[*p][read.len() - 1];
        }
    }
    //rebuild_path(&path);
    best_result
}

fn rebuild_path(path: &Vec<Vec<f32>>) {
    for j in 0..path[0].len() {
        let val = path[0][j];
        let pred = val as usize;
        let dir = val - (val as i32) as f32;
        match (dir * 10f32) as i32 {
            1 | 2 | 3 => {
                println!("ok")
            }
            _ => {
                println!(" ERROR:  pred: {pred} dir: {dir}");
            }
        }
    }
    for i in 0..path.len() - 1 {
        let val = path[i][0];
        let pred = val as usize;
        let dir = val - (val as i32) as f32;
        match (dir * 10f32) as i32 {
            1 | 2 | 3 => {
                println!("ok")
            }
            _ => {
                println!(" ERROR:  pred: {pred} dir: {dir}");
            }
        }
    }
}
/*
path [i][j] = pred(int) + dir(decimal)
D = 0.1
U = 0.2
L = 0.3
*/
