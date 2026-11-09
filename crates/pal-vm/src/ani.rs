//! SoftPAL sprite motion programs (`.ani`).
//!
//! Recovered from trenka.exe (iroha): loader 0x4473E0, interpreter 0x4466A0,
//! per-frame driver 0x446FE0/0x447060, clear 0x447180 and the task callbacks
//! at 0x402FF0..0x404440. Koikake's files use the same encoding; Sena's old
//! single-instruction reader covered only its `fc 13 fd <op>` prefix.
//!
//! A program is a byte stream of one-byte opcodes with little-endian i32
//! operands. The last dword of the file is the entry offset; label `n` is the
//! dword at `len - 0xC - 4 * n`. Running a thread executes instructions until
//! one of them waits; time-based instructions spawn tasks that add their
//! progress to a per-frame transient delta and, on completion, to a committed
//! delta. Clearing a program whose thread executed `0x13` folds the committed
//! delta into the sprite's base state; otherwise the effect is discarded.

use std::sync::Arc;

/// Instructions executed by one thread per frame. Native has no bound; a
/// shipped program always waits inside its loops, so this only stops
/// malformed input from hanging the frame.
const INSTRUCTION_BUDGET: usize = 4096;
const CHILD_SLOTS: usize = 8;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AniError {
    Truncated,
    EntryOutOfRange(i32),
}

impl std::fmt::Display for AniError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Truncated => write!(f, "motion program is shorter than its trailer"),
            Self::EntryOutOfRange(entry) => {
                write!(f, "motion program entry {entry} is out of range")
            }
        }
    }
}

impl std::error::Error for AniError {}

#[derive(Clone, Debug)]
pub struct AniProgram {
    bytes: Arc<[u8]>,
    entry: u32,
}

impl AniProgram {
    pub fn parse(bytes: &[u8]) -> Result<Self, AniError> {
        let code_len = bytes
            .len()
            .checked_sub(4)
            .filter(|len| *len > 0)
            .ok_or(AniError::Truncated)?;
        let entry = i32::from_le_bytes(bytes[code_len..].try_into().unwrap());
        if entry < 0 || entry as usize >= code_len {
            return Err(AniError::EntryOutOfRange(entry));
        }
        Ok(Self {
            bytes: bytes.into(),
            entry: entry as u32,
        })
    }

    fn code_len(&self) -> usize {
        self.bytes.len() - 4
    }

    fn opcode(&self, pc: u32) -> Option<u8> {
        if (pc as usize) < self.code_len() {
            self.bytes.get(pc as usize).copied()
        } else {
            None
        }
    }

    fn operands<const N: usize>(&self, pc: u32) -> Option<[i32; N]> {
        let start = pc as usize;
        let end = start.checked_add(N * 4)?;
        if end > self.code_len() {
            return None;
        }
        let mut out = [0; N];
        for (index, value) in out.iter_mut().enumerate() {
            let at = start + index * 4;
            *value = i32::from_le_bytes(self.bytes[at..at + 4].try_into().unwrap());
        }
        Some(out)
    }

    fn label(&self, index: i32) -> Option<u32> {
        let offset = (self.bytes.len() as i64) - 0xC - 4 * i64::from(index);
        if offset < 0 || offset + 4 > self.bytes.len() as i64 {
            return None;
        }
        let offset = offset as usize;
        let target = i32::from_le_bytes(self.bytes[offset..offset + 4].try_into().unwrap());
        (target >= 0 && (target as usize) < self.code_len()).then_some(target as u32)
    }
}

/// Animated sprite deltas, mirroring the native record lanes: integer
/// position (+0x178) and rotation (+0x184), float scale (+0x1A0) and 16-bit
/// A/R/G/B color words (+0x1A4).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MotionDelta {
    pub position: [i32; 3],
    pub rotation: [i32; 3],
    pub scale: f32,
    /// Alpha, red, green, blue.
    pub color: [i16; 4],
}

impl MotionDelta {
    pub fn add(&mut self, other: &Self) {
        for axis in 0..3 {
            self.position[axis] = self.position[axis].wrapping_add(other.position[axis]);
            self.rotation[axis] = self.rotation[axis].wrapping_add(other.rotation[axis]);
        }
        self.scale += other.scale;
        for channel in 0..4 {
            self.color[channel] = self.color[channel].wrapping_add(other.color[channel]);
        }
    }

    pub fn is_zero(&self) -> bool {
        *self == Self::default()
    }
}

/// Side effects the interpreter asks the sprite owner to apply.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AniEffect {
    /// `PalSpriteRectSetPos(sprite, cell_x, cell_y)` (opcode 0x0C).
    RectSetPos(i32, i32),
    /// `PalSpriteSetRect(sprite, left, top, right, bottom)` (opcode 0x0D).
    SetRect([i32; 4]),
}

pub struct AniContext<'a> {
    pub now_ms: u32,
    /// Native skip mode (flag 0x10): waits finish their tasks immediately.
    pub skipping: bool,
    /// Sprite base position used by the absolute move (opcode 0x11).
    pub base_position: [i32; 3],
    pub random: &'a mut dyn FnMut() -> u32,
    pub effects: &'a mut Vec<AniEffect>,
}

#[derive(Clone, Debug)]
enum Task {
    Wait {
        start: u32,
        duration: Option<u32>,
    },
    Move {
        mode: i32,
        delta: [i32; 3],
        start: u32,
        duration: u32,
    },
    Rotate {
        delta: [i32; 3],
        start: u32,
        duration: u32,
    },
    Scale {
        delta: i32,
        start: u32,
        duration: u32,
    },
    Alpha {
        delta: i32,
        start: u32,
        duration: u32,
    },
    Toward {
        target: [i32; 2],
        step: [f32; 2],
        current: [f32; 2],
        interval: u32,
        last: u32,
    },
    Wave {
        params: [i32; 8],
        start: u32,
        duration: u32,
        commit: bool,
    },
}

fn progress(now: u32, start: u32, duration: u32) -> (u32, f32) {
    let elapsed = now.wrapping_sub(start).min(duration);
    (elapsed, elapsed as f32 / duration as f32)
}

/// Native easing selectors of the move task (0x403880); bit 2 is tested
/// first, then bit 1, then bit 4.
fn ease(mode: i32, elapsed: u32, duration: u32) -> f32 {
    let duration_f = duration as f32;
    if mode & 2 != 0 {
        let x = f64::from(elapsed as f32 / duration_f).log10() as f32 * 80.0 * -0.01;
        1.0 - 1.0f32.min(x)
    } else if mode & 1 != 0 {
        let x = f64::from((duration - elapsed) as f32 / duration_f).log10() as f32 * 55.0 * -0.01;
        1.0f32.min(x)
    } else if mode & 4 != 0 {
        let x = elapsed as f32 / duration_f * 128.0 * 0.003_906_25 * std::f32::consts::PI;
        1.0f32.min(f64::from(x).sin() as f32)
    } else {
        1.0f32.min(elapsed as f32 / duration_f)
    }
}

fn scaled(value: i32, factor: f32) -> i32 {
    (value as f32 * factor) as i32
}

fn wave_offset(params: &[i32; 8], factor: f32) -> [i32; 2] {
    let sine = |phase: i32| (std::f64::consts::TAU * f64::from(phase & 0x1FF) / 512.0).sin() as f32;
    let amp_x = params[2].wrapping_add(scaled(params[6], factor));
    let phase_x = params[0].wrapping_add(scaled(params[4], factor));
    let amp_y = params[3].wrapping_add(scaled(params[7], factor));
    let phase_y = scaled(params[5], factor)
        .wrapping_add(params[1])
        .wrapping_add(0x80);
    [
        (amp_x as f32 * sine(phase_x)) as i32,
        (amp_y as f32 * sine(phase_y)) as i32,
    ]
}

impl Task {
    /// Advance one frame. Returns false once the task is finished.
    fn update(
        &mut self,
        now: u32,
        waiting: &mut bool,
        committed: &mut MotionDelta,
        transient: &mut MotionDelta,
    ) -> bool {
        match self {
            Self::Wait { start, duration } => {
                let done = duration.is_some_and(|duration| now.wrapping_sub(*start) >= duration);
                if done {
                    *waiting = false;
                }
                !done
            }
            Self::Move {
                mode,
                delta,
                start,
                duration,
            } => {
                let elapsed = now.wrapping_sub(*start).min(*duration);
                let factor = ease(*mode, elapsed, *duration);
                let target = if elapsed >= *duration {
                    committed
                } else {
                    transient
                };
                for (value, &step) in target.position.iter_mut().zip(delta.iter()) {
                    *value = value.wrapping_add(scaled(step, factor));
                }
                elapsed < *duration
            }
            Self::Rotate {
                delta,
                start,
                duration,
            } => {
                let (elapsed, factor) = progress(now, *start, *duration);
                let target = if elapsed >= *duration {
                    committed
                } else {
                    transient
                };
                for (value, &step) in target.rotation.iter_mut().zip(delta.iter()) {
                    *value = value.wrapping_add(scaled(step, factor));
                }
                elapsed < *duration
            }
            Self::Scale {
                delta,
                start,
                duration,
            } => {
                let (elapsed, factor) = progress(now, *start, *duration);
                let target = if elapsed >= *duration {
                    committed
                } else {
                    transient
                };
                target.scale += factor * *delta as f32 / 100.0;
                elapsed < *duration
            }
            Self::Alpha {
                delta,
                start,
                duration,
            } => {
                let (elapsed, factor) = progress(now, *start, *duration);
                let target = if elapsed >= *duration {
                    committed
                } else {
                    transient
                };
                target.color[0] = target.color[0].wrapping_add(scaled(*delta, factor) as i16);
                elapsed < *duration
            }
            Self::Toward {
                target,
                step,
                current,
                interval,
                last,
            } => {
                if now.wrapping_sub(*last) >= *interval {
                    *last = now;
                    current[0] += step[0];
                    current[1] += step[1];
                    if (current[0] + 0.5) as i32 == target[0]
                        && (current[1] + 0.5) as i32 == target[1]
                    {
                        committed.position[0] = committed.position[0].wrapping_add(target[0]);
                        committed.position[1] = committed.position[1].wrapping_add(target[1]);
                        return false;
                    }
                }
                transient.position[0] = transient.position[0].wrapping_add(current[0] as i32);
                transient.position[1] = transient.position[1].wrapping_add(current[1] as i32);
                true
            }
            Self::Wave {
                params,
                start,
                duration,
                commit,
            } => {
                let (elapsed, factor) = progress(now, *start, *duration);
                let [x, y] = wave_offset(params, factor.min(1.0));
                let target = if elapsed >= *duration && *commit {
                    committed
                } else {
                    transient
                };
                target.position[0] = target.position[0].wrapping_add(x);
                target.position[1] = target.position[1].wrapping_add(y);
                elapsed < *duration
            }
        }
    }

    /// Skip-mode completion callbacks (the task's second function pointer).
    fn finish(&self, committed: &mut MotionDelta) {
        match self {
            Self::Wait { .. } => {}
            // 0x403820 commits x and y only.
            Self::Move { delta, .. } => {
                committed.position[0] = committed.position[0].wrapping_add(delta[0]);
                committed.position[1] = committed.position[1].wrapping_add(delta[1]);
            }
            Self::Rotate { delta, .. } => {
                for (value, &step) in committed.rotation.iter_mut().zip(delta.iter()) {
                    *value = value.wrapping_add(step);
                }
            }
            Self::Scale { delta, .. } => committed.scale += *delta as f32 / 100.0,
            Self::Alpha { delta, .. } => {
                committed.color[0] = committed.color[0].wrapping_add(*delta as i16)
            }
            Self::Toward { target, .. } => {
                committed.position[0] = committed.position[0].wrapping_add(target[0]);
                committed.position[1] = committed.position[1].wrapping_add(target[1]);
            }
            Self::Wave { params, commit, .. } => {
                if *commit {
                    let [x, y] = wave_offset(params, 1.0);
                    committed.position[0] = committed.position[0].wrapping_add(x);
                    committed.position[1] = committed.position[1].wrapping_add(y);
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
struct AniThread {
    pc: u32,
    /// Native state 1: the `0xFC` prologue runs until `0xFD`.
    prologue: bool,
    /// Native state 3: a wait instruction is pending.
    waiting: bool,
    persist: bool,
    tasks: Vec<Task>,
}

enum RunResult {
    Yield,
    End,
}

impl AniThread {
    fn new(pc: u32) -> Self {
        Self {
            pc,
            prologue: false,
            waiting: false,
            persist: false,
            tasks: Vec::new(),
        }
    }

    fn finish_tasks(&mut self, committed: &mut MotionDelta) {
        for task in self.tasks.drain(..) {
            task.finish(committed);
        }
        self.waiting = false;
    }

    fn tick(&mut self, now: u32, committed: &mut MotionDelta, transient: &mut MotionDelta) {
        let mut waiting = self.waiting;
        self.tasks
            .retain_mut(|task| task.update(now, &mut waiting, committed, transient));
        self.waiting = waiting;
    }

    fn run(
        &mut self,
        program: &AniProgram,
        ctx: &mut AniContext<'_>,
        committed: &mut MotionDelta,
        children: &mut [Option<AniThread>; CHILD_SLOTS],
    ) -> RunResult {
        for _ in 0..INSTRUCTION_BUDGET {
            let Some(op) = program.opcode(self.pc) else {
                return RunResult::End;
            };
            let pc = self.pc + 1;
            self.pc = pc;
            let now = ctx.now_ms;
            macro_rules! args {
                ($n:literal) => {{
                    let Some(args) = program.operands::<$n>(pc) else {
                        return RunResult::End;
                    };
                    self.pc = pc + $n * 4;
                    args
                }};
            }
            let nonzero = |duration: i32| (duration as u32).max(1);
            match op {
                0x00 | 0x0E | 0x0F | 0x12 => {
                    let [x, y, z, duration] = args!(4);
                    let mode = match op {
                        0x0E => 1,
                        0x0F => 2,
                        0x12 => 4,
                        _ => 0,
                    };
                    self.tasks.push(Task::Move {
                        mode,
                        delta: [x, y, z],
                        start: now,
                        duration: nonzero(duration),
                    });
                }
                0x11 => {
                    let [mode, x, y, z, duration] = args!(5);
                    let base = ctx.base_position;
                    let delta = [
                        x.wrapping_sub(base[0]),
                        y.wrapping_sub(base[1]),
                        z.wrapping_sub(base[2]),
                    ];
                    self.tasks.push(Task::Move {
                        mode,
                        delta,
                        start: now,
                        duration: nonzero(duration),
                    });
                }
                0x01 => {
                    let [duration] = args!(1);
                    self.tasks.push(Task::Wait {
                        start: now,
                        duration: (duration != -1).then_some(duration as u32),
                    });
                    self.waiting = true;
                }
                0x02 => {
                    let [delta, duration] = args!(2);
                    self.tasks.push(Task::Scale {
                        delta,
                        start: now,
                        duration: nonzero(duration),
                    });
                }
                0x03 => {
                    let [delta, duration] = args!(2);
                    self.tasks.push(Task::Alpha {
                        delta,
                        start: now,
                        duration: nonzero(duration),
                    });
                }
                0x04 => {
                    let [x, y, z, duration] = args!(4);
                    self.tasks.push(Task::Rotate {
                        delta: [x, y, z],
                        start: now,
                        duration: nonzero(duration),
                    });
                }
                0x07 => {
                    let delta = args!(3);
                    for (value, &step) in committed.position.iter_mut().zip(delta.iter()) {
                        *value = value.wrapping_add(step);
                    }
                }
                0x08 => {
                    let [low, high] = args!(2);
                    let span = high.wrapping_sub(low) as u32;
                    let offset = if span == 0 { 0 } else { (ctx.random)() % span };
                    self.tasks.push(Task::Wait {
                        start: now,
                        duration: Some(offset.wrapping_add(low as u32)),
                    });
                    self.waiting = true;
                }
                0x0A => {
                    let [label] = args!(1);
                    let Some(target) = program.label(label) else {
                        return RunResult::End;
                    };
                    self.pc = target;
                    if ctx.skipping {
                        return RunResult::Yield;
                    }
                }
                0x0B => {
                    let [slot, label] = args!(2);
                    if let (Some(child), Some(target)) =
                        (children.get_mut(slot as u32 as usize), program.label(label))
                    {
                        *child = Some(AniThread::new(target));
                    }
                }
                0x0C => {
                    let [x, y] = args!(2);
                    ctx.effects.push(AniEffect::RectSetPos(x, y));
                }
                0x0D => {
                    let [x, y, width, height] = args!(4);
                    ctx.effects.push(AniEffect::SetRect([
                        x,
                        y,
                        x.wrapping_add(width),
                        y.wrapping_add(height),
                    ]));
                }
                0x13 => self.persist = true,
                0x14 => {
                    let [x, y, interval] = args!(3);
                    let angle = (y as f32).atan2(x as f32);
                    self.tasks.push(Task::Toward {
                        target: [x, y],
                        step: [angle.cos(), angle.sin()],
                        current: [0.0; 2],
                        interval: interval as u32,
                        last: now,
                    });
                    // Native sets the wait state and no task ever clears it.
                    self.waiting = true;
                }
                0x15 | 0x16 => {
                    let [a, b, c, d, e, f, g, h, duration] = args!(9);
                    self.tasks.push(Task::Wave {
                        params: [a, b, c, d, e, f, g, h],
                        start: now,
                        duration: nonzero(duration),
                        commit: op == 0x15,
                    });
                }
                0xFC => {}
                0xFD => {
                    if self.prologue {
                        self.prologue = false;
                        return RunResult::Yield;
                    }
                }
                0xFE => {
                    let [duration] = args!(1);
                    self.tasks.push(Task::Wait {
                        start: now,
                        duration: Some(duration as u32),
                    });
                    self.waiting = true;
                }
                _ => return RunResult::End,
            }
            if ctx.skipping && self.waiting {
                self.finish_tasks(committed);
            }
            if self.waiting {
                return RunResult::Yield;
            }
        }
        log::warn!(
            "[trace-ani] instruction budget exhausted at pc=0x{:X}",
            self.pc
        );
        RunResult::Yield
    }
}

/// A running motion program bound to one sprite.
#[derive(Clone, Debug)]
pub struct AniPlayer {
    program: AniProgram,
    main: AniThread,
    children: [Option<AniThread>; CHILD_SLOTS],
    committed: MotionDelta,
    transient: MotionDelta,
}

impl AniPlayer {
    /// Load and start a program: the loader runs a `0xFC` prologue, then the
    /// owner (sp_set, btn_set, ...) immediately resumes it until it waits.
    /// Returns None when the program ends before its first wait.
    pub fn start(program: AniProgram, ctx: &mut AniContext<'_>) -> Option<Self> {
        let mut player = Self {
            main: AniThread::new(program.entry),
            program,
            children: Default::default(),
            committed: MotionDelta::default(),
            transient: MotionDelta::default(),
        };
        player.main.prologue = player.program.opcode(player.program.entry) == Some(0xFC);
        if player.main.prologue {
            if let RunResult::End = player.run_main(ctx) {
                return None;
            }
            player.main.prologue = false;
        }
        match player.run_main(ctx) {
            RunResult::End => None,
            RunResult::Yield => Some(player),
        }
    }

    fn run_main(&mut self, ctx: &mut AniContext<'_>) -> RunResult {
        self.main
            .run(&self.program, ctx, &mut self.committed, &mut self.children)
    }

    /// One native frame (0x447060 children, then 0x446FE0 main). Returns
    /// false when the main thread ended; the caller then clears the player.
    pub fn advance(&mut self, ctx: &mut AniContext<'_>) -> bool {
        self.transient = MotionDelta::default();
        for slot in 0..CHILD_SLOTS {
            let Some(mut child) = self.children[slot].take() else {
                continue;
            };
            let mut grandchildren: [Option<AniThread>; CHILD_SLOTS] = Default::default();
            if !child.waiting
                && matches!(
                    child.run(&self.program, ctx, &mut self.committed, &mut grandchildren),
                    RunResult::End
                )
            {
                continue;
            }
            child.tick(ctx.now_ms, &mut self.committed, &mut self.transient);
            self.children[slot] = Some(child);
        }
        if !self.main.waiting && matches!(self.run_main(ctx), RunResult::End) {
            return false;
        }
        self.main
            .tick(ctx.now_ms, &mut self.committed, &mut self.transient);
        true
    }

    /// Committed plus in-flight deltas for this frame.
    pub fn delta(&self) -> MotionDelta {
        let mut total = self.committed;
        total.add(&self.transient);
        total
    }

    /// The delta native 0x447180 folds into the sprite base when cleared.
    pub fn persisted_delta(&self) -> Option<MotionDelta> {
        self.main.persist.then_some(self.committed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn program(code: &[u8], labels: &[i32]) -> AniProgram {
        let mut bytes = code.to_vec();
        // Trailer: labels (highest first), one unused dword, entry offset.
        for label in labels.iter().rev() {
            bytes.extend_from_slice(&label.to_le_bytes());
        }
        bytes.extend_from_slice(&0i32.to_le_bytes());
        bytes.extend_from_slice(&0i32.to_le_bytes());
        AniProgram::parse(&bytes).unwrap()
    }

    fn op(code: u8, args: &[i32]) -> Vec<u8> {
        let mut out = vec![code];
        for arg in args {
            out.extend_from_slice(&arg.to_le_bytes());
        }
        out
    }

    struct Harness {
        effects: Vec<AniEffect>,
        random: Box<dyn FnMut() -> u32>,
    }

    impl Harness {
        fn new() -> Self {
            Self {
                effects: Vec::new(),
                random: Box::new(|| 0),
            }
        }

        fn ctx(&mut self, now_ms: u32) -> AniContext<'_> {
            AniContext {
                now_ms,
                skipping: false,
                base_position: [0; 3],
                random: &mut *self.random,
                effects: &mut self.effects,
            }
        }
    }

    #[test]
    fn trailer_selects_entry_and_labels() {
        let code = [op(0xFE, &[10]), op(0x0A, &[0])].concat();
        let program = program(&code, &[0]);
        assert_eq!(program.entry, 0);
        assert_eq!(program.label(0), Some(0));
        assert_eq!(program.label(5), None);
        assert!(AniProgram::parse(&[1, 0, 0, 0]).is_err());
    }

    #[test]
    fn alpha_tween_waits_and_loops_like_the_line_effect() {
        // fd; alpha +255/100ms; wait 100; alpha -255/100ms; wait 100; jmp 0
        let code = [
            vec![0xFD],
            op(0x03, &[255, 100]),
            op(0xFE, &[100]),
            op(0x03, &[-255, 100]),
            op(0xFE, &[100]),
            op(0x0A, &[0]),
        ]
        .concat();
        let mut harness = Harness::new();
        let mut player = AniPlayer::start(program(&code, &[0]), &mut harness.ctx(0)).unwrap();
        assert!(player.advance(&mut harness.ctx(50)));
        assert_eq!(player.delta().color[0], 127);
        assert!(player.advance(&mut harness.ctx(100)));
        assert_eq!(player.delta().color[0], 255);
        // The wait completes this frame; the next frame starts the fade out.
        assert!(player.advance(&mut harness.ctx(101)));
        assert!(player.advance(&mut harness.ctx(151)));
        assert_eq!(player.delta().color[0], 255 - 127);
        assert!(player.advance(&mut harness.ctx(201)));
        assert_eq!(player.delta().color[0], 0);
        assert!(player.persisted_delta().is_none());
    }

    #[test]
    fn persist_flag_and_end_report_committed_delta() {
        let code = [
            vec![0xFC, 0x13, 0xFD],
            op(0x00, &[10, -4, 0, 100]),
            op(0xFE, &[100]),
            vec![0xFF],
        ]
        .concat();
        let mut harness = Harness::new();
        let mut player = AniPlayer::start(program(&code, &[]), &mut harness.ctx(0)).unwrap();
        assert!(player.advance(&mut harness.ctx(50)));
        assert_eq!(player.delta().position, [5, -2, 0]);
        assert!(player.advance(&mut harness.ctx(100)));
        assert!(!player.advance(&mut harness.ctx(116)));
        assert_eq!(player.persisted_delta().unwrap().position, [10, -4, 0]);
    }

    #[test]
    fn easing_modes_match_the_native_curves() {
        assert_eq!(ease(0, 50, 100), 0.5);
        assert_eq!(ease(2, 0, 100), 0.0);
        assert_eq!(ease(2, 100, 100), 1.0);
        assert!((ease(1, 50, 100) - (0.55 * 2f32.log10())).abs() < 1e-6);
        assert!((ease(4, 50, 100) - (std::f32::consts::FRAC_PI_4).sin()).abs() < 1e-6);
    }

    #[test]
    fn rect_effects_and_scale_follow_the_program() {
        let code = [
            vec![0xFD],
            op(0x0C, &[3, 0]),
            op(0x0D, &[1, 2, 10, 20]),
            op(0x02, &[-30, 10]),
            op(0xFE, &[0]),
        ]
        .concat();
        let mut harness = Harness::new();
        let mut player = AniPlayer::start(program(&code, &[]), &mut harness.ctx(0)).unwrap();
        assert_eq!(
            harness.effects,
            [
                AniEffect::RectSetPos(3, 0),
                AniEffect::SetRect([1, 2, 11, 22])
            ]
        );
        assert!(player.advance(&mut harness.ctx(10)));
        assert!((player.delta().scale + 0.3).abs() < 1e-6);
    }

    #[test]
    fn wave_without_commit_returns_to_rest() {
        let code = [
            vec![0xFD, 0x13],
            op(0x16, &[0, 0, 0, 10, 0, 256, 0, 0, 100]),
            op(0xFE, &[200]),
        ]
        .concat();
        let mut harness = Harness::new();
        let mut player = AniPlayer::start(program(&code, &[]), &mut harness.ctx(0)).unwrap();
        assert!(player.advance(&mut harness.ctx(0)));
        assert_eq!(player.delta().position[1], 10);
        assert!(player.advance(&mut harness.ctx(150)));
        assert_eq!(player.persisted_delta().unwrap().position, [0, 0, 0]);
    }

    #[test]
    fn child_threads_share_the_sprite_deltas() {
        let code = [
            vec![0xFD],
            op(0x0B, &[0, 1]),
            op(0xFE, &[1000]),
            vec![0xFF],
            // label 1: child moves x by 8 at once.
            op(0x07, &[8, 0, 0]),
            vec![0xFF],
        ]
        .concat();
        let child_pc = 1 + 9 + 5 + 1;
        let mut harness = Harness::new();
        let mut player =
            AniPlayer::start(program(&code, &[0, child_pc]), &mut harness.ctx(0)).unwrap();
        assert!(player.advance(&mut harness.ctx(16)));
        assert_eq!(player.delta().position[0], 8);
    }

    #[test]
    fn malformed_jump_loop_is_bounded() {
        let code = op(0x0A, &[0]);
        let mut harness = Harness::new();
        assert!(AniPlayer::start(program(&code, &[0]), &mut harness.ctx(0)).is_some());
    }
}
