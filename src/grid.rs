use std::f32::consts::PI;

use raylib::prelude::*;

use crate::particle::{EffectType, ParticleSystem};
use crate::sound::SoundManager;

// --- Wymiary planszy (z grid.h) ---
pub const COLS: usize = 7;
pub const ROWS: usize = 14;
pub const CELL: f32 = 32.0;
pub const GRID_PIXEL_W: f32 = COLS as f32 * CELL;
pub const GRID_PIXEL_H: f32 = ROWS as f32 * CELL;

const BAG_SIZE: usize = 7;
const POWER: f32 = 1.0;
const NEXT_COUNT: usize = BAG_SIZE;

const DAS: f32 = 0.100;
const ARR: f32 = 0.001;

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
    cells: [[Cell; COLS]; ROWS],
    piece: Piece,
    soft_drop: bool,

    recency: [f32; BAG_SIZE],
    next_queue: [i32; NEXT_COUNT],

    das_left: f32,
    arr_left: f32,
    das_right: f32,
    arr_right: f32,

    hold_tier: i32,
    can_hold: bool,

    score: i32,

    font: Font,

    sim_acc: f32,
    piece_acc: f32,

    screen_w: i32,
    screen_h: i32,
}

fn tier_color(tier: i32) -> Color {
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
            recency: [1.0; BAG_SIZE],
            next_queue: [0; NEXT_COUNT],
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

        for i in 0..NEXT_COUNT {
            let next = grid.gen_next(rl);
            grid.next_queue[i] = next;
        }
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

    pub fn draw(&self, d: &mut RaylibDrawHandle, particles: &ParticleSystem) {
        let cell = CELL as i32;
        let ox = (self.screen_w - GRID_PIXEL_W as i32) / 2;
        let oy = (self.screen_h - GRID_PIXEL_H as i32) / 2;

        d.clear_background(Color::BLACK);

        let grid_color = Color::new(255, 255, 255, 20);
        for r in 0..=ROWS {
            let y = oy + r as i32 * cell;
            d.draw_line(ox, y, ox + GRID_PIXEL_W as i32, y, grid_color);
        }
        for c in 0..=COLS {
            let x = ox + c as i32 * cell;
            d.draw_line(x, oy, x, oy + GRID_PIXEL_H as i32, grid_color);
        }

        for r in 0..ROWS {
            for c in 0..COLS {
                if self.cells[r][c].kind == CellType::Sand {
                    let tier = self.cells[r][c].tier;
                    draw_tier_shape(
                        d,
                        ox as f32 + c as f32 * CELL + CELL / 2.0,
                        oy as f32 + r as f32 * CELL + CELL / 2.0,
                        tier,
                        tier_color(tier),
                    );
                }
            }
        }

        if self.piece.active {
            draw_tier_shape(
                d,
                ox as f32 + self.piece.c as f32 * CELL + CELL / 2.0,
                oy as f32 + self.piece.r as f32 * CELL + CELL / 2.0,
                self.piece.cell.tier,
                tier_color(self.piece.cell.tier),
            );

            let mut ghost_r = self.piece.r;
            while ghost_r + 1 < ROWS as i32
                && self.cells[(ghost_r + 1) as usize][self.piece.c as usize].kind == CellType::Empty
            {
                ghost_r += 1;
            }

            if ghost_r != self.piece.r {
                let mut ghost_color = tier_color(self.piece.cell.tier);
                ghost_color.a = 80;
                draw_tier_shape(
                    d,
                    ox as f32 + self.piece.c as f32 * CELL + CELL / 2.0,
                    oy as f32 + ghost_r as f32 * CELL + CELL / 2.0,
                    self.piece.cell.tier,
                    ghost_color,
                );
            }

            for lr in self.piece.r..=ghost_r {
                let mut line_color = tier_color(self.piece.cell.tier);
                line_color.a = 25;
                d.draw_rectangle(
                    ox + self.piece.c * cell,
                    oy + lr * cell,
                    cell,
                    cell,
                    line_color,
                );
            }
        }

        let qx = ox + GRID_PIXEL_W as i32 + cell;
        let qy = oy;

        let q_size = self.font.measure_text("next", CELL, 0.0);
        d.draw_text_ex(
            &self.font,
            "next",
            Vector2::new(qx as f32 + CELL / 2.0 - q_size.x / 2.0, oy as f32 - CELL),
            CELL,
            0.0,
            Color::WHITE,
        );
        for i in 0..BAG_SIZE {
            let tier = self.next_queue[i];
            draw_tier_shape(
                d,
                qx as f32 + CELL / 2.0,
                qy as f32 + i as f32 * CELL + CELL / 2.0,
                tier,
                tier_color(tier),
            );
        }

        let h_size = self.font.measure_text("hold", CELL, 0.0);
        d.draw_text_ex(
            &self.font,
            "hold",
            Vector2::new(
                ox as f32 - CELL - CELL + CELL / 2.0 - h_size.x / 2.0,
                oy as f32 - CELL,
            ),
            CELL,
            0.0,
            Color::WHITE,
        );
        if self.hold_tier != -1 {
            let c = if self.can_hold {
                tier_color(self.hold_tier)
            } else {
                Color::new(100, 100, 100, 255)
            };
            draw_tier_shape(
                d,
                ox as f32 - CELL - CELL / 2.0,
                oy as f32 + CELL / 2.0,
                self.hold_tier,
                c,
            );
        }

        let score_buf = format!("{}", self.get_score());
        let score_size = self.font.measure_text(&score_buf, CELL, 0.0);
        d.draw_text_ex(
            &self.font,
            &score_buf,
            Vector2::new(
                ox as f32 + GRID_PIXEL_W / 2.0 - score_size.x / 2.0,
                oy as f32 - CELL,
            ),
            CELL,
            0.0,
            Color::WHITE,
        );

        let progress = (self.score % 10000) as f32 / 10000.0;
        d.draw_rectangle(
            ox,
            oy + GRID_PIXEL_H as i32 + 4,
            (GRID_PIXEL_W * progress) as i32,
            3,
            Color::WHITE,
        );

        particles.draw(d);
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

    fn gen_next(&mut self, rl: &RaylibHandle) -> i32 {
        let mut weights = [0.0f32; BAG_SIZE];
        let mut total = 0.0f32;
        for i in 0..BAG_SIZE {
            weights[i] = (1.0 / self.recency[i]).powf(POWER);
            total += weights[i];
        }

        let roll = rl.get_random_value::<i32>(0..10001) as f32 / 10000.0 * total;
        let mut pick = BAG_SIZE - 1;
        let mut acc = 0.0f32;
        for i in 0..BAG_SIZE {
            acc += weights[i];
            if roll < acc {
                pick = i;
                break;
            }
        }

        self.recency[pick] += 1.0;
        for i in 0..BAG_SIZE {
            if i != pick {
                self.recency[i] /= 2.0;
            }
        }

        pick as i32
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

    fn next_from_bag(&mut self, rl: &RaylibHandle) -> i32 {
        let pick = self.next_queue[0];
        for i in 0..NEXT_COUNT - 1 {
            self.next_queue[i] = self.next_queue[i + 1];
        }
        self.next_queue[NEXT_COUNT - 1] = self.gen_next(rl);
        pick
    }

    fn piece_spawn(&mut self, rl: &RaylibHandle) {
        self.piece.c = COLS as i32 / 2;
        self.piece.r = 0;
        self.piece.cell = Cell::sand(self.next_from_bag(rl));
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

fn draw_star(
    d: &mut RaylibDrawHandle,
    cx: f32,
    cy: f32,
    points: usize,
    outer_r: f32,
    inner_r: f32,
    color: Color,
) {
    let angle_step = PI / points as f32;
    let mut verts = Vec::with_capacity(points * 2);
    for i in 0..points * 2 {
        let angle = i as f32 * angle_step - PI / 2.0;
        let r = if i % 2 == 0 { outer_r } else { inner_r };
        verts.push(Vector2::new(cx + angle.cos() * r, cy + angle.sin() * r));
    }
    for i in 0..points * 2 {
        let a = verts[i];
        let b = verts[(i + 1) % (points * 2)];
        d.draw_line_v(a, b, color);
    }
}

fn draw_tier_shape(d: &mut RaylibDrawHandle, cx: f32, cy: f32, tier: i32, color: Color) {
    let r = CELL * 0.4;
    const SIDES: [i32; 11] = [0, 3, 4, 5, 6, 0, 0, 0, 0, 0, 0];
    const POINTS: [i32; 11] = [0, 0, 0, 0, 0, 3, 4, 5, 6, 7, 8];
    const ANGLES: [f32; 11] = [0.0, 0.0, 45.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];

    if tier == 0 {
        d.draw_circle_lines(cx as i32, cy as i32, r, color);
    } else if tier >= 5 {
        draw_star(
            d,
            cx,
            cy,
            POINTS[tier as usize] as usize,
            r,
            r * 0.30,
            color,
        );
    } else {
        d.draw_poly_lines(
            Vector2::new(cx, cy),
            SIDES[tier as usize],
            r,
            ANGLES[tier as usize],
            color,
        );
    }
}
