use std::io::{self, stdout}
;

use crossterm::{
    event::{self, DisableFocusChange, DisableMouseCapture, EnableFocusChange, EnableMouseCapture, Event, KeyCode}, 
    execute, 
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}
};
use ratatui::prelude::*;

mod file_operation;
mod ui;
mod context;
mod render;

use crate::context::Context;

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen, EnableFocusChange, EnableMouseCapture)?;

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let context = &mut Context::new();

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
            if key.kind == event::KeyEventKind::Press && !context.get_popup().unwrap() {
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
                    Char('p') => context.set_popup(),
                    _ => {}
                }
            } else if key.kind == event::KeyEventKind::Press {
                use KeyCode::*;
            
                match key.code {
                    Backspace => {
                        if !context.input.is_empty() {
                            context.input.pop();
                        }
                    }
                    Enter => {
                        context.set_popup();
                        file_operation::create_dir(context.path.clone() + "/" + context.get_input().unwrap()).expect("TODO: panic message");
                        
                        context.set_input(String::default());
                        context.state = 0;
                    }
                    Esc => {
                        context.set_popup();
                    }

                    Char(c) => {
                        context.input.push(c);
                    }

                    _ => {}
                }
            }
        }
    }
    Ok(false)
}
