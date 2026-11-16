//! Game rules. Knows nothing about the terminal, so it can be unit tested.

pub const FIELD_W: usize = 40;
pub const FIELD_H: usize = 20;
pub const BASKET_W: usize = 5;
pub const BASKET_STEP: usize = 2;
pub const TICKS_PER_SECOND: u64 = 20;

const FLAKES_PER_ROW: u32 = 4;
const START_LIVES: u32 = 3;
const EMBER_MELTS: u32 = 2;
const FLASH_TICKS: u8 = 6;

/// The finished snowman. It is built from the bottom row upwards.
pub const SNOWMAN: [&str; 10] = [
    "      ___",
    "     |   |",
    "    _|___|_",
    "     (o o)",
    "     ( > )",
    "  --(  :  )--",
    "   (   :   )",
    "  (    :    )",
    "  (         )",
    "   '-------'",
];

pub const GOAL: u32 = SNOWMAN.len() as u32 * FLAKES_PER_ROW;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Flake,
    Ember,
}

#[derive(Clone, Debug)]
pub struct Item {
    pub x: usize,
    pub y: f32,
    pub speed: f32,
    pub kind: Kind,
    pub glyph: char,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Title,
    Playing,
    Paused,
    Won,
    Lost,
}

pub struct Game {
    pub phase: Phase,
    pub basket_x: usize,
    pub items: Vec<Item>,
    pub caught: u32,
    pub lives: u32,
    pub ticks: u64,
    /// Counts down after an ember hit; the renderer flashes while it is > 0.
    pub flash: u8,
    rng: Rng,
}

impl Game {
    pub fn new(seed: u64) -> Self {
        Self {
            phase: Phase::Title,
            basket_x: (FIELD_W - BASKET_W) / 2,
            items: Vec::new(),
            caught: 0,
            lives: START_LIVES,
            ticks: 0,
            flash: 0,
            rng: Rng::new(seed),
        }
    }

    /// Starts a fresh round, keeping the random sequence going.
    pub fn start(&mut self) {
        let seed = self.rng.next_u64();
        *self = Self::new(seed);
        self.phase = Phase::Playing;
    }

    pub fn toggle_pause(&mut self) {
        self.phase = match self.phase {
            Phase::Playing => Phase::Paused,
            Phase::Paused => Phase::Playing,
            other => other,
        };
    }

    pub fn move_left(&mut self) {
        if self.phase == Phase::Playing {
            self.basket_x = self.basket_x.saturating_sub(BASKET_STEP);
        }
    }

    pub fn move_right(&mut self) {
        if self.phase == Phase::Playing {
            self.basket_x = (self.basket_x + BASKET_STEP).min(FIELD_W - BASKET_W);
        }
    }

    /// Number of snowman rows that are finished (counted from the bottom).
    pub fn rows_built(&self) -> usize {
        (self.caught / FLAKES_PER_ROW) as usize
    }

    /// Flakes already collected for the row currently being built.
    pub fn row_progress(&self) -> (u32, u32) {
        (self.caught % FLAKES_PER_ROW, FLAKES_PER_ROW)
    }

    pub fn seconds(&self) -> u64 {
        self.ticks / TICKS_PER_SECOND
    }

    pub fn tick(&mut self) {
        if self.phase != Phase::Playing {
            return;
        }
        self.ticks += 1;
        self.flash = self.flash.saturating_sub(1);
        self.spawn();
        self.fall();
    }

    fn spawn(&mut self) {
        let level = self.rows_built() as f32;
        if self.rng.next_f32() >= 0.10 + 0.008 * level {
            return;
        }
        let ember_chance = (0.14 + 0.02 * level).min(0.35);
        let kind = if self.rng.next_f32() < ember_chance {
            Kind::Ember
        } else {
            Kind::Flake
        };
        let glyph = match kind {
            Kind::Ember => 'o',
            Kind::Flake => ['*', '*', '+', '.'][self.rng.below(4)],
        };
        self.items.push(Item {
            x: self.rng.below(FIELD_W),
            y: 0.0,
            speed: 0.14 + 0.14 * self.rng.next_f32() + 0.012 * level,
            kind,
            glyph,
        });
    }

    fn fall(&mut self) {
        let basket_row = (FIELD_H - 1) as f32;
        let basket = self.basket_x..self.basket_x + BASKET_W;
        let mut landed = Vec::new();
        self.items.retain_mut(|item| {
            item.y += item.speed;
            if item.y < basket_row {
                return true;
            }
            if basket.contains(&item.x) {
                landed.push(item.kind);
            }
            false
        });
        for kind in landed {
            self.catch(kind);
        }
    }

    fn catch(&mut self, kind: Kind) {
        if self.phase != Phase::Playing {
            return;
        }
        match kind {
            Kind::Flake => {
                self.caught += 1;
                if self.caught >= GOAL {
                    self.phase = Phase::Won;
                }
            }
            Kind::Ember => {
                self.caught = self.caught.saturating_sub(EMBER_MELTS);
                self.lives = self.lives.saturating_sub(1);
                self.flash = FLASH_TICKS;
                if self.lives == 0 {
                    self.phase = Phase::Lost;
                }
            }
        }
    }
}

/// Small xorshift generator, enough for falling snow.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// Uniform in `0.0..1.0`.
    fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn playing() -> Game {
        let mut game = Game::new(42);
        game.start();
        game
    }

    /// An item that reaches the basket row on the next tick.
    fn landing(x: usize, kind: Kind) -> Item {
        Item {
            x,
            y: (FIELD_H - 1) as f32 - 0.1,
            speed: 0.2,
            kind,
            glyph: '*',
        }
    }

    #[test]
    fn catching_a_flake_adds_snow() {
        let mut game = playing();
        game.items.push(landing(game.basket_x, Kind::Flake));
        game.tick();
        assert_eq!(game.caught, 1);
    }

    #[test]
    fn both_basket_edges_catch() {
        let mut game = playing();
        game.items.push(landing(game.basket_x, Kind::Flake));
        game.items
            .push(landing(game.basket_x + BASKET_W - 1, Kind::Flake));
        game.items
            .push(landing(game.basket_x + BASKET_W, Kind::Flake));
        game.tick();
        assert_eq!(game.caught, 2);
    }

    #[test]
    fn missed_flake_disappears() {
        let mut game = playing();
        game.items.push(landing(0, Kind::Flake));
        game.tick();
        assert_eq!(game.caught, 0);
        assert!(game.items.iter().all(|i| i.y < 1.0));
    }

    #[test]
    fn ember_melts_snow_and_costs_a_life() {
        let mut game = playing();
        game.caught = 5;
        game.items.push(landing(game.basket_x, Kind::Ember));
        game.tick();
        assert_eq!(game.caught, 3);
        assert_eq!(game.lives, 2);
        assert_eq!(game.phase, Phase::Playing);
    }

    #[test]
    fn last_life_ends_the_game() {
        let mut game = playing();
        game.lives = 1;
        game.items.push(landing(game.basket_x, Kind::Ember));
        game.tick();
        assert_eq!(game.phase, Phase::Lost);
    }

    #[test]
    fn last_flake_wins() {
        let mut game = playing();
        game.caught = GOAL - 1;
        game.items.push(landing(game.basket_x, Kind::Flake));
        game.tick();
        assert_eq!(game.phase, Phase::Won);
        assert_eq!(game.rows_built(), SNOWMAN.len());
    }

    #[test]
    fn basket_stays_inside_the_field() {
        let mut game = playing();
        for _ in 0..FIELD_W {
            game.move_left();
        }
        assert_eq!(game.basket_x, 0);
        for _ in 0..FIELD_W {
            game.move_right();
        }
        assert_eq!(game.basket_x, FIELD_W - BASKET_W);
    }

    #[test]
    fn nothing_moves_while_paused() {
        let mut game = playing();
        game.toggle_pause();
        game.items.push(landing(game.basket_x, Kind::Flake));
        game.tick();
        game.move_left();
        assert_eq!(game.caught, 0);
        assert_eq!(game.basket_x, (FIELD_W - BASKET_W) / 2);
    }

    #[test]
    fn spawned_items_stay_inside_the_field() {
        let mut game = playing();
        for _ in 0..2000 {
            game.tick();
            if game.phase != Phase::Playing {
                game.start();
            }
            assert!(game.items.iter().all(|i| i.x < FIELD_W));
        }
    }
}
