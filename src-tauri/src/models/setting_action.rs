use futures::channel::mpsc::Sender;
use crate::models::server_setting::ServerSetting;

#[derive(Debug)]
pub enum SettingsAction {
    Get(Sender<ServerSetting>),
    Update(ServerSetting),
}

impl PartialEq for SettingsAction {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Get(..), Self::Get(..)) => true,
            (Self::Update(l0), Self::Update(r0)) => l0 == r0,
            _ => false,
        }
    }
}