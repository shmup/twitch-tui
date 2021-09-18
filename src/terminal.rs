use std::{io, time::Duration};

use anyhow::Result;
use chrono::offset::Local;
use futures::FutureExt;
use termion::{event::Key, input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
use tokio::{
    sync::mpsc::{Receiver, Sender},
    task::unconstrained,
};
use tui::{backend::TermionBackend, layout::Constraint, Terminal};

use crate::{
    handlers::{config::CompleteConfig, data::Data},
    ui::{chat::draw_chat_ui, keybinds::draw_keybinds_ui},
    utils::{
        app::{App, State},
        event,
        text::align_text,
    },
};

pub async fn ui_driver(
    config: CompleteConfig,
    mut app: App,
    tx: Sender<String>,
    mut rx: Receiver<Data>,
) -> Result<()> {
    let mut events = event::Events::with_config(event::Config {
        exit_key: Key::Null,
        tick_rate: Duration::from_millis(config.terminal.tick_delay),
    })
    .await;

    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let username_column_title = align_text(
        "Username",
        &config.frontend.username_alignment,
        config.frontend.maximum_username_length,
    );

    let mut column_titles = vec![
        username_column_title.to_owned(),
        "Message content".to_string(),
    ];

    let mut table_constraints = vec![
        Constraint::Length(config.frontend.maximum_username_length),
        Constraint::Percentage(100),
    ];

    if config.frontend.date_shown {
        column_titles.insert(0, "Time".to_string());

        table_constraints.insert(
            0,
            Constraint::Length(
                Local::now()
                    .format(config.frontend.date_format.as_str())
                    .to_string()
                    .len() as u16,
            ),
        );
    }

    app.column_titles = Option::Some(column_titles);
    app.table_constraints = Option::Some(table_constraints);

    let chat_config = config.clone();

    terminal.clear().unwrap();

    'outer: loop {
        if let Some(Some(info)) = unconstrained(rx.recv()).now_or_never() {
            app.messages.push_front(info);
        }

        terminal.draw(|mut frame| match app.state {
            State::Normal | State::Input => {
                draw_chat_ui(&mut frame, &mut app, chat_config.to_owned()).unwrap()
            }
            State::KeybindHelp => draw_keybinds_ui(&mut frame, chat_config.to_owned()).unwrap(),
        })?;

        if let Some(event::Event::Input(input_event)) = &events.next().await {
            match app.state {
                State::Input => match input_event {
                    Key::Char('\n') => {
                        let input_message: String = app.input_text.drain(..).collect();
                        app.messages.push_front(Data::new(
                            Local::now()
                                .format(config.frontend.date_format.as_str())
                                .to_string(),
                            config.twitch.username.to_string(),
                            input_message.clone(),
                            false,
                        ));

                        tx.send(input_message).await.unwrap();
                    }
                    Key::Char(c) => {
                        app.input_text.push(c.to_string().parse().unwrap());
                    }
                    Key::Backspace => {
                        app.input_text.pop();
                    }
                    Key::Esc => {
                        app.state = State::Normal;
                    }
                    _ => {}
                },
                _ => match input_event {
                    Key::Char('c') => app.state = State::Normal,
                    Key::Char('?') => app.state = State::KeybindHelp,
                    Key::Char('i') => {
                        if config.frontend.input {
                            app.state = State::Input
                        }
                    }
                    Key::Esc => match app.state {
                        State::Normal => break 'outer,
                        State::KeybindHelp | State::Input => app.state = State::Normal,
                    },
                    _ => {}
                },
            }
        }
    }

    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     use crate::{terminal, App};
//     use std::fs;
//
//     fn setup() -> (CompleteConfig, App) {
//         let config: CompleteConfig =
//             toml::from_str(fs::read_to_string("default-config.toml").unwrap().as_str()).unwrap();
//
//         let app = App::new(config.terminal.maximum_messages as usize);
//
//         (config, app)
//     }
//
//     // This test can only be ran in full-fledged terminals,
//     // built-in terminals ones similar to Visual Studio Code and anything that
//     // JetBrains has made will not work.
//     // If the TUI detects that it cannot run because of terminal type, it will pass.
//     // This test is still in the progress of being made.
//     #[test]
//     fn test_chunk_height_limit() {
//         let (config, app) = setup();
//
//         let (tx, rx) = std::sync::mpsc::channel();
//         let cloned_config = config.clone();
//
//         std::thread::spawn(move || {
//             for i in 1..100 {
//                 tx.send(Data::new(
//                     Local::now()
//                         .format(&config.frontend.date_format.as_str())
//                         .to_string(),
//                     i.to_string(),
//                     i.to_string(),
//                     false,
//                 ))
//                 .unwrap();
//             }
//         });
//
//         terminal::ui_driver(cloned_config, app, rx).unwrap();
//     }
// }
