use raylib::prelude::*;

const MERGE_SOUND_COUNT: usize = 10;

pub struct SoundManager<'a> {
    land: Sound<'a>,
    merge: [Sound<'a>; MERGE_SOUND_COUNT],
    explode: Sound<'a>,
    music: Music<'a>,
}

impl<'a> SoundManager<'a> {
    /// Odpowiednik `sound_init()`.
    pub fn new(audio: &'a RaylibAudio) -> Self {
        let land = audio
            .new_sound("assets/sfx/land.wav")
            .expect("failed to load land.wav");

        let merge = std::array::from_fn(|i| {
            audio
                .new_sound(&format!("assets/sfx/merge{}.wav", i + 1))
                .expect("failed to load merge sound")
        });

        let explode = audio
            .new_sound("assets/sfx/merge_explode.wav")
            .expect("failed to load merge_explode.wav");

        let mut music = audio
            .new_music("assets/sfx/music0.wav")
            .expect("failed to load music0.wav");

        music.looping = true;
        music.set_volume(0.2);
        music.play_stream();

        Self {
            land,
            merge,
            explode,
            music,
        }
    }

    pub fn play_land(&self) {
        self.land.play();
    }

    pub fn play_merge(&self, tier: usize) {
        if tier < MERGE_SOUND_COUNT {
            self.merge[tier].play();
        }
    }

    pub fn play_explode(&self) {
        self.explode.play();
    }

    pub fn update_music(&mut self) {
        self.music.update_stream();
    }
}
