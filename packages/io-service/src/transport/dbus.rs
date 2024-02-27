// use dbus_tokio::connection::{self, IOResource};
// use dbus::{blocking::SyncConnection, nonblock};
// use std::{sync::Arc, time::Duration};
// use dbus::message::MatchRule;
// use super::TransportOut;
// use tokio::sync::mpsc;
// use crate::error::OtaErr;


// pub struct DbusDriver {
//     pub tx: mpsc::Sender<Result<TransportOut, OtaErr>>,
//     pub rx: mpsc::Receiver<Result<TransportOut, OtaErr>>,
//     pub conn: Arc<dbus::nonblock::SyncConnection>,
// }


// impl DbusDriver {
//     pub async fn new() -> DbusDriver {
//         let (tx, rx) = mpsc::channel::<Result<TransportOut, OtaErr>>(2);
//         let (resource, conn) = connection::new_session_sync().unwrap();
//         tokio::spawn(async move {
//             let err = resource.await;
//             panic!("Lost connection to D-Bus: {}", err);
//         });
//         DbusDriver {
//             tx: tx,
//             rx: rx,
//             conn: conn,
//         }
//     }
//     pub async fn send(&mut self)-> Result<(),OtaErr> {        
//         // Create interval - a Stream that will fire an event periodically
//         let mut interval = tokio::time::interval(Duration::from_secs(2));
    
//         // Create a future calling D-Bus method each time the interval generates a tick
//         let conn2 = self.conn.clone();
//         let calls = async move {
//                 interval.tick().await;
//                 let conn = conn2.clone();
    
//                 log::info!("Calling Hello...");
//                 let proxy = nonblock::Proxy::new("com.example.dbustest", "/hello", Duration::from_secs(2), conn);
//                 // TODO: Handle timeouts and errors here
//                 let (x,): (String,) = proxy.method_call("com.example.dbustest", "Hello", ("Tokio async/await",)).await.unwrap();
//                 log::info!("{}", x);
//         };
    
//         // To receive D-Bus signals we need to add a match that defines which signals should be forwarded
//         // to our application.
//         let mr = MatchRule::new_signal("com.example.dbustest", "HelloHappened");
//         let incoming_signal = self.conn.add_match(mr).await.unwrap().cb(|_, (source,): (String,)| {
//             log::info!("Hello from {} happened on the bus!", source);
//             true
//         });
    
//         // This will never return (except on panic) as there's no exit condition in the calls loop
//         calls.await;
    
//         // Needed here to ensure the "incoming_signal" object is not dropped too early
//         self.conn.remove_match(incoming_signal.token()).await.unwrap();

//         Ok(())
//     }

//     pub async fn recv(&mut self) -> Result<TransportOut, OtaErr> {
//         return self.rx.recv().await.unwrap();
//     }
// }