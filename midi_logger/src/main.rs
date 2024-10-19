use anyhow::{anyhow, Context, Result};
use heart_beat_logger::HeartbeatLogger;
use midir::{MidiInput, MidiInputPort, MidiInputPorts};
use std::error::Error;
use std::fs;
use std::io::{stdin, stdout, Write};
use std::sync::mpsc;
use std::sync::mpsc::Sender;

const PORT_FILE: &str = "last_port.txt";

enum Message {
    Keypress,
    CtrlC,
}

fn main() {
    match run() {
        Ok(_) => (),
        Err(err) => println!("Error: {}", err),
    }
}

fn set_up_ctr_c(sender: Sender<Message>) -> Result<(), Box<dyn Error>> {
    ctrlc::set_handler(move || {
        sender.send(Message::CtrlC).unwrap();
    })?;

    Ok(())
}

fn run() -> Result<(), Box<dyn Error>> {
    let midi_in = MidiInput::new("midir reading input")?;
    let in_ports = midi_in.ports();
    let chosen_port = choose_midi_port(&midi_in, &in_ports)?;
    let _ = save_port_selection(&midi_in, chosen_port);
    let chosen_port_name = midi_in.port_name(&chosen_port)?;

    let mut heart_beat = HeartbeatLogger::new_with_http_logger(
        5, // TODO: Don't hard code!
        60.0,
    );

    let (sender, receiver) = mpsc::channel();
    set_up_ctr_c(sender.clone())?;

    let midi_sender = sender.clone();
    // _conn_in needs to be a named parameter, otherwise it will be dropped immediately
    let _conn_in = midi_in.connect(
        chosen_port,
        "midir-read-input",
        move |_stamp, message, _| {
            // Check if the message is a Note On event (status byte & 0xF0 == 0x90)
            if message[0] & 0xF0 == 0x90 {
                midi_sender.send(Message::Keypress).unwrap();
            }
        },
        (),
    )?;

    println!(
        "Connection open, reading input from '{}' (press Ctrl-C to exit) ...",
        chosen_port_name
    );

    while let Ok(message) = receiver.recv() {
        match message {
            Message::Keypress => heart_beat.beat(),
            Message::CtrlC => break,
        }
    }

    heart_beat.stop();

    println!("Exiting...");
    Ok(())
}

fn save_port_selection(midi_in: &MidiInput, port: &MidiInputPort) -> Result<()> {
    let port_name = midi_in.port_name(port).context("Getting port name")?;
    fs::write(PORT_FILE, port_name).context("Saving to file")
}

fn load_last_port_name() -> Option<String> {
    fs::read_to_string(PORT_FILE).ok()
}

fn choose_last_port_selection<'a>(
    midi_in: &MidiInput,
    in_ports: &'a MidiInputPorts,
) -> Option<&'a MidiInputPort> {
    if let Some(last_name) = load_last_port_name() {
        return in_ports
            .iter()
            .find(|p| midi_in.port_name(p).unwrap() == last_name);
    }

    None
}

fn choose_midi_port<'a>(
    midi_in: &MidiInput,
    in_ports: &'a MidiInputPorts,
) -> Result<&'a MidiInputPort> {
    match in_ports.len() {
        0 => Err(anyhow!("no input port found")),
        1 => {
            println!(
                "Choosing the only available input port: {}",
                midi_in.port_name(&in_ports[0])?
            );
            Ok(&in_ports[0])
        }
        _ => {
            if let Some(last_port) = choose_last_port_selection(midi_in, in_ports) {
                Ok(last_port)
            } else {
                user_select_port(&midi_in, &in_ports)
            }
        }
    }
}

fn user_select_port<'a>(
    midi_in: &MidiInput,
    in_ports: &'a [MidiInputPort],
) -> Result<&'a MidiInputPort> {
    println!("\nAvailable input ports:");
    for (i, p) in in_ports.iter().enumerate() {
        println!("{}: {}", i, midi_in.port_name(p)?);
    }
    print!("Please select input port: ");
    stdout().flush()?;
    let mut input = String::new();
    stdin().read_line(&mut input)?;
    in_ports
        .get(input.trim().parse::<usize>()?)
        .context("Invalid port selected")
}
