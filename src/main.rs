// - Play tones together
// - Wave viewer
// - Wave types
// - Wave selectors/modifier
// - Play a midi
// - Record a midi from key presses
// - Save a midi
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossterm::{
    event::{
        poll, read, DisableBracketedPaste, DisableFocusChange, DisableMouseCapture,
        EnableBracketedPaste, EnableFocusChange, EnableMouseCapture, Event, KeyCode,
    },
    execute,
};
use std::f64::consts::PI;
use std::io;
use std::sync::mpsc;
use std::thread;

mod frequencies;
use frequencies::Note;
use std::time::Duration;

struct SinWave {
    hz: f64,
    t: u64,
}

impl SinWave {
    fn new(hz: f64) -> Self {
        SinWave { hz, t: 0 }
    }
}

impl Iterator for SinWave {
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        let amplitude = (2.0 * PI * self.hz * (self.t as f64) / 44000.0).sin();
        self.t += 1;
        Some(amplitude)
    }
}

struct CombinedWave {
    waves: Vec<SinWave>,
}

impl Iterator for CombinedWave {
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        let mut amplitude = 0.0;
        for wave in &mut self.waves {
            let wave_iter = wave.into_iter();
            amplitude += wave_iter.next().unwrap_or(0.0);
        }
        return Some(amplitude / self.waves.len() as f64);
    }
}

fn play_note(note: Note) -> Result<(), Box<dyn std::error::Error>> {
    let mut wave = SinWave::new(frequencies::frequency(note));
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("Did not find default output device");
    let config = device.default_output_config().unwrap();

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);
    let stream_config: cpal::StreamConfig = config.into();

    let stream = device.build_output_stream(
        &stream_config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| write_data(data, 2, &mut wave),
        err_fn,
        None,
    )?;
    stream.play()?;

    std::thread::sleep(std::time::Duration::from_millis(100));
    println!("played a note\r");
    Ok(())
}

enum CaptureEvent {
    Play(ReplaceEvent),
    Unhandled,
    Break,
}

struct ReplaceEvent {
    note: Note,
}

fn capture_input(tx: mpsc::Sender<Note>) -> Result<(), io::Error> {
    crossterm::terminal::enable_raw_mode()?;
    execute!(
        std::io::stdout(),
        EnableBracketedPaste,
        EnableFocusChange,
        EnableMouseCapture,
    )?;

    loop {
        if poll(Duration::from_millis(1))? {
            let result = match read()? {
                Event::Key(event) => match event.code {
                    KeyCode::Char('a') => CaptureEvent::Play(ReplaceEvent { note: Note::C3 }),
                    KeyCode::Char('w') => CaptureEvent::Play(ReplaceEvent {
                        note: Note::Csharp3,
                    }),
                    KeyCode::Char('s') => CaptureEvent::Play(ReplaceEvent { note: Note::D3 }),
                    KeyCode::Char('e') => CaptureEvent::Play(ReplaceEvent {
                        note: Note::Dsharp3,
                    }),
                    KeyCode::Char('d') => CaptureEvent::Play(ReplaceEvent { note: Note::E3 }),
                    KeyCode::Char('f') => CaptureEvent::Play(ReplaceEvent { note: Note::F3 }),
                    KeyCode::Char('t') => CaptureEvent::Play(ReplaceEvent {
                        note: Note::Fsharp3,
                    }),
                    KeyCode::Char('g') => CaptureEvent::Play(ReplaceEvent { note: Note::G3 }),
                    KeyCode::Char('y') => CaptureEvent::Play(ReplaceEvent {
                        note: Note::Gsharp3,
                    }),
                    KeyCode::Char('h') => CaptureEvent::Play(ReplaceEvent { note: Note::A3 }),
                    KeyCode::Char('u') => CaptureEvent::Play(ReplaceEvent {
                        note: Note::Asharp3,
                    }),
                    KeyCode::Char('j') => CaptureEvent::Play(ReplaceEvent { note: Note::B3 }),
                    KeyCode::Char('k') => CaptureEvent::Play(ReplaceEvent { note: Note::C4 }),
                    KeyCode::Char('q') => CaptureEvent::Break,
                    _ => CaptureEvent::Unhandled,
                },
                _ => CaptureEvent::Unhandled,
            };

            match result {
                CaptureEvent::Play(event) => {
                    tx.send(event.note).unwrap();
                }
                CaptureEvent::Break => break,
                CaptureEvent::Unhandled => (),
            }
        }
    }
    crossterm::terminal::disable_raw_mode()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = mpsc::channel();

    let capture_input_handle = thread::spawn(move || {
        let _ = capture_input(tx);
    });

    loop {
        match rx.recv() {
            Ok(msg) => {
                println!("got a message!\r");
                let _ = play_note(msg);
            }
            Err(e) => {
                eprintln!("Oh no!: {}", e);
                break;
            }
        }
    }
    capture_input_handle.join().unwrap();
    Ok(())
}

fn write_data(output: &mut [f32], channels: usize, next_sample: &mut dyn Iterator<Item = f64>) {
    for frame in output.chunks_mut(channels) {
        let sample = next_sample.next().unwrap();
        for s in frame.iter_mut() {
            *s = sample as f32;
        }
    }
}
