use rumqttc::{MqttOptions, AsyncClient, EventLoop, Event, QoS};
use crate::error::OtaErr;
use tokio::time::Duration;
use super::TransportOut;
use tokio::sync::mpsc;
use serde_json::Value;


pub struct MqttDriver {
    pub tx: mpsc::Sender<Result<TransportOut, OtaErr>>,
    pub rx: mpsc::Receiver<Result<TransportOut, OtaErr>>,
    pub options: MqttOptions, 
    pub client: AsyncClient,
    pub eventloop: EventLoop,
    pub flag:bool,
}

impl MqttDriver { 
    pub async fn new(id:String, host:String, port: u16, keep_alive:u64) -> Self {
        let mut mqttoptions = MqttOptions::new(id, host, port);

        mqttoptions.set_credentials("component", "123");
        mqttoptions.set_keep_alive(Duration::from_secs(keep_alive));

        let (client, eventloop) = AsyncClient::new(mqttoptions.clone(), 10);

        client.subscribe("component/io/+", QoS::AtMostOnce).await.unwrap();

        let (tx, rx) = mpsc::channel::<Result<TransportOut, OtaErr>>(5);
        MqttDriver {
            tx: tx,
            rx: rx,
            options: mqttoptions.clone(),
            client: client,
            eventloop: eventloop, 
            flag:false                                        
        }
    }
    pub async fn send(&mut self, topic: String, message: Vec<u8>, qos: QoS, retain: bool)-> Result<(),OtaErr> {
        
        log::info!("--> {} : {}", topic, String::from_utf8_lossy(&message).to_string());

        match self.client.publish(topic, qos, retain, message).await {
            Ok(res) => {
                Ok(res)
            }
            Err(_) => {
                Err(OtaErr::MqttErr)
            }
        }
    }

    pub async fn recv(&mut self) -> Result<TransportOut, OtaErr> {
        loop {
            let event = self.eventloop.poll().await;
            match &event {
                Ok(v) => {
                    match v {
                        Event::Incoming(packet) => {
                            match packet {
                                rumqttc::Packet::Publish(publish) => {
                                    let payload_str: String = String::from_utf8_lossy(&publish.payload).to_string();
                                    let parsed_json: Value = serde_json::from_str(&payload_str).unwrap();
                                    let source = parsed_json["source"].as_str().unwrap();
                                    if source == "io" {
                                        
                                    }
                                    else {
                                        self.tx.send(Ok(TransportOut::ResponseMqttEvent(parsed_json.clone()))).await.unwrap();
                                        log::info!("<-- {}:{}",publish.topic ,payload_str);
                                        return self.rx.recv().await.unwrap();
                                    }
                                }
                                _ => {
                                }
                            }
                        }
                        Event::Outgoing(_) => {}
                    }
                }
                Err(e) => {
                    log::info!("Error = {e:?}");
                    self.tx.send(Err(OtaErr::MqttErr)).await.unwrap();
                    return self.rx.recv().await.unwrap();
                }
            }
            
        }
    }
}
