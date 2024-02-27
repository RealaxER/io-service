use tokio::{time::{interval,Interval, Duration}, select};
use crate::{gpio::GpioIn, json::JsonDriver, logic::OtaLogic, transport::mqtt::MqttDriver};
use crate::logic::{GpioLogicOut,GpioLogicIn,DeviceOs};
use crate::error::OtaErr;
use crate::gpio::GpioDriver;
use crate::gpio::ButtonDriver;
use crate::gpio::StatusGpio;
use crate::json::JsonIn;
use tokio::time::sleep;

macro_rules! WAIT_UNLOCK {
    ($object:expr, $event:expr) => { 
        {
            if $object.gpio.status == StatusGpio::ButtonPre {
                sleep(Duration::from_millis(100)).await;
                $object.logic.outputs.push_back($event);
                return Ok(());
            }
        }
    };
}
enum TemperatureLevel {
    //Cpu temperature level
    
    Cool = 0,
    CloseWarm = 1,
    Warm = 2,
    CloseHot = 3,
    Hot = 4,
    None = 5,
}

macro_rules! LOCK {
    ($object:expr) => { 
        {
            $object.gpio.status = StatusGpio::ButtonPre;
        }
    };
}

macro_rules! UNLOCK {
    ($object:expr) => { 
        {
            $object.gpio.status = StatusGpio::LedCtrl;
        }
    };
}

pub struct SystemIntergration {
    interval: Interval,
    pub logic: OtaLogic,
    transport: MqttDriver,
    gpio: GpioDriver,
    button: ButtonDriver,
    json: JsonDriver,
    index: usize,
}

impl SystemIntergration {
    pub async fn new(device:String, id_mac:String, led_vec:Vec<u64>, io_vec:Vec<u64>, fan_vec:Vec<u64>, time_blink:u64, button_pin:u64) -> Self {
        let mut _device = DeviceOs::None;
        if device  == "Ai".to_string(){
            _device = DeviceOs::Ai;
        }
        else {
            _device = DeviceOs::Hc;
        }

        SystemIntergration {
            interval: interval(Duration::from_millis(100)),
            logic: OtaLogic::new(_device, id_mac),
            transport: MqttDriver::new(
                "io_service".to_string(),
                "localhost".to_string(),
                1883,
                5,  
            ).await,
            gpio: GpioDriver::new(led_vec, io_vec,fan_vec, time_blink, button_pin),
            button: ButtonDriver::new(button_pin),
            json: JsonDriver{},
            index: 0,
        }
    }

    pub async fn recv(&mut self) -> Result<(),OtaErr> {
        select! {
            _ = self.interval.tick() => {
                let _ = self.button.button_handle().await;
                self.logic.tick += 1;
                self.index +=1;

                if self.index >= 400 { //(40s) 
                    self.index = 0;
                    self.logic.outputs.push_back(GpioLogicOut::KeepAliveEvent);
                    self.logic.outputs.push_back(GpioLogicOut::CheckTempCpuEvent);
                }
            },

            etransport  = self.transport.recv() =>{
                self.logic.on_event(GpioLogicIn::Transport(etransport));
            },

            egpio = self.gpio.recv() => {
                self.logic.on_event(GpioLogicIn::Gpio(egpio));
            }
             
            ebutton = self.button.recv() => {
                self.logic.on_event(GpioLogicIn::Gpio(ebutton));
            }
        
        }
          
        while let Some(out) = self.logic.pop_action() {
            match out {
                    GpioLogicOut::LedOnEvent{led_pin} => {
                        WAIT_UNLOCK!(self, GpioLogicOut::LedOnEvent {led_pin:led_pin});
                        log::info!("On light event");
                        self.gpio.send(GpioIn::LedOn {pin: led_pin}).await.unwrap();
                    }
                    GpioLogicOut::LedOffEvent{led_pin}  => {
                        WAIT_UNLOCK!(self, GpioLogicOut::LedOffEvent {led_pin:led_pin});
                        log::info!("On off event");
                        self.gpio.send(GpioIn::LedOff {pin: led_pin}).await.unwrap();
                    }

                    GpioLogicOut::LedBlinkContinueEvent {led_pin, blink, time, fre }=> {
                        WAIT_UNLOCK!(self, GpioLogicOut::LedBlinkContinueEvent  {led_pin: led_pin, blink:blink ,time: time, fre: fre });
                        self.gpio.send(GpioIn::LedBlink {pin: led_pin ,blink:blink , time: time , fre: fre , get_tick: self.logic.tick.clone() }).await.unwrap();
                    }

                    GpioLogicOut::LedBlinkEvent{led_pin, blink, time, fre } => {
                        WAIT_UNLOCK!(self, GpioLogicOut::LedBlinkEvent {led_pin: led_pin, blink:blink ,time: time, fre: fre});
                        self.gpio.status = StatusGpio::Blink;
                        log::info!("On blink event");
                        log::info!("Fre new : {:?}", fre);
                        let check = self.gpio.blink_parse(led_pin, fre).await;
                        match check {
                            Ok(()) => self.gpio.send(GpioIn::LedBlink {pin: led_pin, blink:blink ,time: time, fre: fre , get_tick: self.logic.tick.clone() }).await.unwrap(),
                            _ => {}
                        }
                    }
                    GpioLogicOut::ButtonBlinkEvent => {
                        LOCK!(self);
                        self.gpio.send(GpioIn::ButonBlink).await.unwrap();
                    }
                    GpioLogicOut::ReturnState => {
                        UNLOCK!(self);
                        log::info!("Returning state");
                        self.gpio.send(GpioIn::ReturnState).await.unwrap();
                    }

                    GpioLogicOut::RelayOnEvent{relay,json_init} => {
                        self.gpio.send(GpioIn::RelayOn{pin:relay as u64}).await.unwrap();

                        let topic = "component/io/status".to_string();
                        let pin: Vec<(bool, String)> = vec![
                            (true, format!("io-{}-{}", self.logic.id_mac.clone(),relay))
                        ];
                        let (mess, _) = self.json.convert(JsonIn::StatusConvert {json_init,pin}).await;

                        self.transport.send(topic, mess.into(), rumqttc::QoS::AtMostOnce, false).await.unwrap();
                    }

                    GpioLogicOut::RelayOffEvent{relay, json_init} => {
                        self.gpio.send(GpioIn::RelayOff{pin:relay as u64}).await.unwrap();

                        let topic = "component/io/status".to_string();
                        let pin: Vec<(bool, String)> = vec![
                            (false, format!("io-{}-{}", self.logic.id_mac.clone(),relay))
                        ];
                        let (mess, _) = self.json.convert(JsonIn::StatusConvert {json_init,pin}).await;

                        self.transport.send(topic, mess.into(), rumqttc::QoS::AtMostOnce, false).await.unwrap();
                    }

                    GpioLogicOut::ConfigRelayEvent=> {
                        let status = self.gpio.get_value_relay().await;
                        let (mess_sync , mess_st ) = self.json.convert( JsonIn::SyncConvert{status, mac_id:self.logic.id_mac.clone()}).await;

                        // config
                        let topic_sync = "component/io/config".to_string();
                        self.transport.send(topic_sync, mess_sync.into(), rumqttc::QoS::AtMostOnce, false).await.unwrap();

                        // status
                        let topic_st = "component/io/status".to_string();
                        self.transport.send(topic_st, mess_st.into(), rumqttc::QoS::AtMostOnce, false).await.unwrap();
                    }

                    GpioLogicOut::KeepAliveEvent =>{
                        log::info!("Keep alive event");
                        let topic = "component/keepalive/io-manager".to_string();                        

                        let (mess, _) = self.json.convert(JsonIn::KeepAlive).await;
                        self.transport.send(topic, mess.into(), rumqttc::QoS::AtMostOnce, false).await.unwrap();
                    }

                    GpioLogicOut::CheckTempCpuEvent => {
                        log::info!("Check temperature !!!");
                        match self.gpio.check_temp().await{
                            Ok(cpu_temperature) => {
                                self.gpio.control_fan(cpu_temperature as i32).await;
                            }

                            Err(_) => {

                            }
                        }
                    }

                    _ => {

                    }
                }
            }
        Ok(())
        }
        
}