pub mod mqtt;
pub mod dbus;
use crate::error::OtaErr;
use serde_json::Value;


pub enum TransportIn {
    
}

#[derive(Clone)]
pub enum TransportOut {
    ResponseMqttEvent(Value),
}

#[async_trait::async_trait]
pub trait Transport {
    async fn send(&mut self, data: TransportIn) -> Result<(), OtaErr>;
    async fn recv(&mut self) -> Result<TransportOut, OtaErr>;
}

