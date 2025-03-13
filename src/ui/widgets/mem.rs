use std::time::Instant;

use crossterm::event::KeyEvent;
use procfs::{
    process::{Process, SmapsRollup},
    ProcResult,
};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
};

use crate::{
    ui::{InputResult, TWO_SECONDS},
    util::fmt_bytes,
};

use super::AppWidget;

pub struct MemWidget {
    rollup: ProcResult<SmapsRollup>,
    last_updated: Instant,
}

impl MemWidget {
    pub fn new(proc: &Process) -> Self {
        Self {
            rollup: proc.smaps_rollup(),
            last_updated: Instant::now(),
        }
    }
}

impl AppWidget for MemWidget {
    const TITLE: &'static str = "Mem";

    fn draw(&mut self, f: &mut ratatui::Frame, area: Rect, _help_text: &mut Text) {
        let mut text: Vec<Line> = Vec::new();

        match &self.rollup {
            Ok(rollup) => {
                let keys = [
                    "Rss",
                    "Pss",
                    "Pss_Dirty",
                    "Pss_Anon",
                    "Pss_File",
                    "Pss_Shmem",
                    "Shared_Clean",
                    "Shared_Dirty",
                    "Private_Clean",
                    "Private_Dirty",
                    "Referenced",
                    "Anonymous",
                    "KSM",
                    "LazyFree",
                    "AnonHugePages",
                    "ShmemPmdMapped",
                    "FilePmdMapped",
                    "Shared_Hugetlb",
                    "Private_Hugetlb",
                    "Swap",
                    "SwapPss",
                    "Locked",
                ];
                let key_style = Style::default().fg(Color::Green);
                let data = &rollup.memory_map_rollup.0[0].extension.map;
                for key in keys {
                    if let Some(x) = data.get(key) {
                        text.push(Line::from(vec![
                            Span::styled(format!("{:20}", format!("{key}:")), key_style),
                            Span::raw(format!("{:>10}", fmt_bytes(*x, "B"))),
                        ]));
                    }
                }
            }
            Err(e) => {
                text.push(Line::from(Span::styled(
                    format!("Error getting memory rollup: {e}"),
                    Style::default().fg(Color::Red).bg(Color::Reset),
                )));
            }
        }

        let widget = Paragraph::new(text).block(Block::default().borders(Borders::NONE));
        f.render_widget(widget, area);
    }

    fn update(&mut self, proc: &Process) {
        if self.last_updated.elapsed() > TWO_SECONDS {
            self.rollup = proc.smaps_rollup();
            self.last_updated = Instant::now();
        }
    }

    fn handle_input(&mut self, _input: KeyEvent, _heightt: u16) -> InputResult {
        InputResult::None
    }
}
