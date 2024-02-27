use std::collections::VecDeque;
use crate::error::OtaErr;
use crate::transport::TransportOut;
use crate::gpio::GpioOut;
use serde_json::Value;

#[derive(PartialEq, Clone, Debug)]
pub enum DeviceOs {
    Ai,
    Hc,
    None,
}
macro_rules! ON {
    () => { 3 };
}

macro_rules! OFF {
    () => { 2 };
}

macro_rules! BLINK {
    () => { 1 };
}

macro_rules! SET {
    () => { "set" };
}

macro_rules! GET {
    () => { "get" };
}

macro_rules! STATUS {
    () => { "status" };
}




macro_rules! EventCodeAi {
    (Internet_Unavailable) => { 22 };
    (Internet_Available) => { 23 };
    (Server_Disconnected) => { 32 };
    (Server_Connected) => { 33 };
    (Server_Message) => { 3311 };
    (AI_Disconnected) => { 72 };
    (AI_Connected) => { 73 };
    (AI_Detected) => { 7311 };
    (Zigbee_Disconnected) => { 42 };
    (Zigbee_Connected) => { 43 };
    (Zigbee_JoinNetwork) => { 4401 };
    (Bluetooth_Disconnected) => { 52 };
    (Bluetooth_Connected) => { 53 };
    (Bluetooth_JoinNetwork) => { 5401 };
}



#[derive(Clone)]
pub enum GpioLogicIn { 
    Transport(Result<TransportOut, OtaErr>),
    Gpio(Result<GpioOut, OtaErr>),
    
}

#[derive(Debug,Clone)]
pub enum GpioLogicOut {
    None,

    KeepAliveEvent,

    LedOnEvent {led_pin:u64},
    LedOffEvent{led_pin:u64},
    LedBlinkEvent{led_pin:u64, blink:bool ,time:u16, fre:u16},
    LedBlinkContinueEvent{led_pin:u64, blink:bool, time: u16, fre: u16},

    RelayOnEvent{relay: usize, json_init: Value},
    RelayOffEvent{relay: usize, json_init: Value},

    ConfigRelayEvent,

    StopEvent,
    ButtonBlinkEvent,
    ReturnState,

    CheckTempCpuEvent,
}



pub struct OtaLogic {
    pub outputs: VecDeque<GpioLogicOut>,
    pub device: DeviceOs,
    pub id_mac: String,
    pub tick :u64,
}

impl OtaLogic {
    pub fn new(device: DeviceOs, mac:String) -> Self {
        let outputs = std::iter::once(GpioLogicOut::None).collect();
        OtaLogic {
            outputs: outputs,
            device: device,
            id_mac: mac,
            tick: 0,
        }
    }



    fn parse_data_string(&mut self, data: &str) -> (String, usize) {
        // Tách id... và value từ chuỗi data
        let parts: Vec<&str> = data.split("-").collect();
        let device_id = parts.get(1).map_or("", |&x| x);
        let led_index: usize = parts.get(2).map_or("0", |&x| x).parse().unwrap_or(0);
    
        (device_id.to_string(), led_index)
    }
    
    fn parse_led_ai(&mut self, event_code:u32) -> GpioLogicOut{

        let state = event_code %10;
        match event_code {
            EventCodeAi!(Internet_Unavailable) | 
            EventCodeAi!(Internet_Available) => {
                
                if state == OFF!() {
                    GpioLogicOut::LedOffEvent { led_pin: 0 }
                }
                else {
                    GpioLogicOut::LedBlinkEvent { led_pin: 0, blink:true, time:0, fre: 1000}
                }
            }
            
            EventCodeAi!(Server_Disconnected) |
            EventCodeAi!(Server_Connected)  |
            EventCodeAi!(Server_Message)   
            => {

                if state == OFF!() {
                    GpioLogicOut::LedBlinkEvent { led_pin: 0, blink: true, time:0, fre: 2000}
                }
                else if state == ON!() {
                    GpioLogicOut::LedOnEvent { led_pin: 0 }
                }
                else {
                    GpioLogicOut::LedBlinkEvent { led_pin: 0, blink:false, time:0, fre: 1000}
                }
            }


            EventCodeAi!(AI_Disconnected) |
            EventCodeAi!(AI_Connected) |
            EventCodeAi!(AI_Detected)  
            => {
                if state == OFF!() {
                    GpioLogicOut::LedOffEvent { led_pin: 1 }
                }
                else if state == ON!() {
                    GpioLogicOut::LedOnEvent { led_pin: 1 }
                }
                else {
                    GpioLogicOut::LedBlinkEvent { led_pin: 1, blink:false, time:0, fre: 1000}
                }
            }

            EventCodeAi!(Zigbee_Disconnected)  |
            EventCodeAi!(Zigbee_Connected) |
            EventCodeAi!(Zigbee_JoinNetwork)  
            => {
                if state == OFF!() {
                    GpioLogicOut::LedOffEvent { led_pin: 2 }
                }
                else if state == ON!() {
                    GpioLogicOut::LedOnEvent { led_pin: 2 }
                }
                else {
                    GpioLogicOut::LedBlinkEvent { led_pin: 2, blink:true, time:0, fre: 1000}
                }
            }


            EventCodeAi!(Bluetooth_Disconnected)  |
            EventCodeAi!(Bluetooth_Connected)  |
            EventCodeAi!(Bluetooth_JoinNetwork)  
            => {
                if state == OFF!() {
                    GpioLogicOut::LedOffEvent { led_pin: 3 }
                }
                else if state == ON!() {
                    GpioLogicOut::LedOnEvent { led_pin: 3 }
                }
                else {
                    GpioLogicOut::LedBlinkEvent { led_pin: 3, blink:true, time:0, fre: 1000}
                }
            }

            _ => {
                GpioLogicOut::None
            }
        }
    }

    fn relay_handle(&mut self, parsed_json:Value) -> GpioLogicOut{
        log::info!("Relay incoming");
        let data_value = parsed_json["objects"][0]["data"][0].as_str().unwrap_or_default();
        let (device_id, relay) = self.parse_data_string(data_value);
        let value = parsed_json["objects"][0]["execution"]["params"]["on"].as_bool().unwrap_or(false);
        if self.id_mac == device_id {
            if value == true {
                log::info!("Relay {} value is true",relay);
                return GpioLogicOut::RelayOnEvent{relay, json_init:parsed_json};
            }
            else {
                log::info!("Relay {} value is false",relay);
                return GpioLogicOut::RelayOffEvent{relay, json_init:parsed_json};
            }
        }
        return GpioLogicOut::None;
    }


    fn led_handle(&mut self, parsed_json:Value, device: DeviceOs) -> GpioLogicOut {
        log::info!("Led incoming");  
        if let Some(objects) = parsed_json.get("objects").and_then(Value::as_array) {
            for obj in objects {
                if let Some(data_array) = obj.get("data").and_then(Value::as_array) {
                    for data_obj in data_array {
                        if let Some(event_code) = data_obj.get("event_code").and_then(Value::as_str) {
                            // Ở đây bạn có thể làm bất cứ điều gì với event_code, ví dụ:
                            log::info!("Event Code: {}", event_code);
                            let event_code = event_code.parse::<u32>().unwrap();     
                            if device == DeviceOs::Hc {   
                                let mut pin = 0; 
                                pin = event_code / 10;
                                let state = event_code % 10;
                        
                                let mut event = GpioLogicOut::StopEvent;
                                log::info!("Pin: {}, state:{}", pin, state);
                                if pin < 2 {
                                    return event;
                                }
                                pin = pin - 2;
                                
                                match state {
                                    ON!() => event =  GpioLogicOut::LedOnEvent { led_pin: pin as u64 }, 
                                    OFF!() => event = GpioLogicOut::LedOffEvent {led_pin:pin as u64}, 
                                    BLINK!()=> event=GpioLogicOut::LedBlinkEvent {led_pin:pin as u64, blink:false, time:20, fre: 1},
                                    _ => {}
                                }
                                return event;
                            }   
                            // device ai
                            else {
                                return self.parse_led_ai(event_code);
                            }   
                        }
                    }
                }
            }
        }
        GpioLogicOut::None
    }

    fn sync_handle(&mut self,device: DeviceOs) -> GpioLogicOut {
        if device == DeviceOs::Ai {
            return GpioLogicOut::ConfigRelayEvent;
        }
        GpioLogicOut::None         
    }

    pub fn on_event(&mut self, _event:GpioLogicIn) {
        match _event {
            GpioLogicIn::Transport(result) => {
                match result {
                    Ok(transport ) => {
                        match transport {
                            TransportOut::ResponseMqttEvent(parsed_json) => {
                                let cmd = parsed_json["cmd"].as_str().unwrap_or("default_cmd");
                                match cmd {
                                    SET!() => {
                                        if let Some(_) = parsed_json.get("control_source") {
                                            // Relay...
                                            let res = self.relay_handle(parsed_json);
                                            self.outputs.push_back(res);
                        
                                        } else {
                                            // Led...
                                            let res = self.led_handle(parsed_json, self.device.clone());
                                            log::info!("res_led: {:?}", res);
                                            self.outputs.push_back(res);
                                        }
                                    }

                                    GET!() => {
                                        let res  = self.sync_handle( self.device.clone());
                                        log::info!("get: {:?}", res);
                                        self.outputs.push_back(res);
                                    }
                                    
                                    _ => {
 
                                    }
                                }

                            }
                        }
                    }
                    Err(e) => {
                        match e {
                            _ => {
                                
                            }
                        }
                    }
                }  
            }
            GpioLogicIn::Gpio(result) => {
                match result {
                    Ok(gpio)=>{
                        match gpio {
                            GpioOut::LedBlinkContinue {led_pin, blink, time, fre} => {
                                self.outputs.push_back(GpioLogicOut::LedBlinkContinueEvent {led_pin,  blink, time: time, fre: fre });
                            }
                            GpioOut::Stop => {
                              self.outputs.push_back(GpioLogicOut::StopEvent);  
                            }
                            GpioOut::ButtonPressed => {
                                self.outputs.push_back(GpioLogicOut::ButtonBlinkEvent);
                                log::info!("Logic button pressed");
                            }
                            GpioOut::ButtonReleased => {
                                log::info!("Logic button released");
                                self.outputs.push_back(GpioLogicOut::ReturnState);
                            }
                            GpioOut::LedTwoReleased => {
                                log::info!("Led two released");
                            }
                            GpioOut::LedThreeReleased => {
                                log::info!("Led three released");
                            }
                            GpioOut::LedFourReleased => {
                                log::info!("Led four released");
                            }
                            GpioOut::LedFiveReleased => {
                                log::info!("Led five released");
                            }
                        }
                    }

                    Err(e) =>{
                        match e {
                            _ =>  {
                                
                            }
                        }
                    }
                }
            }
        }
    }
    pub fn pop_action(&mut self) -> Option<GpioLogicOut> {
        self.outputs.pop_front()
    }

}

#[cfg(test)]
mod test {

    // use super::*;
   
}
