fn lcs(a: &[String], b: &[String]) -> Vec<String> {
    let (m, n) = (a.len(), b.len());
    let mut dp = vec![vec![0u32; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            dp[i][j] = if a[i-1] == b[j-1] { dp[i-1][j-1] + 1 }
                       else { dp[i-1][j].max(dp[i][j-1]) };
        }
    }
    let mut result = Vec::new();
    let (mut i, mut j) = (m, n);
    while i > 0 && j > 0 {
        if a[i-1] == b[j-1] { result.push(a[i-1].clone()); i -= 1; j -= 1; }
        else if dp[i-1][j] >= dp[i][j-1] { i -= 1; }
        else { j -= 1; }
    }
    result.reverse();
    result
}

pub fn diff3_merge(base: &[String], local: &[String], remote: &[String]) -> (Vec<String>, bool) {
    let local_chunks  = diff_chunks(base, local);
    let remote_chunks = diff_chunks(base, remote);
    let mut result = Vec::new();
    let mut has_conflict = false;
    let mut base_pos = 0usize;
    let mut li = 0usize;
    let mut ri = 0usize;

    loop {
        let lc = local_chunks.get(li);
        let rc = remote_chunks.get(ri);

        match (lc, rc) {
            (None, None) => {
                result.extend_from_slice(&base[base_pos..]);
                break;
            }
            (Some(l), None) => {
                result.extend_from_slice(&base[base_pos..l.base_start]);
                result.extend_from_slice(&l.lines);
                base_pos = l.base_end;
                li += 1;
            }
            (None, Some(r)) => {
                result.extend_from_slice(&base[base_pos..r.base_start]);
                result.extend_from_slice(&r.lines);
                base_pos = r.base_end;
                ri += 1;
            }
            (Some(l), Some(r)) => {
                // emit unchanged base up to whichever chunk comes first
                let next = l.base_start.min(r.base_start);
                result.extend_from_slice(&base[base_pos..next]);
                base_pos = next;

                if l.base_start == r.base_start {
                    // both sides changed the same region
                    if l.lines == r.lines {
                        // same change — convergence, take it
                        result.extend_from_slice(&l.lines);
                    } else {
                        // genuine conflict
                        has_conflict = true;
                        result.push("<<<<<<< LOCAL".to_string());
                        result.extend_from_slice(&l.lines);
                        result.push("=======".to_string());
                        result.extend_from_slice(&r.lines);
                        result.push(">>>>>>> REMOTE".to_string());
                    }
                    base_pos = l.base_end.max(r.base_end);
                    li += 1;
                    ri += 1;
                } else if l.base_start < r.base_start {
                    result.extend_from_slice(&l.lines);
                    base_pos = l.base_end;
                    li += 1;
                } else {
                    result.extend_from_slice(&r.lines);
                    base_pos = r.base_end;
                    ri += 1;
                }
            }
        }
    }
    (result, has_conflict)
}

struct Chunk {
    base_start: usize,
    base_end:   usize,
    lines:      Vec<String>,
}

fn diff_chunks(base: &[String], new: &[String]) -> Vec<Chunk> {
    let lcs = lcs(base, new);
    let mut chunks = Vec::new();
    let mut bi = 0usize;
    let mut ni = 0usize;
    let mut lcs_i = 0usize;

    while bi < base.len() || ni < new.len() {
        // skip matching lines
        if lcs_i < lcs.len()
            && bi < base.len()
            && ni < new.len()
            && base[bi] == lcs[lcs_i]
            && new[ni] == lcs[lcs_i]
        {
            bi += 1; ni += 1; lcs_i += 1;
            continue;
        }

        let base_start = bi;
        let mut new_lines = Vec::new();

        // consume differing lines until both re-sync with LCS
        while bi < base.len() || ni < new.len() {
            let base_synced = lcs_i < lcs.len() && bi < base.len() && base[bi] == lcs[lcs_i];
            let new_synced  = lcs_i < lcs.len() && ni < new.len()  && new[ni]  == lcs[lcs_i];
            if base_synced && new_synced { break; }
            if !base_synced && bi < base.len() { bi += 1; }
            if !new_synced  && ni < new.len()  { new_lines.push(new[ni].clone()); ni += 1; }
        }

        if base_start != bi || !new_lines.is_empty() {
            chunks.push(Chunk { base_start, base_end: bi, lines: new_lines });
        }
    }
    chunks
}