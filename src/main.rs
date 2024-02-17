use std::io::{self, stdout}
;

use crossterm::{
    event::{self, DisableFocusChange, DisableMouseCapture, EnableFocusChange, EnableMouseCapture, Event, KeyCode}, execute, terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}, ExecutableCommand
};
use ratatui::prelude::*;

mod file_operation;
mod ui;
mod context;

use crate::context::Context;

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen, EnableFocusChange, EnableMouseCapture)?;

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let context = &mut context::Context::new();

    loop {
        terminal.draw(| frame: &mut Frame<'_> | ui::ui(frame, context))?;

        if handle_events(context)? {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture, DisableFocusChange)?;

    Ok(())
}

fn handle_events(context: &mut Context) -> io::Result<bool> {
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                use KeyCode::*;

                match key.code {
                    Char('q') | Esc => return Ok(true),
                    Enter => context.open_item(),
                    // Tab will be changed
                    Tab => file_operation::open_dir(context),
                    Down =>
                        if context.items.len() > context.state + 1 {
                            Context::increment_state(context);
                        },
                    Up =>
                        if context.state > 0 {
                            context.decrease_state();
                        },
                    _ => {}
                }
            }
        }
    }
    Ok(false)
}
