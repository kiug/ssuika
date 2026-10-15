const BAG_SIZE: usize = 7;
const POWER: f32 = 1.0;
const NEXT_COUNT: usize = BAG_SIZE;

pub struct Bag {
    recency: [f32; BAG_SIZE],
    next_queue: [i32; NEXT_COUNT],
}

impl Bag {
    /// Inicjalizuje worek pierwszymi losowaniami (po jednym na element kolejki).
    pub fn new(rolls: [i32; NEXT_COUNT]) -> Self {
        let mut bag = Bag {
            recency: [1.0; BAG_SIZE],
            next_queue: [0; NEXT_COUNT],
        };
        for i in 0..NEXT_COUNT {
            bag.next_queue[i] = bag.gen_next(rolls[i]);
        }
        bag
    }

    /// Pobiera kolejny klocek i dopełnia kolejkę.
    pub fn next(&mut self, roll: i32) -> i32 {
        let pick = self.next_queue[0];
        for i in 0..NEXT_COUNT - 1 {
            self.next_queue[i] = self.next_queue[i + 1];
        }
        self.next_queue[NEXT_COUNT - 1] = self.gen_next(roll);
        pick
    }

    /// Podgląd kolejki do renderowania panelu „next”.
    pub fn preview(&self, index: usize) -> i32 {
        self.next_queue[index]
    }

    /// Liczba elementów widocznych w podglądzie.
    pub fn preview_len(&self) -> usize {
        NEXT_COUNT
    }

    fn gen_next(&mut self, roll: i32) -> i32 {
        let mut weights = [0.0f32; BAG_SIZE];
        let mut total = 0.0f32;
        for i in 0..BAG_SIZE {
            weights[i] = (1.0 / self.recency[i]).powf(POWER);
            total += weights[i];
        }

        let roll = roll as f32 / i32::MAX as f32 * total;
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
}
