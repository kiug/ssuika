use std::f32::consts::PI;

use raylib::prelude::*;

const MAX_PARTICLES: usize = 512;

#[derive(Clone, Copy)]
pub enum EffectType {
    Land,
    Merge,
    Explode,
}

#[derive(Clone, Copy)]
struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: f32,
    size: f32,
    color: Color,
}

impl Particle {
    const DEAD: Self = Particle {
        x: 0.0,
        y: 0.0,
        vx: 0.0,
        vy: 0.0,
        life: 0.0,
        size: 0.0,
        color: Color::BLACK,
    };
}

pub struct ParticleSystem {
    pool: [Particle; MAX_PARTICLES],
}

impl ParticleSystem {
    pub fn new() -> Self {
        ParticleSystem {
            pool: [Particle::DEAD; MAX_PARTICLES],
        }
    }

    fn spawn_particle(
        &mut self,
        x: f32,
        y: f32,
        vx: f32,
        vy: f32,
        life: f32,
        size: f32,
        color: Color,
    ) {
        for p in &mut self.pool {
            if p.life <= 0.0 {
                *p = Particle {
                    x,
                    y,
                    vx,
                    vy,
                    life,
                    size,
                    color,
                };
                return;
            }
        }
    }

    pub fn spawn(&mut self, x: f32, y: f32, color: Color, effect: EffectType, rl: &RaylibHandle) {
        match effect {
            EffectType::Land => {
                let count = 10;
                for i in 0..count {
                    let angle = (i as f32 / count as f32) * 2.0 * PI;
                    let speed = rl.get_random_value::<i32>(20..51) as f32;
                    let vx = angle.cos() * speed;
                    let vy = angle.sin() * speed * 0.5 - 30.0;
                    let mut c = Color::GRAY;
                    c.a = 200;
                    self.spawn_particle(x, y, vx, vy, 0.4, 2.0, c);
                }
            }
            EffectType::Merge => {
                let count = 16;
                for i in 0..count {
                    let angle = (i as f32 / count as f32) * 2.0 * PI;
                    let speed = rl.get_random_value::<i32>(40..101) as f32;
                    let vx = angle.cos() * speed;
                    let vy = angle.sin() * speed;
                    let mut c = color;
                    c.a = 255;
                    self.spawn_particle(x, y, vx, vy, 0.6, 3.0, c);
                }
            }
            EffectType::Explode => {
                let count = 48;
                for i in 0..count {
                    let angle = (i as f32 / count as f32) * 2.0 * PI;
                    let speed = rl.get_random_value::<i32>(200..301) as f32;
                    let vx = angle.cos() * speed;
                    let vy = angle.sin() * speed;
                    let mut c = Color::color_from_hsv(
                        rl.get_random_value::<i32>(255..361) as f32,
                        1.0,
                        1.0,
                    );
                    c.a = 255;
                    self.spawn_particle(x, y, vx, vy, 1.0, 4.0, c);
                }
            }
        }
    }

    pub fn update(&mut self, dt: f32) {
        let gravity = 200.0;
        for p in &mut self.pool {
            if p.life <= 0.0 {
                continue;
            }
            p.life -= dt * 2.0;
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.vy += gravity * dt;
        }
    }

    pub fn draw(&self, d: &mut RaylibDrawHandle) {
        for p in &self.pool {
            if p.life <= 0.0 {
                continue;
            }
            let mut c = p.color;
            c.a = (p.life * 255.0) as u8;
            d.draw_circle_v(Vector2::new(p.x, p.y), p.size, c);
        }
    }
}
