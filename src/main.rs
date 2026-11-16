//! Snowman Catch: catch falling snowflakes to build a snowman.

mod game;

use std::io::{self, Write};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::style::{Color, Print, ResetColor, SetForegroundColor};
use crossterm::terminal::{self, Clear, ClearType};
use crossterm::{cursor, execute, queue};

use game::{BASKET_W, FIELD_H, FIELD_W, GOAL, Game, Kind, Phase, SNOWMAN, TICKS_PER_SECOND};

const PANEL_W: usize = 22;
const SCREEN_W: usize = FIELD_W + 2 + 2 + PANEL_W;
const SCREEN_H: usize = FIELD_H + 4;

/// Top-left corner of the play field's border on the canvas.
const FIELD_X: usize = 0;
const FIELD_Y: usize = 1;
const PANEL_X: usize = FIELD_W + 4;

const ORANGE: Color = Color::Rgb {
    r: 255,
    g: 140,
    b: 0,
};

fn main() -> io::Result<()> {
    let _terminal = TerminalGuard::enter()?;
    run(&mut io::stdout())
}

/// Puts the terminal into game mode and restores it on drop, also on errors.
struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(io::stdout(), terminal::EnterAlternateScreen, cursor::Hide)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), cursor::Show, terminal::LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

fn run(out: &mut impl Write) -> io::Result<()> {
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(1, |d| d.as_nanos() as u64);
    let mut game = Game::new(seed);
    let tick = Duration::from_millis(1000 / TICKS_PER_SECOND);
    let mut last_tick = Instant::now();
    let mut needs_clear = true;

    loop {
        if event::poll(tick.saturating_sub(last_tick.elapsed()))? {
            match event::read()? {
                Event::Key(key) if key.kind != KeyEventKind::Release => {
                    let ctrl_c = key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL);
                    if ctrl_c || !handle_key(&mut game, key.code) {
                        return Ok(());
                    }
                }
                Event::Resize(..) => needs_clear = true,
                _ => {}
            }
        }
        if last_tick.elapsed() >= tick {
            last_tick = Instant::now();
            game.tick();
            if needs_clear {
                queue!(out, Clear(ClearType::All))?;
                needs_clear = false;
            }
            draw(out, &game)?;
        }
    }
}

/// Returns `false` when the player wants to quit.
fn handle_key(game: &mut Game, code: KeyCode) -> bool {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => return false,
        KeyCode::Left | KeyCode::Char('a' | 'h') => game.move_left(),
        KeyCode::Right | KeyCode::Char('d' | 'l') => game.move_right(),
        KeyCode::Char('p') => game.toggle_pause(),
        KeyCode::Enter | KeyCode::Char(' ') => match game.phase {
            Phase::Title | Phase::Won | Phase::Lost => game.start(),
            Phase::Paused => game.toggle_pause(),
            Phase::Playing => {}
        },
        KeyCode::Char('r') => game.start(),
        _ => {}
    }
    true
}

fn draw(out: &mut impl Write, game: &Game) -> io::Result<()> {
    let (cols, rows) = terminal::size()?;
    let (cols, rows) = (cols as usize, rows as usize);
    if cols < SCREEN_W || rows < SCREEN_H {
        queue!(
            out,
            Clear(ClearType::All),
            cursor::MoveTo(0, 0),
            Print(format!(
                "Terminal too small: need {SCREEN_W}x{SCREEN_H}, have {cols}x{rows}"
            ))
        )?;
        return out.flush();
    }

    let mut canvas = Canvas::new();
    draw_header(&mut canvas, game);
    draw_field(&mut canvas, game);
    draw_panel(&mut canvas, game);
    draw_overlay(&mut canvas, game);
    canvas.text(
        1,
        SCREEN_H - 1,
        "<-/-> or a/d move   p pause   r restart   q quit",
        Color::DarkGrey,
    );
    canvas.render(out, (cols - SCREEN_W) / 2, (rows - SCREEN_H) / 2)
}

fn draw_header(canvas: &mut Canvas, game: &Game) {
    canvas.text(1, 0, "* SNOWMAN CATCH *", Color::Cyan);
    canvas.text(
        21,
        0,
        &format!("Snow {:>2}/{GOAL}", game.caught),
        Color::White,
    );
    canvas.text(34, 0, "Lives", Color::White);
    for life in 0..game.lives as usize {
        canvas.put(40 + life * 2, 0, '♥', Color::Red);
    }
    let seconds = game.seconds();
    canvas.text(
        PANEL_X + 4,
        0,
        &format!("Time {}:{:02}", seconds / 60, seconds % 60),
        Color::White,
    );
}

fn draw_field(canvas: &mut Canvas, game: &Game) {
    let border = if game.flash > 0 {
        Color::Red
    } else {
        Color::DarkGreen
    };
    canvas.frame(FIELD_X, FIELD_Y, FIELD_W + 2, FIELD_H + 2, border);

    let (left, top) = (FIELD_X + 1, FIELD_Y + 1);
    for item in &game.items {
        let color = match (item.kind, item.glyph) {
            (Kind::Ember, _) => Color::Red,
            (Kind::Flake, '.') => Color::Grey,
            (Kind::Flake, '+') => Color::Cyan,
            (Kind::Flake, _) => Color::White,
        };
        canvas.put(left + item.x, top + item.y as usize, item.glyph, color);
    }

    let mut basket = String::from("\\");
    basket.push_str(&"_".repeat(BASKET_W - 2));
    basket.push('/');
    canvas.text(left + game.basket_x, top + FIELD_H - 1, &basket, Color::Yellow);
}

fn draw_panel(canvas: &mut Canvas, game: &Game) {
    canvas.text(PANEL_X + 4, 2, "Your snowman", Color::Cyan);

    let built_from = SNOWMAN.len() - game.rows_built().min(SNOWMAN.len());
    for (row, line) in SNOWMAN.iter().enumerate() {
        for (col, ch) in line.chars().enumerate() {
            let color = if row < built_from {
                Color::DarkGrey
            } else {
                match ch {
                    '>' => ORANGE,
                    '-' if row == 5 => Color::DarkYellow,
                    '_' | '|' if row < 3 => Color::Blue,
                    _ => Color::White,
                }
            };
            canvas.put(PANEL_X + 3 + col, 5 + row, ch, color);
        }
    }

    let (done, per_row) = game.row_progress();
    let bar: String = (0..per_row)
        .map(|i| if i < done { '*' } else { '.' })
        .collect();
    canvas.text(PANEL_X + 3, 17, &format!("Next row [{bar}]"), Color::White);

    canvas.text(PANEL_X + 3, 19, "*", Color::White);
    canvas.text(PANEL_X + 5, 19, "+1 snow", Color::Grey);
    canvas.text(PANEL_X + 3, 20, "o", Color::Red);
    canvas.text(PANEL_X + 5, 20, "ember: melts 2,", Color::Grey);
    canvas.text(PANEL_X + 5, 21, "costs a life", Color::Grey);
}

fn draw_overlay(canvas: &mut Canvas, game: &Game) {
    let seconds = game.seconds();
    let time = format!("Built in {}:{:02}", seconds / 60, seconds % 60);
    let (lines, color): (Vec<&str>, Color) = match game.phase {
        Phase::Playing => return,
        Phase::Title => (
            vec![
                "Catch the snowflakes",
                "to build your snowman.",
                "Avoid the red embers!",
                "",
                "Press Enter to start",
            ],
            Color::Cyan,
        ),
        Phase::Paused => (vec!["Paused", "", "Press p to continue"], Color::Yellow),
        Phase::Won => (
            vec![
                "Merry Christmas!",
                "Your snowman is finished.",
                &time,
                "",
                "Press Enter to play again",
            ],
            Color::Green,
        ),
        Phase::Lost => (
            vec![
                "Your snowman melted...",
                "",
                "Press Enter to try again",
            ],
            Color::Red,
        ),
    };

    let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0) + 4;
    let height = lines.len() + 2;
    let x = FIELD_X + 1 + (FIELD_W - width) / 2;
    let y = FIELD_Y + 1 + (FIELD_H - height) / 2;
    for row in 0..height {
        canvas.text(x, y + row, &" ".repeat(width), color);
    }
    canvas.frame(x, y, width, height, color);
    for (i, line) in lines.iter().enumerate() {
        let pad = (width - line.chars().count()) / 2;
        canvas.text(x + pad, y + 1 + i, line, Color::White);
    }
}

/// One frame, drawn cell by cell and then written out in a single pass,
/// so the terminal never shows a half-drawn screen.
struct Canvas {
    cells: Vec<(char, Color)>,
}

impl Canvas {
    fn new() -> Self {
        Self {
            cells: vec![(' ', Color::Reset); SCREEN_W * SCREEN_H],
        }
    }

    fn put(&mut self, x: usize, y: usize, ch: char, color: Color) {
        if x < SCREEN_W && y < SCREEN_H {
            self.cells[y * SCREEN_W + x] = (ch, color);
        }
    }

    fn text(&mut self, x: usize, y: usize, text: &str, color: Color) {
        for (i, ch) in text.chars().enumerate() {
            self.put(x + i, y, ch, color);
        }
    }

    fn frame(&mut self, x: usize, y: usize, width: usize, height: usize, color: Color) {
        let (right, bottom) = (x + width - 1, y + height - 1);
        for col in x + 1..right {
            self.put(col, y, '─', color);
            self.put(col, bottom, '─', color);
        }
        for row in y + 1..bottom {
            self.put(x, row, '│', color);
            self.put(right, row, '│', color);
        }
        self.put(x, y, '┌', color);
        self.put(right, y, '┐', color);
        self.put(x, bottom, '└', color);
        self.put(right, bottom, '┘', color);
    }

    fn render(&self, out: &mut impl Write, left: usize, top: usize) -> io::Result<()> {
        for (row, line) in self.cells.chunks(SCREEN_W).enumerate() {
            queue!(out, cursor::MoveTo(left as u16, (top + row) as u16))?;
            let mut current = None;
            for &(ch, color) in line {
                if current != Some(color) {
                    queue!(out, SetForegroundColor(color))?;
                    current = Some(color);
                }
                queue!(out, Print(ch))?;
            }
        }
        queue!(out, ResetColor)?;
        out.flush()
    }
}
