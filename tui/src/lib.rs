use std::io::{stdout, Result, Stdout};

use async_signal::{Signal, Signals};
use futures::{
    channel::mpsc::{self, Sender},
    stream, Future, FutureExt, SinkExt, StreamExt,
};
pub use ratatui;
use ratatui::{
    prelude::CrosstermBackend,
    text::Line,
    widgets::{Paragraph, Widget},
    Frame, TerminalOptions, Viewport,
};

pub mod widget;

pub type Backend = CrosstermBackend<Stdout>;

pub fn run<P: Program, T: Send, F>(mut program: P, f: impl Fn(Handle<P::Message>) -> F) -> Result<T>
where
    F: Future<Output = T> + Send,
{
    smol::block_on(async move {
        // Ctrl-c capture
        let ctrl_c = Signals::new([Signal::Int])?;

        // Setup terminal
        let mut terminal = ratatui::Terminal::with_options(
            CrosstermBackend::new(stdout()),
            TerminalOptions {
                viewport: Viewport::Inline(P::LINES),
            },
        )?;

        // Draw initial view
        terminal.draw(|frame| {
            program.draw(frame);
        })?;

        // Setup channel
        let (sender, receiver) = mpsc::channel(10);

        // We can receive user event or finished status
        enum Input<P, T> {
            Event(Event<P>),
            Finished(T),
            Term,
        }

        // Run task
        let mut run = f(Handle { sender })
            .boxed()
            .map(Input::<P::Message, _>::Finished)
            .into_stream();
        // Channel task
        let mut receiver = receiver.map(Input::<_, T>::Event);
        // Ctrl c task
        let mut ctrl_c = ctrl_c.map(|_| Input::Term);

        loop {
            // Get next input
            let input = stream::select(&mut run, stream::select(&mut receiver, &mut ctrl_c))
                .next()
                .await
                .unwrap();

            match input {
                Input::Event(event) => match event {
                    Event::Message(message) => {
                        // Update
                        program.update(message);
                        // Draw
                        terminal.draw(|frame| program.draw(frame))?;
                    }
                    Event::Print(content) => {
                        let lines = content.lines().collect::<Vec<_>>();
                        let num_lines = lines.len();
                        let paragraph =
                            Paragraph::new(lines.into_iter().map(Line::from).collect::<Vec<_>>());

                        terminal.insert_before(num_lines as u16, |buf| {
                            paragraph.render(buf.area, buf)
                        })?;
                        terminal.draw(|frame| program.draw(frame))?;
                    }
                },
                Input::Finished(t) => {
                    terminal.show_cursor()?;
                    terminal.clear()?;
                    return Ok(t);
                }
                Input::Term => {
                    terminal.show_cursor()?;
                    terminal.clear()?;
                    std::process::exit(0);
                }
            }
        }
    })
}

pub trait Program: Sized {
    type Message;

    const LINES: u16;

    fn update(&mut self, message: Self::Message);

    fn draw(&self, frame: &mut Frame<Backend>);
}

pub struct Handle<Message> {
    sender: Sender<Event<Message>>,
}

impl<Message> Clone for Handle<Message> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

impl<Message> Handle<Message> {
    pub async fn print(&mut self, content: String) {
        let _ = self.sender.send(Event::Print(content)).await;
    }

    pub async fn update(&mut self, message: Message) {
        let _ = self.sender.send(Event::Message(message)).await;
    }
}

enum Event<Message> {
    Message(Message),
    Print(String),
}
