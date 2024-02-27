use sysfs_gpio::{Direction,Pin};
use crate::error::OtaErr;
use tokio::time::{sleep,Duration};
use tokio::sync::mpsc;
use std::sync::{Arc, Mutex};
use tokio::fs::File;
use tokio::io::{self, AsyncReadExt};

macro_rules! ON {
    () => { 0 };
}

macro_rules! OFF {
    () => { 1 };
}

macro_rules! BLINK {
    () => { 2 };
}



pub enum GpioIn {
    LedOn {pin:u64},
    LedOff{pin:u64},
    LedBlink {pin:u64, blink:bool, time: u16, fre: u16, get_tick:u64},

    RelayOn {pin:u64},
    RelayOff {pin:u64},

    ButonBlink, 
    ReturnState,

    FanModeLv1,
    FanModeLv2,
    FanModeLv3,


}
#[derive(Clone)]
pub enum GpioOut {
    Stop,
    LedBlinkContinue{led_pin :u64, blink:bool, time: u16, fre: u16},
    ButtonPressed,
    ButtonReleased,
    LedTwoReleased,
    LedThreeReleased,
    LedFourReleased,
    LedFiveReleased,
}
#[derive(PartialEq)]
pub enum StatusGpio {
    Blink,
    ButtonPre,
    LedCtrl 
}

pub struct GpioDriver {
    leds:Vec<(Pin, u8, u64,u8, u16)>,
    fan:Vec<Pin>,
    io:Vec<(Pin, u8)>,
    button_pin:u64,
    pub status: StatusGpio,
    pub tx: mpsc::Sender<Result<GpioOut, OtaErr>>,
    pub rx: mpsc::Receiver<Result<GpioOut, OtaErr>>,
    time_blink: u64,
    pub last_cpu_temperature:u32,
}


pub struct ButtonDriver {
    button: Pin,
    pub tx: mpsc::Sender<Result<GpioOut, OtaErr>>,
    pub rx: mpsc::Receiver<Result<GpioOut, OtaErr>>,
    temp: Arc<Mutex<u8>>, // Biến temp được đặt ở đây
}

impl ButtonDriver {
    pub fn new(button_pin: u64) -> ButtonDriver {
        let (tx, rx) = mpsc::channel::<Result<GpioOut, OtaErr>>(5);
        ButtonDriver {
            button: Pin::new(button_pin),
            tx: tx,
            rx: rx,
            temp: Arc::new(Mutex::new(1)), // Khởi tạo temp với giá trị ban đầu là 0
        }
    }

    pub async fn button_handle(&mut self) -> Result<(), OtaErr> {
        self.button.export().map_err(|_| OtaErr::SelectPinErr)?;
        self.button.set_direction(Direction::In).map_err(|_| OtaErr::SetDirectionErr)?;

        let val = self.button.get_value().unwrap();
        // Sử dụng temp_clone để cập nhật temp_value
        let mut temp_guard = self.temp.lock().unwrap();
        if *temp_guard != val {
            if val == 0 {
                log::info!("Button pressed");
                let _ = self.tx.send(Ok(GpioOut::ButtonPressed)).await;
            }
            else {
                log::info!("Button released");
                let _ = self.tx.send(Ok(GpioOut::ButtonReleased)).await;
            }
            *temp_guard = val;
        }
        drop(temp_guard);
        Ok(())
    }

    pub async fn recv(&mut self) -> Result<GpioOut, OtaErr> {
        self.rx.recv().await.unwrap()
    }

}



pub async fn toogle_led(pin: Pin) {
    if pin.get_value().unwrap() == 0 {
        pin.set_value(1).unwrap();
    }else {
        pin.set_value(0).unwrap();
    }
}

impl GpioDriver {
    pub fn new(led_vec:Vec<u64>, io_vec:Vec<u64>, fan_vec:Vec<u64>,time_blink:u64, button_pin:u64) -> GpioDriver {
        let (tx, rx) = mpsc::channel::<Result<GpioOut, OtaErr>>(5);


        // config leds 
        let mut leds = Vec::new();
        for pin in led_vec {
            let led_pin = Pin::new(pin);
            leds.push((led_pin, 1, 0, 0, 0)); // Initialize LED state, assuming 0 for OFF
        }

        // config io
        let mut ios = Vec::new();
        for io in io_vec {
            let io_pin = Pin::new(io);
            ios.push((io_pin, 1)); // Initialize
        }

        // config fan 
        let mut fans = Vec::new();
        for fan in fan_vec {
            let fan_pin = Pin::new(fan);
            fans.push(fan_pin); // Initialize
        }

        GpioDriver {
            leds:leds,
            fan:fans,
            io:ios,
            button_pin: button_pin,
            status: StatusGpio::LedCtrl,
            tx: tx,
            rx: rx,
            time_blink: time_blink,
            last_cpu_temperature:0,
        }
    }

    pub async fn recv(&mut self)-> Result<GpioOut,OtaErr> {
        self.rx.recv().await.unwrap()
    }

    pub async fn send(&mut self,event:GpioIn)-> Result<(),OtaErr> {
        match event {
            GpioIn::LedOn{pin} => {
                log::info!("on");
                if let Some((led, state, _, _, _)) = self.leds.iter_mut().nth(pin as usize) {
                    *state = ON!();
                    // led.export().map_err(|_| {OtaErr::SelectPinErr})?;
                    // led.set_direction(Direction::Out).map_err(|_| {OtaErr::SetDirectionErr})?;
                    // led.set_value(0).map_err(|_| {OtaErr::SetValueErr})?; 
                }
                Ok(())
            }

            GpioIn::LedOff{pin} => {
                log::info!("off");
                if let Some((led, state,_, _, _)) = self.leds.iter_mut().nth(pin as usize) {
                    *state = OFF!();
                    // led.export().map_err(|_| {OtaErr::SelectPinErr})?;
                    // led.set_direction(Direction::Out).map_err(|_| {OtaErr::SetDirectionErr})?;
                    // led.set_value(1).map_err(|_| {OtaErr::SetValueErr})?; 
                }
                Ok(())
            }

            GpioIn::LedBlink{time,blink, fre, pin, get_tick} => {
                if let Some((led,state, tick, index, temp)) = self.leds.iter_mut().nth(pin as usize) { 
                    log::info!("Temp:{}",temp);
                    if *temp == fre {
                        let _fre = fre /100;
                        if *index == 0 {
                            *state = BLINK!();
                        }
    
                        if *state != BLINK!() {
                            *index = 0;
                            return Ok(());
                        }
                        if get_tick - *tick >= _fre as u64{ 
                            *tick = get_tick;
                            *index += 1;                
                            //toogle_led(*led).await;
                            log::info!("Time:{}, fre:{}",time,fre);
                            log::info!("Toggle led");
    
    
                            if blink == false {
                                if *index >= time as u8 {
                                    *index = 0;
                                    return Ok(());
                                }
                            }  
                        }
                        let state_clone = state.clone();
                        let tx_clone = self.tx.clone();
                        let _ = tokio::spawn(async move{
                            if state_clone == BLINK!() {
                                sleep(Duration::from_millis(fre.into())).await;
                                tx_clone.send(Ok(GpioOut::LedBlinkContinue{led_pin :pin, blink:blink, time: time, fre: fre})).await.unwrap();
                            }    
                        });
                    }
                } 
                Ok(())
            }   
            GpioIn::ButonBlink => {
                let led_clone = self.leds.clone();
                let button = Pin::new(self.button_pin);
                let time_clone = self.time_blink.clone();
                let tx_clone = self.tx.clone();
                
                let _ = tokio::spawn(async move{
                    for &(led, _, _,_ , _) in &led_clone 
                    {
                        led.export().map_err(|_| {OtaErr::SelectPinErr}).unwrap();
                        led.set_direction(Direction::Out).map_err(|_| {OtaErr::SetDirectionErr}).unwrap();
                        led.set_value(1).map_err(|_| {OtaErr::SetValueErr}).unwrap();
                    }
                    for (index, &(led, _, _,_ , _)) in led_clone.iter().enumerate() {
                        
                        sleep(Duration::from_millis(time_clone)).await;
                        if button.get_value().unwrap() == 1 {
                            
                            break;
                        }
                      
                        led.set_value(0).map_err(|_| {OtaErr::SetValueErr}).unwrap();
                        
                        if index == led_clone.len() - 1 {
                            sleep(Duration::from_millis(time_clone)).await;
                        }
                    }
                    if button.get_value().unwrap() == 0 {
                        for &(led, _, _,_ , _) in &led_clone {
                            if button.get_value().unwrap() == 1 {
                                break;
                            }
                            led.set_value(1).map_err(|_| {OtaErr::SetValueErr}).unwrap();
                        }
                       let _ = tx_clone.send(Ok(GpioOut::ButtonPressed)).await.unwrap(); 
                    }  
                });

                
                Ok(())
            }

            GpioIn::ReturnState => {
                // get mode
                for (mut index, &(led, _, _,_ , _)) in self.leds.iter().enumerate() {
                    let status = led.get_value().map_err(|_| {OtaErr::SetValueErr}).unwrap();
                    if status == OFF!() {
                        index  -= 1;
                        let event = match index {
                            0 => GpioOut::LedTwoReleased,
                            1 => GpioOut::LedThreeReleased,
                            2 => GpioOut::LedFourReleased,
                            3 => GpioOut::LedFiveReleased,
                            _ => GpioOut::Stop,
                        }; 
                        let _ = self.tx.send(Ok(event)).await.unwrap();
                        break;
                    }
                }
                // return state 
                for (led, state, _,_ , _) in &self.leds{
                    // Extract pin and state from the tuple
                    // Process pin and state here
                    if *state == BLINK!() {

                    }
                    else {
                        led.set_value(*state).map_err(|_| {OtaErr::SetValueErr}).unwrap();
                    }
                }
                Ok(())
            }
            GpioIn::RelayOn{pin:relay} => {
                log::info!("on");
                if let Some((relay, state)) = self.io.iter_mut().nth(relay as usize) {
                    *state = ON!();
                    // led.export().map_err(|_| {OtaErr::SelectPinErr})?;
                    // led.set_direction(Direction::Out).map_err(|_| {OtaErr::SetDirectionErr})?;
                    // led.set_value(0).map_err(|_| {OtaErr::SetValueErr})?; 
                }
                Ok(())
            }

            GpioIn::RelayOff{pin:relay} => {
                log::info!("off");
                if let Some((relay, state)) = self.io.iter_mut().nth(relay as usize) {
                    *state = OFF!();
                    // led.export().map_err(|_| {OtaErr::SelectPinErr})?;
                    // led.set_direction(Direction::Out).map_err(|_| {OtaErr::SetDirectionErr})?;
                    // led.set_value(0).map_err(|_| {OtaErr::SetValueErr})?; 
                }
                Ok(())
            }

            GpioIn::FanModeLv1 => {
                log::info!("Fan mode 1");
                // for pin in &self.fan {
                //     pin.export().map_err(|_| {OtaErr::SelectPinErr})?;
                //     pin.set_direction(Direction::Out).map_err(|_| {OtaErr::SetDirectionErr})?;
                //     pin.set_value(0).map_err(|_| {OtaErr::SetValueErr})?; 
                // }
                Ok(())
            }
            
            GpioIn::FanModeLv2 => {
                log::info!("Fan mode 2");
                // for (index, pin) in self.fan.iter().enumerate() {
                //     pin.export().map_err(|_| {OtaErr::SelectPinErr})?;
                //     pin.set_direction(Direction::Out).map_err(|_| {OtaErr::SetDirectionErr})?;

                //     if index == 0 {
                //         pin.set_value(0).map_err(|_| {OtaErr::SetValueErr})?;
                //     }
                //     else {
                //         pin.set_value(1).map_err(|_| {OtaErr::SetValueErr})?; 
                    
                //     }
                
                // }
                Ok(())
            }
            
            GpioIn::FanModeLv3 => {
                log::info!("Fan mode level 3");
                // for (index, pin) in self.fan.iter().enumerate() {
                //     pin.export().map_err(|_| {OtaErr::SelectPinErr})?;
                //     pin.set_direction(Direction::Out).map_err(|_| {OtaErr::SetDirectionErr})?;
                //     if index == 1 {
                //         pin.set_value(0).map_err(|_| {OtaErr::SetValueErr})?;
                //     }
                //     else {
                //         pin.set_value(1).map_err(|_| {OtaErr::SetValueErr})?; 
                    
                //     }    
                // }
                Ok(())
            }
            
        }
    }
    pub async fn blink_parse(&mut self,pin:u64,fre_init :u16) -> Result<(),OtaErr>  {
        if let Some((_ ,_, _, _, temp)) = self.leds.iter_mut().nth(pin as usize) { 
            if *temp != fre_init as u16 {
                *temp = fre_init as u16;
                return Ok(());
            }
            else {
                *temp = fre_init as u16;
                return Err(OtaErr::RepeatErr);
            }
        }
        return Err(OtaErr::RepeatErr);
    }

    pub async fn check_temp(&mut self) -> Result<u32, OtaErr>{
        let mut file = File::open("bhien".to_string()).await.map_err(|_| {OtaErr::OpenFileErr})?;

        let mut buffer = Vec::new();

        file.read_to_end(&mut buffer).await.map_err(|_| {OtaErr::ReadFileErr})?;
        
        let content = String::from_utf8_lossy(&buffer);

        let temp = content.trim().parse::<u32>().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)).map_err(|_| {OtaErr::ConvertTempErr})?;
        Ok(temp)
    }
    
    pub async fn control_fan(&mut self, cpu_temperature:i32) {
        let diff:i32 = (cpu_temperature as i32) - (self.last_cpu_temperature as i32);
        //let mut temperature_level = TemperatureLevel::None;
        if diff != 0  {
            if cpu_temperature < 48 {
                let _ = self.send(GpioIn::FanModeLv1).await;
            } 

            else if cpu_temperature <= 52 {
                if diff < 0 {
                    let _ = self.send(GpioIn::FanModeLv1).await;

                }
                else { 
                    let _ = self.send(GpioIn::FanModeLv2).await;
                }
            } 

            else if cpu_temperature <= 58 {
                let _ = self.send(GpioIn::FanModeLv2).await;
            } 

            else if cpu_temperature <= 62 {
                if diff < 0 {
                    let _ = self.send(GpioIn::FanModeLv2).await;

                }
                else { 
                    let _ = self.send(GpioIn::FanModeLv3).await;
                }
            } 
            
            else {
                let _ = self.send(GpioIn::FanModeLv3).await;
            }
        }
        self.last_cpu_temperature = cpu_temperature as u32;
    }

    pub async fn get_value_relay(&mut self) -> Vec<bool> {
        let mut states:Vec<bool> = Vec::new();
        for (_, state) in &self.io {
            if *state == ON!() {
                states.push(true);
            }
            else {
                states.push(false);

            }
        }
        return states;
    }
}


