use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::{PathBuf};
use log::{debug, warn};

pub struct PortSelector {
    last_port_file: PathBuf,
}

impl Default for PortSelector {
    fn default() -> Self {
        Self::new_with_default_path()
            .expect("Failed to create default port selector")
    }
}

impl PortSelector {
    pub fn new(last_port_file: PathBuf) -> Self {
        Self { last_port_file }
    }

    pub fn new_with_default_path() -> Result<Self> {
        let mut path = dirs::config_dir()
            .ok_or_else(|| anyhow!("Could not determine config directory"))?;
        path.push("midi_logger");
        path.push("last_port.txt");

        Ok(Self::new(path))
    }

    pub fn select_port<'a>(
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

        fs::write(&self.last_port_file, &port_name)
            .context("Failed to save port name")?;

        debug!("Saved MIDI port selection to {}: {}", self.last_port_file.display(), port_name);
        Ok(())
    }

    fn select_last_port_from_available(
        &self,
        available_ports: &[String],
    ) -> Option<usize> {
        let last_name = self.load_last_port_name()?;
        let index = available_ports.iter()
            .position(|p| p.as_str() == last_name);

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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_port_selector() -> (PortSelector, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let port_file = temp_dir.path().join("last_port.txt");
        let selector = PortSelector::new(port_file);
        (selector, temp_dir)
    }

    #[test]
    fn test_port_selection_with_single_port() {
        let (selector, _temp_dir) = create_test_port_selector();
        let midi_in = MidiInput::new("test").unwrap();
        let ports = midi_in.ports();

        if !ports.is_empty() {
            let result = selector.select_port(&midi_in, &ports, |_| Ok(0));
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_save_and_load_port() {
        let (selector, _temp_dir) = create_test_port_selector();

        // Create a mock MIDI input and port
        let midi_in = MidiInput::new("test").unwrap();
        let ports = midi_in.ports();

        if !ports.is_empty() {
            // Save port selection
            selector.save_port_selection(&midi_in, &ports[0]).unwrap();

            // Verify port file contains port name
            let saved_name = fs::read_to_string(&selector.last_port_file).unwrap();
            let actual_name = midi_in.port_name(&ports[0]).unwrap();
            assert_eq!(saved_name, actual_name);

            // Test loading saved port
            let loaded_port = selector.select_last_port_from_available(&midi_in, &ports);
            assert!(loaded_port.is_some());
        }
    }

    #[test]
    fn test_invalid_port_file() {
        let (selector, _temp_dir) = create_test_port_selector();
        assert!(selector.load_last_port_name().is_none());
    }

    #[test]
    fn test_directory_creation() {
        let temp_dir = TempDir::new().unwrap();
        let port_file = temp_dir.path().join("subdir").join("last_port.txt");
        let selector = PortSelector::new(port_file.clone());

        let midi_in = MidiInput::new("test").unwrap();
        let ports = midi_in.ports();

        if !ports.is_empty() {
            selector.save_port_selection(&midi_in, &ports[0]).unwrap();
            assert!(port_file.exists());
            assert!(port_file.parent().unwrap().exists());
        }
    }
}