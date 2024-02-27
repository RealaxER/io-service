extern crate serde_json;
use serde_json::{Value, json};
use rand::Rng;

pub enum JsonIn {
    StatusConvert{json_init: Value , pin:Vec<(bool,String)>},
    SyncConvert{status: Vec<bool>, mac_id: String},
    KeepAlive,
}


pub struct JsonDriver {

}

impl JsonDriver {
    pub async fn get_reqid(&mut self) -> String {
        let random_string: String = rand::thread_rng()
        .sample_iter(rand::distributions::Alphanumeric)
        .take(12)
        .map(char::from)
        .collect();

        random_string.to_string()
    }
    pub async fn convert(&mut self , type_res:JsonIn) -> (String, String) {
        match type_res {
            JsonIn::StatusConvert{json_init,pin} => {
                let json_str = r#"{
                    "cmd": "status",
                    "control_source": {
                        "id": "" ,
                        "previous_control_reqid": "",
                        "type": "app"
                    },
                    "objects": [{
                        "bridge_key": "io",
                        "data": [],
                        "type": "devices"
                    }],
                    "reqid": "",
                    "source": "io"
                }"#;
                        
               // get control source  
                let mut json_status: Value = serde_json::from_str(json_str).unwrap();
                let control_source = json_init["control_source"].clone();    
                json_status["control_source"] = control_source;
        
                // get reqid 
                let reqid = json_init["reqid"].clone();
                json_status["reqid"] = reqid;
                
                // get data 
                let objects = json_status["objects"].as_array_mut().unwrap();

                // Truy cập vào mảng "data" của phần tử đầu tiên trong "objects"
                let data = objects[0]["data"].as_array_mut().unwrap();

                for (status , hash) in &pin {
                    data.push(json!({
                        "hash": hash.to_string(),
                        "states": {
                            "OnOff": {"on": status }
                        }
                    }));
                }

                (json_status.to_string(),"".to_string())
            }
            JsonIn::SyncConvert{status, mac_id} => {

                let json_str = r#"
                {
                    "cmd": "sync",
                    "objects": [
                        {
                            "bridge_key": "io",
                            "data":[
                            ],
                            "type": "devices_local"
                        }
                    ],
                    "reqid": "",
                    "source": "io"
                }"#;
                let mut json_config: Value = serde_json::from_str(json_str).unwrap();

                let mut json_status = json!({
                    "cmd": "status",
                    "objects": [
                        {
                            "bridge_key": "io",
                            "data": [
                            ],
                            "type": "devices"
                        }
                    ],
                    "reqid": "4YmPnprQ2DWR1D9",
                    "source": "io"
                });

                // get data 
                let objects_cf = json_config["objects"].as_array_mut().unwrap();

                // Truy cập vào mảng "data" của phần tử đầu tiên trong "objects"
                let data_cf = objects_cf[0]["data"].as_array_mut().unwrap();

                // get data 
                let objects_st = json_status["objects"].as_array_mut().unwrap();

                // Truy cập vào mảng "data" của phần tử đầu tiên trong "objects"
                let data_st = objects_st[0]["data"].as_array_mut().unwrap();

                for (index, value) in status.iter().enumerate() {
                    let hash = format!("io-{}-{}", mac_id, index); 

                    data_cf.push(json!({
                        "bridge_key": "io",
                        "hash": hash.to_string(),
                        "isDefault": true,
                        "mac": mac_id.to_string(),
                        "macdev": mac_id.to_string(),
                        "traits": [
                            {
                                "is_main": *value,
                                "name": "OnOff"
                            }
                        ],
                        "type": "SWITCH"
                    }));

                    data_st.push(json!({
                        "hash": hash.to_string(),
                        "states": {
                            "OnOff": {
                                "on":*value
                            }
                        }
                    }));

                }
                json_status["reqid"] = json!(self.get_reqid().await);
                json_config["reqid"] = json!(self.get_reqid().await);
                

                return (json_config.to_string() , json_status.to_string());
                
            }
            JsonIn::KeepAlive => {
                let mut json_ka = json!({
                    "cmd": "status",
                    "objects": [
                        {
                            "bridge_key": "io",
                            "data": [],
                            "type": "keepalive"
                        }
                    ],
                    "reqid": "",
                    "source": "io"
                });

                json_ka["reqid"] = json!(self.get_reqid().await);

                return (json_ka.to_string(), "".to_string());
            }
        }
    }
}