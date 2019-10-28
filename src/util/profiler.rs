use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc};

lazy_static! {
    static ref PROFILER: Arc<Mutex<Profiler>> = Arc::new(Mutex::new(Profiler::new())) ;
}

#[macro_export]
macro_rules! profile {
    ( $x:expr ) => {
        {
            profiler::push("...");
            let result = $x;
            profiler::pop();
            result
        }
    };
    ( $n:expr, $x:expr ) => {
        {
            profiler::push($n);
            let result = $x;
            profiler::pop();
            result
        }
    };
}

pub const ROOT: Handle = 0usize;

pub fn push(name: &'static str) {
    PROFILER.lock()
        .unwrap()
        .push(name);
}

pub fn pop() {
    PROFILER.lock()
        .unwrap()
        .pop();
}

pub fn analysis() -> Analysis {
    PROFILER.lock()
        .unwrap()
        .analysis()
}

#[derive(Clone)]
pub struct TimeFrame {
    enter: DateTime<Utc>,
    exit: DateTime<Utc>,
}

impl TimeFrame {
    pub fn nanoseconds(&self) -> i64 {
        self.exit.timestamp_nanos() - self.enter.timestamp_nanos()
    }
}

pub type Handle = usize;

#[derive(Clone, Copy)]
struct Timer (Option<DateTime<Utc>>);

impl Timer {
    pub fn new() -> Self {
        Self (None)
    }
    pub fn reset(&mut self) {
        self.0 = None
    }
    pub fn set(&mut self, enter: DateTime<Utc>) -> DateTime<Utc> {
        self.0 = Some(enter);
        enter
    }
    pub fn enter(&mut self) -> DateTime<Utc> {
        if let Some(enter) = self.0 {
            panic!("timer already running: {:?}", enter)
        } else {
            self.set(Utc::now())
        }
    }
    pub fn exit(&mut self) -> TimeFrame {
        if let Some(enter) = self.0 {
            self.reset();
            TimeFrame { enter, exit: Utc::now() }
        } else {
            panic!("timer not running!")
        }
    }
}

#[derive(Clone)]
pub struct AnalyzedFrame {
    index: Handle,
    parent: Handle,
    total: i64,
    tally: Vec<(Handle, i64)>
}

impl AnalyzedFrame {
    pub fn from_frame(frame: &Frame) -> Self {
        Self { parent: frame.parent, index: frame.index, total: frame.time(), tally: Vec::new() }
    }
    pub fn total(&self) -> i64 {
        self.total
    }
    pub fn tally_up(&self) -> i64 {
        self.tally.iter()
            .fold(0i64, |mut sum, (_, val)| {
                sum += val;
                sum
            })
    }
    pub fn tally(&self) -> &Vec<(Handle, i64)> {
        &self.tally
    }
    pub fn subtally(&mut self, handle: Handle, tally: i64) {
        self.tally.push((handle, tally))
    }
    pub fn child_percent(&self, analysis: &Analysis) -> f64 {
        self.total as f64 / self.parent(analysis).1.total() as f64
    }
    pub fn total_percent(&self, analysis: &Analysis) -> f64 {
        self.total as f64 / analysis.total() as f64
    }
    pub fn parent<'a>(&self, analysis: &'a Analysis) -> (&'a Frame, &'a AnalyzedFrame) {
        let (frame, _) = &analysis.frame(self.index);
        analysis.frame(frame.parent)
    }
    pub fn frame<'a>(&self, profiler: &'a Profiler) -> &'a Frame {
        profiler.lookup(self.index)
    }
}

#[derive(Clone)]
pub struct Frame {
    name: &'static str,
    index: Handle,
    parent: Handle,
    timer: Timer,
    frame: Option<TimeFrame>,
}

impl Frame {
    pub fn new(index: Handle, parent: Handle, name: &'static str) -> Self {
        Self {
            name,
            index,
            parent,
            timer: Timer::new(),
            frame: None,
        }
    }
    pub fn index(&self) -> Handle {
        self.index
    }
    pub fn parent(&self) -> Handle {
        self.parent
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn enter(&mut self) -> DateTime<Utc> {
        self.timer.enter()
    }
    pub fn exit(&mut self) {
        self.frame = Some(self.timer.exit());
    }
    pub fn time(&self) -> i64 {
        if let Some(frame) = &self.frame {
            frame.nanoseconds()
        } else {
            panic!("bug: timer not ready")
        }
    }
}

#[derive(Clone)]
pub struct Analysis {
    frames: Vec<Frame>,
    analysis: Vec<AnalyzedFrame>,
}

impl Analysis {
    pub fn frame<'a>(&'a self, handle: Handle) -> (&'a Frame, &'a AnalyzedFrame) {
        (&self.frames[handle], &self.analysis[handle])
    }
    pub fn total(&self) -> i64 {
        self.analysis[0].total()
    }
}

#[derive(Clone)]
pub struct Profiler {
    frames: Vec<Frame>,
    top: Handle,
}

impl Profiler {

    pub fn new() -> Self {
        Self { frames: Vec::new(), top: 0usize }
    }

    fn untallied_analysis(&self) -> Vec<AnalyzedFrame> {
        self.frames.iter().map(AnalyzedFrame::from_frame).collect()
    }

    fn tallied_analysis(&self) -> Vec<AnalyzedFrame> {
        let mut analysis = self.untallied_analysis();
        for i in 1..analysis.len() {
            let parent = analysis[i].frame(self).parent;
            let total = analysis[i].total();
            analysis[parent].subtally(i, total)
        }
        analysis
    }

    pub fn analysis(&self) -> Analysis {
        let analysis = self.tallied_analysis();
        Analysis { frames: self.frames.clone(), analysis }
    }

    pub fn push(&mut self, name: &'static str) {
        let index = self.frames.len();
        let mut frame = Frame::new(index, self.top, name);
        frame.enter();
        self.frames.push(frame);
        self.top = index;
    }

    pub fn pop(&mut self) -> &Frame {
        let frame = &mut self.frames[self.top];
        frame.exit();
        self.top = frame.parent;
        frame
    }

    pub fn lookup(&self, handle: Handle) -> &Frame {
        self.frames.get(handle).unwrap()
    }
    pub fn lookup_mut(&mut self, handle: Handle) -> &mut Frame {
        self.frames.get_mut(handle).unwrap()
    }
}
