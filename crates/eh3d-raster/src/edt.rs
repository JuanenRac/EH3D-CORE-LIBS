//! The exact Euclidean distance from every cell to the nearest copper cell (Felzenszwalb and Huttenlocher: one pass along the columns and one along the rows).

const INF: f32 = 1.0e20;

/// One dimension: `d[q]` is the minimum over `p` of `(q - p)^2 + f[p]`.
fn line(f: &[f32], d: &mut [f32], v: &mut [usize], z: &mut [f32]) {
    let n = f.len();
    let mut k = 0usize;
    v[0] = 0;
    z[0] = -INF;
    z[1] = INF;
    for q in 1..n {
        loop {
            let (qf, vf) = (q as f32, v[k] as f32);
            let s = ((f[q] + qf * qf) - (f[v[k]] + vf * vf)) / (2.0 * qf - 2.0 * vf);
            if s <= z[k] && k > 0 {
                k -= 1;
            } else {
                if s <= z[k] {
                    // k is 0 and the new parabola is below the first one everywhere: it replaces it
                    v[0] = q;
                    z[1] = INF;
                } else {
                    k += 1;
                    v[k] = q;
                    z[k] = s;
                    z[k + 1] = INF;
                }
                break;
            }
        }
    }
    k = 0;
    for (q, out) in d.iter_mut().enumerate() {
        while z[k + 1] < q as f32 {
            k += 1;
        }
        let diff = q as f32 - v[k] as f32;
        *out = diff * diff + f[v[k]];
    }
}

/// The squared distance, in cells, from each cell to the nearest `on` cell (0 on the copper itself). With no copper at all every value is huge.
pub fn squared_distance(on: &[bool], w: usize, h: usize) -> Vec<f32> {
    let mut g: Vec<f32> = on.iter().map(|&b| if b { 0.0 } else { INF }).collect();
    let n = w.max(h);
    let (mut f, mut d, mut v, mut z) = (
        vec![0f32; n],
        vec![0f32; n],
        vec![0usize; n],
        vec![0f32; n + 1],
    );
    for x in 0..w {
        for y in 0..h {
            f[y] = g[y * w + x];
        }
        line(&f[..h], &mut d[..h], &mut v, &mut z);
        for y in 0..h {
            g[y * w + x] = d[y];
        }
    }
    for y in 0..h {
        f[..w].copy_from_slice(&g[y * w..(y + 1) * w]);
        line(&f[..w], &mut d[..w], &mut v, &mut z);
        g[y * w..(y + 1) * w].copy_from_slice(&d[..w]);
    }
    g
}

/// The distance in millimetres.
pub fn distance_mm(on: &[bool], w: usize, h: usize, res: f64) -> Vec<f32> {
    squared_distance(on, w, h)
        .into_iter()
        .map(|d2| {
            if d2 >= INF / 2.0 {
                f32::MAX
            } else {
                (f64::from(d2).sqrt() * res) as f32
            }
        })
        .collect()
}
