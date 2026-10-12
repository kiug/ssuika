use std::f32::consts::PI;

use raylib::prelude::*;

use crate::grid::{CELL, COLS, CellType, GRID_PIXEL_H, GRID_PIXEL_W, Grid, ROWS, tier_color};
use crate::particle::ParticleSystem;

pub fn draw(grid: &Grid, d: &mut RaylibDrawHandle, particles: &ParticleSystem) {
    let cell = CELL as i32;
    let ox = (grid.screen_w - GRID_PIXEL_W as i32) / 2;
    let oy = (grid.screen_h - GRID_PIXEL_H as i32) / 2;

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
            if grid.cells[r][c].kind == CellType::Sand {
                let tier = grid.cells[r][c].tier;
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

    if grid.piece.active {
        draw_tier_shape(
            d,
            ox as f32 + grid.piece.c as f32 * CELL + CELL / 2.0,
            oy as f32 + grid.piece.r as f32 * CELL + CELL / 2.0,
            grid.piece.cell.tier,
            tier_color(grid.piece.cell.tier),
        );

        let mut ghost_r = grid.piece.r;
        while ghost_r + 1 < ROWS as i32
            && grid.cells[(ghost_r + 1) as usize][grid.piece.c as usize].kind == CellType::Empty
        {
            ghost_r += 1;
        }

        if ghost_r != grid.piece.r {
            let mut ghost_color = tier_color(grid.piece.cell.tier);
            ghost_color.a = 80;
            draw_tier_shape(
                d,
                ox as f32 + grid.piece.c as f32 * CELL + CELL / 2.0,
                oy as f32 + ghost_r as f32 * CELL + CELL / 2.0,
                grid.piece.cell.tier,
                ghost_color,
            );
        }

        for lr in grid.piece.r..=ghost_r {
            let mut line_color = tier_color(grid.piece.cell.tier);
            line_color.a = 25;
            d.draw_rectangle(
                ox + grid.piece.c * cell,
                oy + lr * cell,
                cell,
                cell,
                line_color,
            );
        }
    }

    let qx = ox + GRID_PIXEL_W as i32 + cell;
    let qy = oy;

    let q_size = grid.font.measure_text("next", CELL, 0.0);
    d.draw_text_ex(
        &grid.font,
        "next",
        Vector2::new(qx as f32 + CELL / 2.0 - q_size.x / 2.0, oy as f32 - CELL),
        CELL,
        0.0,
        Color::WHITE,
    );
    for i in 0..grid.bag.preview_len() {
        let tier = grid.bag.preview(i);
        draw_tier_shape(
            d,
            qx as f32 + CELL / 2.0,
            qy as f32 + i as f32 * CELL + CELL / 2.0,
            tier,
            tier_color(tier),
        );
    }

    let h_size = grid.font.measure_text("hold", CELL, 0.0);
    d.draw_text_ex(
        &grid.font,
        "hold",
        Vector2::new(
            ox as f32 - CELL - CELL + CELL / 2.0 - h_size.x / 2.0,
            oy as f32 - CELL,
        ),
        CELL,
        0.0,
        Color::WHITE,
    );
    if grid.hold_tier != -1 {
        let c = if grid.can_hold {
            tier_color(grid.hold_tier)
        } else {
            Color::new(100, 100, 100, 255)
        };
        draw_tier_shape(
            d,
            ox as f32 - CELL - CELL / 2.0,
            oy as f32 + CELL / 2.0,
            grid.hold_tier,
            c,
        );
    }

    let score_buf = format!("{}", grid.get_score());
    let score_size = grid.font.measure_text(&score_buf, CELL, 0.0);
    d.draw_text_ex(
        &grid.font,
        &score_buf,
        Vector2::new(
            ox as f32 + GRID_PIXEL_W / 2.0 - score_size.x / 2.0,
            oy as f32 - CELL,
        ),
        CELL,
        0.0,
        Color::WHITE,
    );

    let progress = (grid.get_score() % 10000) as f32 / 10000.0;
    d.draw_rectangle(
        ox,
        oy + GRID_PIXEL_H as i32 + 4,
        (GRID_PIXEL_W * progress) as i32,
        3,
        Color::WHITE,
    );

    particles.draw(d);
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
