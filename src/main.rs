mod grid;
mod particle;
mod sound;

use grid::Grid;
use particle::ParticleSystem;
use sound::SoundManager;

fn main() {
    let (mut rl, thread) = raylib::init().size(1280, 720).title("ssuika").build();
    let mut grid = Grid::new(&mut rl, &thread);
    let mut particles = ParticleSystem::new();
    let mut sound = SoundManager::new();

    while !rl.window_should_close() {
        let dt = rl.get_frame_time();
        grid.input(&rl, dt, &mut particles, &sound);
        grid.update(&rl, dt, &mut particles, &sound);
        let mut d = rl.begin_drawing(&thread);
        grid.draw(&mut d, &particles);
        sound.update_music();
    }
}
