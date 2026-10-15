use raylib::prelude::*;

use crate::bag::Bag;
use crate::particle::{EffectType, ParticleSystem};
use crate::sound::SoundManager;

// --- Wymiary planszy (z grid.h) ---
pub const COLS: usize = 7;
pub const ROWS: usize = 14;
pub const CELL: f32 = 32.0;
pub const GRID_PIXEL_W: f32 = COLS as f32 * CELL;
pub const GRID_PIXEL_H: f32 = ROWS as f32 * CELL;

const DAS: f32 = 0.100;
const ARR: f32 = 0.05;

const MAX_LEVEL: i32 = 10;

const PIECE_HZ_TABLE: [f32; 10] = [2.0, 2.5, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0, 12.0, 15.0];
const SIM_HZ_TABLE: [f32; 10] = [7.0, 8.0, 9.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CellType {
    Empty,
    Sand,
}

#[derive(Clone, Copy)]
pub struct Cell {
    pub kind: CellType,
    pub tier: i32,
    pub fresh: bool,
}

impl Cell {
    const EMPTY: Self = Cell {
        kind: CellType::Empty,
        tier: 0,
        fresh: false,
    };

    fn sand(tier: i32) -> Self {
        Cell {
            kind: CellType::Sand,
            tier,
            fresh: true,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Piece {
    pub r: i32,
    pub c: i32,
    pub cell: Cell,
    pub active: bool,
}

impl Default for Piece {
    fn default() -> Self {
        Piece {
            r: 0,
            c: 0,
            cell: Cell::EMPTY,
            active: false,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FallDir {
    None,
    Down,
    Left,
    Right,
}

pub struct Grid {
    pub(crate) cells: [[Cell; COLS]; ROWS],
    pub(crate) piece: Piece,
    soft_drop: bool,

    pub(crate) bag: Bag,

    das_left: f32,
    arr_left: f32,
    das_right: f32,
    arr_right: f32,

    pub(crate) hold_tier: i32,
    pub(crate) can_hold: bool,

    score: i32,

    pub(crate) font: Font,

    sim_acc: f32,
    piece_acc: f32,

    pub(crate) screen_w: i32,
    pub(crate) screen_h: i32,
}

pub(crate) fn tier_color(tier: i32) -> Color {
    let hue = (tier as f32 / 10.0) * 270.0;
    Color::color_from_hsv(hue, 1.0, 1.0)
}

impl Grid {
    pub fn new(rl: &mut RaylibHandle, thread: &RaylibThread) -> Self {
        let font = rl
            .load_font_ex(thread, "assets/iosevka.ttf", CELL as i32, None)
            .expect("failed to load assets/iosevka.ttf");

        let screen_w = rl.get_screen_width();
        let screen_h = rl.get_screen_height();

        let mut grid = Grid {
            cells: [[Cell::EMPTY; COLS]; ROWS],
            piece: Piece::default(),
            soft_drop: false,
            bag: Bag::new(std::array::from_fn(|_| {
                rl.get_random_value::<i32>(0..i32::MAX)
            })),
            das_left: 0.0,
            arr_left: 0.0,
            das_right: 0.0,
            arr_right: 0.0,
            hold_tier: -1,
            can_hold: true,
            score: 0,
            font,
            sim_acc: 0.0,
            piece_acc: 0.0,
            screen_w,
            screen_h,
        };

        grid.piece_spawn(rl);

        grid
    }

    pub fn get_score(&self) -> i32 {
        self.score
    }

    pub fn input(
        &mut self,
        rl: &RaylibHandle,
        dt: f32,
        particles: &mut ParticleSystem,
        sound: &SoundManager,
    ) {
        if !self.piece.active {
            return;
        }
        self.soft_drop = rl.is_key_down(KeyboardKey::KEY_DOWN);

        if rl.is_key_pressed(KeyboardKey::KEY_SPACE) && self.grid_settled() {
            while self.piece.r + 1 < ROWS as i32
                && self.cells[(self.piece.r + 1) as usize][self.piece.c as usize].kind
                    == CellType::Empty
            {
                self.piece.r += 1;
            }
            self.cells[self.piece.r as usize][self.piece.c as usize] = self.piece.cell;
            self.cells[self.piece.r as usize][self.piece.c as usize].fresh = true;
            self.piece.active = false;
            particles.spawn(
                self.cell_sx(self.piece.c),
                self.cell_sy(self.piece.r) + CELL / 2.0,
                tier_color(self.piece.cell.tier),
                EffectType::Land,
                rl,
            );
            sound.play_land();
            self.piece_spawn(rl);
            self.can_hold = true;
            return;
        }

        if rl.is_key_pressed(KeyboardKey::KEY_C) && self.can_hold {
            if self.hold_tier == -1 {
                self.hold_tier = self.piece.cell.tier;
                self.piece_spawn(rl);
            } else {
                let tmp = self.hold_tier;
                self.hold_tier = self.piece.cell.tier;
                self.piece.cell.tier = tmp;
                self.piece.r = 0;
                self.piece.c = COLS as i32 / 2;
            }
            self.can_hold = false;
        }

        if rl.is_key_pressed(KeyboardKey::KEY_LEFT) {
            self.try_move(-1);
            self.das_left = 0.0;
            self.arr_left = 0.0;
        } else if rl.is_key_down(KeyboardKey::KEY_LEFT) {
            self.das_left += dt;
            if self.das_left >= DAS {
                self.arr_left += dt;
                if self.arr_left >= ARR {
                    self.try_move(-1);
                    self.arr_left = 0.0;
                }
            }
        } else {
            self.das_left = 0.0;
            self.arr_left = 0.0;
        }

        if rl.is_key_pressed(KeyboardKey::KEY_RIGHT) {
            self.try_move(1);
            self.das_right = 0.0;
            self.arr_right = 0.0;
        } else if rl.is_key_down(KeyboardKey::KEY_RIGHT) {
            self.das_right += dt;
            if self.das_right >= DAS {
                self.arr_right += dt;
                if self.arr_right >= ARR {
                    self.try_move(1);
                    self.arr_right = 0.0;
                }
            }
        } else {
            self.das_right = 0.0;
            self.arr_right = 0.0;
        }
    }

    pub fn update(
        &mut self,
        rl: &RaylibHandle,
        dt: f32,
        particles: &mut ParticleSystem,
        sound: &SoundManager,
    ) {
        self.screen_w = rl.get_screen_width();
        self.screen_h = rl.get_screen_height();

        particles.update(dt);

        let sim_hz = SIM_HZ_TABLE[self.level() as usize];
        let piece_hz = PIECE_HZ_TABLE[self.level() as usize];

        let piece_step = if self.soft_drop {
            1.0 / (piece_hz * 2.0)
        } else {
            1.0 / piece_hz
        };
        self.piece_acc += dt;

        while self.piece_acc >= piece_step {
            self.piece_acc -= piece_step;
            let settled = self.grid_settled();

            if !self.piece.active && settled {
                self.piece_spawn(rl);
            }

            if self.piece.active && settled {
                if self.piece.r + 1 < ROWS as i32
                    && self.cells[(self.piece.r + 1) as usize][self.piece.c as usize].kind
                        == CellType::Empty
                {
                    self.piece.r += 1;
                } else {
                    self.cells[self.piece.r as usize][self.piece.c as usize] = self.piece.cell;
                    self.cells[self.piece.r as usize][self.piece.c as usize].fresh = true;
                    self.piece.active = false;
                    self.can_hold = true;
                    particles.spawn(
                        self.cell_sx(self.piece.c),
                        self.cell_sy(self.piece.r) + CELL / 2.0,
                        tier_color(self.piece.cell.tier),
                        EffectType::Land,
                        rl,
                    );
                    sound.play_land();
                }
            }
        }

        self.sim_acc += dt;
        let mut max_ticks = 1;
        while self.sim_acc >= 1.0 / sim_hz && max_ticks > 0 {
            self.sim_acc -= 1.0 / sim_hz;
            max_ticks -= 1;
            self.merge_once(rl, particles, sound);
            self.fall_once();
        }
    }

    // --- Prywatne pomocniki ---

    fn fall_dir(&self, r: i32, c: i32) -> FallDir {
        if r + 1 >= ROWS as i32 {
            return FallDir::None;
        }
        if self.cells[(r + 1) as usize][c as usize].kind == CellType::Empty {
            return FallDir::Down;
        }

        let can_left = c > 0
            && self.cells[(r + 1) as usize][(c - 1) as usize].kind == CellType::Empty
            && self.cells[r as usize][(c - 1) as usize].kind == CellType::Empty;
        let can_right = c < COLS as i32 - 1
            && self.cells[(r + 1) as usize][(c + 1) as usize].kind == CellType::Empty
            && self.cells[r as usize][(c + 1) as usize].kind == CellType::Empty;

        if can_left && !can_right {
            return FallDir::Left;
        }
        if !can_left && can_right {
            return FallDir::Right;
        }
        if can_left && can_right {
            return FallDir::Left;
        }
        FallDir::None
    }

    fn cell_sx(&self, c: i32) -> f32 {
        (self.screen_w as f32 - GRID_PIXEL_W) / 2.0 + c as f32 * CELL + CELL / 2.0
    }

    fn cell_sy(&self, r: i32) -> f32 {
        (self.screen_h as f32 - GRID_PIXEL_H) / 2.0 + r as f32 * CELL + CELL / 2.0
    }

    fn grid_settled(&self) -> bool {
        for r in 0..ROWS - 1 {
            for c in 0..COLS {
                if self.cells[r][c].kind == CellType::Sand
                    && self.fall_dir(r as i32, c as i32) != FallDir::None
                {
                    return false;
                }
            }
        }
        true
    }

    fn try_move(&mut self, dc: i32) {
        let nc = self.piece.c + dc;
        if nc >= 0
            && nc < COLS as i32
            && self.cells[self.piece.r as usize][nc as usize].kind == CellType::Empty
        {
            self.piece.c = nc;
        }
    }

    fn piece_spawn(&mut self, rl: &RaylibHandle) {
        self.piece.c = COLS as i32 / 2;
        self.piece.r = 0;
        self.piece.cell = Cell::sand(self.bag.next(rl.get_random_value::<i32>(0..i32::MAX)));
        self.piece.active = true;
    }

    fn fall_once(&mut self) {
        for r in (0..ROWS - 1).rev() {
            let left = r % 2 == 0;
            for i in 0..COLS {
                let c = if left { i } else { COLS - 1 - i };
                if self.cells[r][c].kind != CellType::Sand {
                    continue;
                }
                match self.fall_dir(r as i32, c as i32) {
                    FallDir::Down => {
                        self.cells[r + 1][c] = self.cells[r][c];
                        self.cells[r + 1][c].fresh = true;
                        self.cells[r][c] = Cell::EMPTY;
                    }
                    FallDir::Left => {
                        self.cells[r + 1][c - 1] = self.cells[r][c];
                        self.cells[r + 1][c - 1].fresh = true;
                        self.cells[r][c] = Cell::EMPTY;
                    }
                    FallDir::Right => {
                        self.cells[r + 1][c + 1] = self.cells[r][c];
                        self.cells[r + 1][c + 1].fresh = true;
                        self.cells[r][c] = Cell::EMPTY;
                    }
                    FallDir::None => {}
                }
            }
        }
    }

    fn try_merge(
        &mut self,
        r: i32,
        c: i32,
        used: &mut [[bool; COLS]; ROWS],
        rl: &RaylibHandle,
        particles: &mut ParticleSystem,
        sound: &SoundManager,
    ) -> bool {
        let tier = self.cells[r as usize][c as usize].tier;
        let is_fresh = self.cells[r as usize][c as usize].fresh;

        let nr = [
            if is_fresh { r + 1 } else { r - 1 },
            if is_fresh { r - 1 } else { r + 1 },
            r,
            r,
        ];
        let nc = [c, c, c - 1, c + 1];

        for i in 0..4 {
            let rr = nr[i];
            let cc = nc[i];
            if rr < 0 || rr >= ROWS as i32 || cc < 0 || cc >= COLS as i32 {
                continue;
            }
            if used[rr as usize][cc as usize]
                || self.cells[rr as usize][cc as usize].kind != CellType::Sand
            {
                continue;
            }
            if self.cells[rr as usize][cc as usize].tier != tier {
                continue;
            }

            let px = (self.cell_sx(c) + self.cell_sx(cc)) / 2.0;
            let py = (self.cell_sy(r) + self.cell_sy(rr)) / 2.0;

            if tier == 10 {
                self.score += 10000;
                let radius = 3;
                for er in (r - radius)..=(r + radius) {
                    for ec in (c - radius)..=(c + radius) {
                        if er < 0 || er >= ROWS as i32 || ec < 0 || ec >= COLS as i32 {
                            continue;
                        }
                        let dr = er - r;
                        let dc = ec - c;
                        if dr * dr + dc * dc <= radius * radius {
                            self.cells[er as usize][ec as usize] = Cell::EMPTY;
                        }
                    }
                }
                // Wierne oryginałowi: czyta `grid[r][c].tier` *po* wyczyszczeniu
                // komórki, więc kolor zawsze będzie tieru 0 (czerwony).
                particles.spawn(
                    self.cell_sx(c),
                    self.cell_sy(r),
                    tier_color(self.cells[r as usize][c as usize].tier),
                    EffectType::Explode,
                    rl,
                );
                sound.play_explode();
            } else {
                let lower_wins = rr == r + 1 && cc == c;
                let wr = if lower_wins { rr } else { r };
                let wc = if lower_wins { cc } else { c };
                let lr = if lower_wins { r } else { rr };
                let lc = if lower_wins { c } else { cc };

                self.cells[wr as usize][wc as usize].tier += 1;
                self.cells[wr as usize][wc as usize].fresh = true;
                self.score += 1 << tier;
                particles.spawn(
                    px,
                    py,
                    tier_color(self.cells[wr as usize][wc as usize].tier),
                    EffectType::Merge,
                    rl,
                );
                sound.play_merge(tier as usize);
                self.cells[lr as usize][lc as usize] = Cell::EMPTY;
            }

            used[r as usize][c as usize] = true;
            used[rr as usize][cc as usize] = true;
            return true;
        }
        false
    }

    fn merge_once(
        &mut self,
        rl: &RaylibHandle,
        particles: &mut ParticleSystem,
        sound: &SoundManager,
    ) -> bool {
        let mut used = [[false; COLS]; ROWS];
        let mut any = false;

        for r in 0..ROWS {
            for c in 0..COLS {
                if !self.cells[r][c].fresh || used[r][c] || self.cells[r][c].kind != CellType::Sand
                {
                    continue;
                }
                if self.try_merge(r as i32, c as i32, &mut used, rl, particles, sound) {
                    any = true;
                }
            }
        }

        for r in 0..ROWS {
            for c in 0..COLS {
                if self.cells[r][c].fresh || used[r][c] || self.cells[r][c].kind != CellType::Sand {
                    continue;
                }
                if self.try_merge(r as i32, c as i32, &mut used, rl, particles, sound) {
                    any = true;
                }
            }
        }

        for r in 0..ROWS {
            for c in 0..COLS {
                self.cells[r][c].fresh = false;
            }
        }

        any
    }

    fn level(&self) -> i32 {
        let level = self.score / 10000;
        if level >= MAX_LEVEL {
            MAX_LEVEL - 1
        } else {
            level
        }
    }
}
