use clap::Parser;
use system_intergration::SystemIntergration;
pub mod system_intergration;
pub mod logic;
pub mod transport;
pub mod error;
pub mod gpio;
pub mod json;

/*
RUST_LOG=info ./io-service \
--button=14 \
--time-blink=1000 \
--leds=10,11,12,13
*/

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    device: String,

    #[clap(short, long,default_value="0")]
    leds: String,

    #[clap(short, long,default_value="0")]
    ios: String,

    #[clap(short, long,default_value="0")]
    fans: String,

    #[clap(short, long,default_value="0")]
    time_blink: u64,

    #[clap(short, long,default_value="0")]
    button: u64,
}

async fn cup_comma(input:String) -> Vec<u64> {

    let mut numbers: Vec<u64> = Vec::new();

    let parts: Vec<&str> = input.split(',').collect();

    // convert u64 Vec
    for part in parts {
        if let Ok(number) = part.parse::<u64>() {
            numbers.push(number);
        }
    }
    numbers
}

#[tokio::main]
async fn main() {
    env_logger::builder().format_timestamp_millis().init();

    let args = Args::parse();
    log::info!("args: {:?}", args);

    let time_blink = args.time_blink;
    let button = args.button;

    let leds_string = args.leds;
    let ios_string = args.ios;
    let fans_string = args.fans;
    let device = args.device;


    let leds = cup_comma(leds_string).await;
    let ios = cup_comma(ios_string).await;
    let fans= cup_comma(fans_string).await;

    log::info!("vec leds: {:?}", leds);
    log::info!("vec ios: {:?}",ios);
    log::info!("vec fans: {:?}",fans);
    let id_mac = "Mi8ea43769e4d6Qb".to_string();
    // Use numbers as needed in your application logic
    let mut system_intergration = SystemIntergration::new(device, id_mac , leds, ios, fans, time_blink, button).await;
    loop {
        match system_intergration.recv().await {
            Ok(_) => {
                
            },
            Err(e) => {
                log::error!("{:?}", e);
                break;
            }
        }

    }
}


