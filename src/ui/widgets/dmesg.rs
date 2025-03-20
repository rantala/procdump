use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::os::unix::fs::OpenOptionsExt;

use crossterm::event::KeyEvent;
use procfs::process::Process;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::ui::ScrollController;

use super::AppWidget;

pub struct DmesgWidget {
    entries: VecDeque<KmsgEntry>,
    scroll: ScrollController,
    kmsg: Result<File, std::io::Error>,
}

const MAX_ENTRIES: usize = 10000;

#[derive(Debug)]
enum KmsgPriority {
    Emergency = 0,
    Alert = 1,
    Critical = 2,
    Error = 3,
    Warning = 4,
    Notice = 5,
    Info = 6,
    Debug = 7,
}

impl KmsgPriority {
    fn try_from(value: u8) -> Option<Self> {
        match value {
            0 => Some(KmsgPriority::Emergency),
            1 => Some(KmsgPriority::Alert),
            2 => Some(KmsgPriority::Critical),
            3 => Some(KmsgPriority::Error),
            4 => Some(KmsgPriority::Warning),
            5 => Some(KmsgPriority::Notice),
            6 => Some(KmsgPriority::Info),
            7 => Some(KmsgPriority::Debug),
            _ => None,
        }
    }
}

/// https://www.kernel.org/doc/Documentation/ABI/testing/dev-kmsg
#[derive(Debug)]
struct KmsgEntry {
    priority: KmsgPriority,
    // facility: u32,
    // sequence: u64,
    timestamp: u64,
    // flags: String,
    subsystem: String,
    content: String,
}

impl DmesgWidget {
    pub fn new(_proc: &Process) -> Self {
        let mut entries = VecDeque::new();
        entries.reserve_exact(MAX_ENTRIES);
        let kmsg = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK)
            .open("/dev/kmsg");
        Self {
            entries,
            scroll: ScrollController::new(),
            kmsg,
        }
    }
    pub fn draw_scrollbar(&self, f: &mut ratatui::Frame, area: Rect) {
        self.scroll.draw_scrollbar(f, area)
    }
    // Format: `priority,sequence_num,timestamp,flags;subsystem:content`
    fn parse_kmsg(line: &str) -> Option<KmsgEntry> {
        let parts: Vec<&str> = line.splitn(2, ';').collect();
        if parts.len() != 2 {
            return None;
        }
        let metadata: Vec<&str> = parts[0].split(',').collect();
        if metadata.len() != 4 {
            return None;
        }
        let prio: u32 = metadata[0].parse().ok()?;
        let priority = KmsgPriority::try_from((prio & 0x07) as u8)?;
        let content_parts: Vec<&str> = parts[1].splitn(2, ':').collect();
        Some(KmsgEntry {
            priority,
            // facility: (prio >> 3),
            // sequence: metadata[1].parse().ok()?,
            timestamp: metadata[2].parse().ok()?,
            // flags: metadata.get(3).map_or(String::new(), |&s| s.to_string()),
            subsystem: if content_parts.len() > 1 {
                content_parts[0].to_string()
            } else {
                String::new()
            },
            content: content_parts.last()?.to_string(),
        })
    }
}

impl AppWidget for DmesgWidget {
    const TITLE: &'static str = "Dmesg";

    fn draw(&mut self, f: &mut ratatui::Frame, area: Rect, _help_text: &mut Text) {
        if let Err(e) = &self.kmsg {
            let text = vec![Line::from(vec![Span::styled(
                format!("Error opening /dev/kmsg: {}", e),
                Style::default().fg(Color::Red),
            )])];
            f.render_widget(
                Paragraph::new(text).block(Block::default().borders(Borders::NONE)),
                area,
            );
            return;
        }

        let mut text: Vec<Line> = Vec::new();
        for entry in self.entries.iter().skip(self.scroll.scroll_offset as usize) {
            let mut spans = vec![Span::styled(
                format!("[{:12}] ", entry.timestamp),
                Style::default().fg(Color::Green),
            )];
            if !entry.subsystem.is_empty() {
                spans.push(Span::styled(&entry.subsystem, Style::default().fg(Color::Yellow)));
                spans.push(Span::raw(":"));
            }
            spans.push(Span::styled(
                &entry.content,
                match entry.priority {
                    KmsgPriority::Emergency | KmsgPriority::Alert | KmsgPriority::Critical | KmsgPriority::Error => Style::default().fg(Color::Red),
                    _ => Style::default(),
                },
            ));
            text.push(Line::from(spans));
        }
        f.render_widget(
            Paragraph::new(text).block(Block::default().borders(Borders::NONE)),
            area,
        );
    }

    fn update(&mut self, _proc: &Process) {
        if let Ok(kmsg) = self.kmsg.as_mut() {
            let mut buf = [0; 8096];
            while let Ok(len) = kmsg.read(&mut buf) {
                if len > 0 {
                    let entry = std::str::from_utf8(&buf[..len]);
                    if let Ok(entry) = entry {
                        if let Some(entry) = Self::parse_kmsg(&entry) {
                            if self.entries.len() >= MAX_ENTRIES {
                                self.entries.remove(0);
                            }
                            self.entries.push_back(entry);
                        }
                    }
                }
            }
            self.scroll.set_max_scroll(self.entries.len() as i32 - 1);
        }
    }

    fn handle_input(&mut self, input: KeyEvent, height: u16) -> super::InputResult {
        self.scroll.handle_input(input, height)
    }
}
