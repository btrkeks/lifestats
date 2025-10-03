use anyhow::{anyhow, Context, Result};
use log::{debug, warn};
use std::fs;
use std::path::PathBuf;

pub struct PortSelector {
    last_port_file: PathBuf,
}

impl Default for PortSelector {
    fn default() -> Self {
        Self::new_with_default_path().expect("Failed to create default port selector")
    }
}

impl PortSelector {
    pub fn new(last_port_file: PathBuf) -> Self {
        Self { last_port_file }
    }

    pub fn new_with_default_path() -> Result<Self> {
        let mut path =
            dirs::config_dir().ok_or_else(|| anyhow!("Could not determine config directory"))?;
        path.push("midi_logger");
        path.push("last_port.txt");

        Ok(Self::new(path))
    }

    pub fn select_port(
        &self,
        available_ports: &[String],
        user_selector: impl FnOnce(&[String]) -> Result<usize>,
    ) -> Result<usize> {
        match available_ports.len() {
            0 => Err(anyhow!("no input port found")),
            1 => {
                debug!("Only one port available, selecting it automatically");
                Ok(0)
            }
            _ => {
                if let Some(port) = self.select_last_port_from_available(available_ports) {
                    Ok(port)
                } else {
                    user_selector(available_ports)
                }
            }
        }
    }

    pub fn save_port_selection(&self, port_name: &str) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.last_port_file.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create the directory for storing the last port selection")?;
        }

        fs::write(&self.last_port_file, port_name).context("Failed to save port name")?;

        debug!(
            "Saved MIDI port selection to {}: {}",
            self.last_port_file.display(),
            port_name
        );
        Ok(())
    }

    fn select_last_port_from_available(&self, available_ports: &[String]) -> Option<usize> {
        let last_name = self.load_last_port_name()?;
        let index = available_ports.iter().position(|p| p.as_str() == last_name);

        if index.is_some() {
            debug!("Using last selected MIDI port: {}", last_name);
        } else {
            warn!("Last selected MIDI port not found: {}", last_name);
        }

        index
    }

    fn load_last_port_name(&self) -> Option<String> {
        match fs::read_to_string(&self.last_port_file) {
            Ok(name) => {
                debug!("Loaded last used MIDI port: {}", name);
                Some(name)
            }
            Err(e) => {
                debug!("Failed to load last MIDI port: {}", e);
                None
            }
        }
    }
}
