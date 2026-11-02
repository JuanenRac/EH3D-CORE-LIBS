//! The order in which the paths are cut, the direction round each of them and the point where each one starts. The paths are traced with the copper on the right of the tool
//! (the tool going clockwise round an island of copper, which is climb milling with a spindle turning clockwise); the settings can turn that round, force clockwise or
//! counterclockwise, choose where each closed path starts and shorten the travel of the tool through the air.

type Pt = (f64, f64);

/// One piece of cutting: a closed path (the last point joins the first) or an open run.
#[derive(Debug, Clone, PartialEq)]
pub struct Cut {
    pub pts: Vec<Pt>,
    pub closed: bool,
}

impl Cut {
    fn valid(&self) -> bool {
        self.pts.len() >= if self.closed { 3 } else { 2 }
            && self.pts.iter().all(|p| p.0.is_finite() && p.1.is_finite())
    }

    fn exit(&self) -> Pt {
        if self.closed {
            self.pts[0]
        } else {
            self.pts[self.pts.len() - 1]
        }
    }

    /// The length of the cut, in mm.
    pub fn length_mm(&self) -> f64 {
        let mut sum: f64 = self.pts.windows(2).map(|w| dist(w[0], w[1])).sum();
        if self.closed {
            sum += dist(self.pts[self.pts.len() - 1], self.pts[0]);
        }
        sum
    }
}

fn dist(a: Pt, b: Pt) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

/// The signed area of a closed path: positive when it goes counterclockwise.
pub fn signed_area(pts: &[Pt]) -> f64 {
    let n = pts.len();
    (0..n)
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum::<f64>()
        / 2.0
}

pub fn is_clockwise(pts: &[Pt]) -> bool {
    signed_area(pts) < 0.0
}

/// Which way round the paths go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// The copper on the right of the tool: with a spindle turning clockwise the tool climbs into the cut.
    Climb,
    /// The copper on the left of the tool: the tool is pushed against the cut.
    Conventional,
    /// Every closed path clockwise whatever the copper (open runs keep their way).
    Clockwise,
    CounterClockwise,
}

impl Direction {
    pub fn parse(v: &str) -> Result<Direction, String> {
        match v.trim().to_ascii_lowercase().as_str() {
            "climb" | "concordant" | "concordante" => Ok(Direction::Climb),
            "conventional" | "convencional" => Ok(Direction::Conventional),
            "clockwise" | "cw" => Ok(Direction::Clockwise),
            "counterclockwise" | "counter_clockwise" | "ccw" => Ok(Direction::CounterClockwise),
            other => Err(format!(
                "direction '{other}': it must be climb, conventional, clockwise or counterclockwise"
            )),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Direction::Climb => "climb",
            Direction::Conventional => "conventional",
            Direction::Clockwise => "clockwise",
            Direction::CounterClockwise => "counterclockwise",
        }
    }
}

/// Where a closed path starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopStart {
    /// At its point nearest to where the tool is.
    Nearest,
    /// At its lowest point (the leftmost of them): the same place every time.
    LowerLeft,
    /// Where the path was traced.
    Original,
}

impl LoopStart {
    pub fn parse(v: &str) -> Result<LoopStart, String> {
        match v.trim().to_ascii_lowercase().as_str() {
            "nearest" => Ok(LoopStart::Nearest),
            "lower_left" | "lower-left" | "lowerleft" => Ok(LoopStart::LowerLeft),
            "original" => Ok(LoopStart::Original),
            other => Err(format!(
                "loop_start '{other}': it must be nearest, lower_left or original"
            )),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            LoopStart::Nearest => "nearest",
            LoopStart::LowerLeft => "lower_left",
            LoopStart::Original => "original",
        }
    }
}

/// How the paths of a pass are put in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    /// The order they were traced in.
    Original,
    /// Always the nearest one next.
    Nearest,
    /// The nearest one next, then stretches of the order turned round while that shortens the travel.
    Optimized,
}

impl Strategy {
    pub fn parse(v: &str) -> Result<Strategy, String> {
        match v.trim().to_ascii_lowercase().as_str() {
            "original" => Ok(Strategy::Original),
            "nearest" => Ok(Strategy::Nearest),
            "optimized" | "optimised" => Ok(Strategy::Optimized),
            other => Err(format!(
                "ordering '{other}': it must be original, nearest or optimized"
            )),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Strategy::Original => "original",
            Strategy::Nearest => "nearest",
            Strategy::Optimized => "optimized",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Order {
    pub direction: Direction,
    /// Where the tool is before the first cut, in mm.
    pub start: Pt,
    pub loop_start: LoopStart,
    pub strategy: Strategy,
    /// Above this many paths the turning round of stretches is skipped (it grows with the cube of the number).
    pub optimize_max_paths: usize,
}

impl Default for Order {
    fn default() -> Self {
        Order {
            direction: Direction::Climb,
            start: (0.0, 0.0),
            loop_start: LoopStart::Nearest,
            strategy: Strategy::Nearest,
            optimize_max_paths: 300,
        }
    }
}

fn nearest_vertex(c: &Cut, at: Pt) -> (usize, f64) {
    let mut best = (0, f64::INFINITY);
    for (i, p) in c.pts.iter().enumerate() {
        let d = dist(*p, at);
        if d < best.1 {
            best = (i, d);
        }
    }
    best
}

fn lower_left(c: &Cut) -> usize {
    let mut best = 0;
    for (i, p) in c.pts.iter().enumerate() {
        let b = c.pts[best];
        if p.1 < b.1 || (p.1 == b.1 && p.0 < b.0) {
            best = i;
        }
    }
    best
}

fn directed(c: &Cut, d: Direction) -> Cut {
    let mut c = c.clone();
    let reverse = match d {
        Direction::Climb => false,
        Direction::Conventional => true,
        Direction::Clockwise => c.closed && signed_area(&c.pts) > 0.0,
        Direction::CounterClockwise => c.closed && signed_area(&c.pts) < 0.0,
    };
    if reverse {
        c.pts.reverse();
    }
    c
}

/// The travel through the air that cutting `cuts` in this order takes, from `from`, in mm.
pub fn travel_mm(cuts: &[Cut], from: Pt) -> f64 {
    let mut at = from;
    let mut sum = 0.0;
    for c in cuts {
        if c.pts.is_empty() {
            continue;
        }
        sum += dist(at, c.pts[0]);
        at = c.exit();
    }
    sum
}

/// Puts the cuts in order, turns them the way the order says and starts each closed one where it says. Returns the cuts and the place where the tool ends.
pub fn arrange(cuts: &[Cut], o: &Order, from: Pt) -> (Vec<Cut>, Pt) {
    let mut items: Vec<Cut> = cuts
        .iter()
        .filter(|c| c.valid())
        .map(|c| directed(c, o.direction))
        .collect();
    if o.loop_start == LoopStart::LowerLeft {
        for c in items.iter_mut().filter(|c| c.closed) {
            let i = lower_left(c);
            c.pts.rotate_left(i);
        }
    }
    let mut at = from;
    let mut out: Vec<Cut> = Vec::with_capacity(items.len());
    match o.strategy {
        Strategy::Original => {
            for mut c in items {
                if c.closed && o.loop_start == LoopStart::Nearest {
                    let (i, _) = nearest_vertex(&c, at);
                    c.pts.rotate_left(i);
                }
                at = c.exit();
                out.push(c);
            }
        }
        Strategy::Nearest | Strategy::Optimized => {
            let mut left = items;
            while !left.is_empty() {
                let mut best = (0usize, f64::INFINITY);
                for (k, c) in left.iter().enumerate() {
                    let d = if c.closed && o.loop_start == LoopStart::Nearest {
                        nearest_vertex(c, at).1
                    } else {
                        dist(at, c.pts[0])
                    };
                    if d < best.1 {
                        best = (k, d);
                    }
                }
                let mut c = left.remove(best.0);
                if c.closed && o.loop_start == LoopStart::Nearest {
                    let (i, _) = nearest_vertex(&c, at);
                    c.pts.rotate_left(i);
                }
                at = c.exit();
                out.push(c);
            }
            if o.strategy == Strategy::Optimized
                && out.len() > 2
                && out.len() <= o.optimize_max_paths
            {
                two_opt(&mut out, from);
                at = out.last().map_or(from, Cut::exit);
            }
        }
    }
    (out, at)
}

fn two_opt(seq: &mut [Cut], from: Pt) {
    let n = seq.len();
    let mut best = travel_mm(seq, from);
    for _ in 0..50 {
        let mut improved = false;
        for i in 0..n - 1 {
            for j in i + 1..n {
                seq[i..=j].reverse();
                let c = travel_mm(seq, from);
                if c < best - 1e-9 {
                    best = c;
                    improved = true;
                } else {
                    seq[i..=j].reverse();
                }
            }
        }
        if !improved {
            break;
        }
    }
}

/// How many closed paths go clockwise and how many counterclockwise, and how many runs are open.
pub fn directions(cuts: &[Cut]) -> (usize, usize, usize) {
    let (mut cw, mut ccw, mut open) = (0, 0, 0);
    for c in cuts {
        if !c.closed {
            open += 1;
        } else if is_clockwise(&c.pts) {
            cw += 1;
        } else {
            ccw += 1;
        }
    }
    (cw, ccw, open)
}
