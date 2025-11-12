use crate::app::core::App;
use std::time::{SystemTime, UNIX_EPOCH};

impl App {
    pub fn next_item(&mut self) {
        let items_len = if self.is_online {
            self.tasks.len()
        } else {
            self.sculpt_items.len()
        };

        if items_len == 0 {
            return;
        }

        if self.is_online {
            let i = match self.tasks_state.selected() {
                Some(i) => {
                    if i >= items_len - 1 {
                        i
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.tasks_state.select(Some(i));
        } else {
            let i = match self.sculpt_state.selected() {
                Some(i) => {
                    if i >= items_len - 1 {
                        i
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.sculpt_state.select(Some(i));
        }
    }

    pub fn previous_item(&mut self) {
        let items_len = if self.is_online {
            self.tasks.len()
        } else {
            self.sculpt_items.len()
        };

        if items_len == 0 {
            return;
        }

        if self.is_online {
            let i = match self.tasks_state.selected() {
                Some(i) => {
                    if i == 0 {
                        0
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.tasks_state.select(Some(i));
        } else {
            let i = match self.sculpt_state.selected() {
                Some(i) => {
                    if i == 0 {
                        0
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.sculpt_state.select(Some(i));
        }
    }

    pub fn start_harvest_confirm(&mut self) {
        self.is_harvesting = true;
    }

    pub fn confirm_harvest(&mut self) {
        self.is_harvesting = false;
        self.success_message = Some("Harvest completed successfully!".to_string());
        self.harvestable_rewards = "0.00 SUI".to_string();
    }

    pub fn cancel_harvest(&mut self) {
        self.is_harvesting = false;
    }

    pub fn get_tech_animation(&self) -> String {
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let frame = (time % 3) as usize;

        // Show loading status if Sculpts are being loaded
        if self.is_loading_sculpts {
            return match frame {
                0 => "║▓▒░ LOADING SCULPTS ░▒▓║".to_string(),
                1 => "║▒▓░ LOADING SCULPTS ░▓▒║".to_string(),
                _ => "║░▓▒ LOADING SCULPTS ▒▓░║".to_string(),
            };
        }

        // only show one state: prioritize print_status
        match &self.print_status {
            crate::app::PrintStatus::Idle => {
                // check if stopping process
                if matches!(self.message_type, crate::app::MessageType::Info) && 
                   self.error_message.as_ref().map_or(false, |msg| msg.contains("Stopping print")) {
                    return "║▒▓░ STOPPING PRINT... ░▓▒║".to_string();
                }

                // only show script_status when no print
                match self.script_status {
                    crate::app::ScriptStatus::Idle => {
                        match frame {
                            0 => "║▓▒░ SYS IDLE ░▒▓║".to_string(),
                            1 => "║▒▓░ SYS IDLE ░▓▒║".to_string(),
                            _ => "║░▓▒ SYS IDLE ▒▓░║".to_string(),
                        }
                    },
                    crate::app::ScriptStatus::Running => "║▒▓░ SCRIPT RUNNING ░▓▒║".to_string(),
                    crate::app::ScriptStatus::Completed => "║▓▒░ SCRIPT COMPLETE ░▒▓║".to_string(),
                    crate::app::ScriptStatus::Failed(_) => "║▒▓░ SCRIPT ERROR ░▓▒║".to_string(),
                }
            }
            crate::app::PrintStatus::Printing => {
                match frame {
                    0 => "║▓▒░ PRINTING ░▒▓║".to_string(),
                    1 => "║▒▓░ PRINTING ░▓▒║".to_string(),
                    _ => "║░▓▒ PRINTING ▒▓░║".to_string(),
                }
            }
            crate::app::PrintStatus::Completed => "║▓▒░ PRINT COMPLETE ░▒▓║".to_string(),
            crate::app::PrintStatus::Error(_) => "║▒▓░ PRINTER ERROR ░▓▒║".to_string(),
        }
    }
}
