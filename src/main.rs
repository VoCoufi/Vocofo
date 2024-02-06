use std::{
    io::{self, stdout},
    rc::Rc,
};

use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};

mod file_operation;

// TODO do something with this
static mut STATE_RIGHT: usize = 0;

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    stdout().execute(crossterm::event::EnableMouseCapture)?;
    stdout().execute(crossterm::event::EnableFocusChange)?;

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut should_quit = false;
    while !should_quit {
        terminal.draw(ui)?;
        should_quit = handle_events()?;
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    stdout().execute(crossterm::event::DisableMouseCapture)?;
    stdout().execute(crossterm::event::DisableFocusChange)?;
    Ok(())
}

fn handle_events() -> io::Result<bool> {
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                use KeyCode::*;
                match key.code {
                    Char('q') | Esc => return Ok(true),
                    Down => unsafe {
                        STATE_RIGHT += 1;
                    },
                    Up => unsafe {
                        STATE_RIGHT -= 1;
                    },
                    _ => {}
                }
            }
        }
    }
    Ok(false)
}

fn ui(frame: &mut Frame) {
    let main_layout = Layout::new(
        Direction::Vertical,
        [
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ],
    )
    .split(frame.size());
    frame.render_widget(
        Block::new().borders(Borders::TOP).title("Vocofo"),
        main_layout[0],
    );
    frame.render_widget(
        Paragraph::new("Q - Quit | C - Copy | P - Paste | R - Rename")
            .bold()
            .alignment(Alignment::Left),
        main_layout[2],
    );

    let inner_layout = Layout::new(
        Direction::Horizontal,
        [Constraint::Percentage(50), Constraint::Percentage(50)],
    )
    .split(main_layout[1]);
    frame.render_widget(
        Block::default().borders(Borders::ALL).title(path_left()),
        inner_layout[0],
    );

    render_right_directory(frame, inner_layout);
}

fn render_right_directory(frame: &mut Frame, inner_layout: Rc<[Rect]>) {
    let items = file_operation::list_children().unwrap();

    let list = List::new(items)
        .block(Block::default().title("List").borders(Borders::ALL))
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
        .highlight_symbol(">>")
        .repeat_highlight_symbol(true)
        .highlight_spacing(HighlightSpacing::WhenSelected)
        .direction(ListDirection::TopToBottom);

    unsafe {
        let mut state = ListState::default().with_selected(Some(STATE_RIGHT));

        frame.render_stateful_widget(list, inner_layout[1], &mut state);
    }
}

fn path_left() -> String {
    "Leva".to_string()
}
