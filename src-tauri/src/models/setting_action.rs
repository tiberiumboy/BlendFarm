use crate::models::server_setting::ServerSetting;
use futures::channel::oneshot::Sender;

// This enum seems complete. Provider of Server Settings configuration.
#[derive(Debug)]
pub enum SettingsAction {
    Get(Sender<ServerSetting>),
    Update(ServerSetting),
}

// Ignore Sender type for now.
impl PartialEq for SettingsAction {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Get(..), Self::Get(..)) => true,
            (Self::Update(l0), Self::Update(r0)) => l0 == r0,
            _ => false,
        }
    }
}
