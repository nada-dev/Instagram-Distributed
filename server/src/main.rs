use common_lib::utils;
use std::net::{SocketAddr, UdpSocket};
use std::str;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::collections::HashMap;


static ONLINE_SERVERS: [&str; 2] = [
	"10.40.54.125:2222", 
	"10.40.43.43:2222"
];

static SERVERS: [&str; 3] = [
	"10.40.54.125", 
	"10.40.43.43",
	"10.40.48.77", 
];


fn main() {
	let requests_port = 4444;
	let next_server = 2;
	let next_next_server = 3;

    let mut handles = vec![];

    let off_flag = Arc::new(Mutex::new(false));
    let off_flag_clone = Arc::clone(&off_flag);
    thread::spawn(move || {
        utils::failure_token_handle_sender(off_flag_clone, SERVERS[next_server as usize])
    });

    let off_server = Arc::new(Mutex::new(0));
    let off_server_clone = Arc::clone(&off_server);
    // launch a  thread for offline handler
    thread::spawn(move || utils::who_offline_handler(off_server_clone));

    let off_server_clone = Arc::clone(&off_server);
    let work_flag = Arc::new(Mutex::new(false));
    let work_flag_clone = Arc::clone(&work_flag);
    thread::spawn(move || {
        utils::working_token_handle(
            off_server_clone,
            work_flag_clone,
            next_server as i32,
            next_next_server,
            SERVERS,
        )
    });
    
    let dos_map: HashMap<String, bool> = HashMap::new();
    let dos_map = Arc::new(Mutex::new(dos_map));
    let dos_map_clone = Arc::clone(&dos_map);

    let off_map: HashMap<String, String> = HashMap::new();
    let off_map = Arc::new(Mutex::new(off_map));
    let off_map_clone = Arc::clone(&off_map);


    let handle1 = thread::spawn(move || {
        utils::store_in_dos(Arc::clone(&dos_map_clone), Arc::clone(&off_map_clone))
    });
    handles.push(handle1);

    let off_map_clone = Arc::clone(&off_map);
    let dos_map_clone = Arc::clone(&dos_map);
    let handle2 = thread::spawn(move || {
        utils::store_in_dos_off_for_views(Arc::clone(&dos_map_clone), Arc::clone(&off_map_clone))
    });
    handles.push(handle2);

    let handle3 = thread::spawn(move || {
        utils::send_who_online(Arc::clone(&dos_map))
    });
    handles.push(handle3);

    println!("Listening for requests on port {}", requests_port);
    loop {
        let socket =
            UdpSocket::bind(format!("0.0.0.0:{}", requests_port)).expect("Failed to bind socket");
        // println!("socket = {:?}", socket);
        utils::handle_requests_v2(
            &socket,
            ONLINE_SERVERS,
            Arc::clone(&work_flag),
            Arc::clone(&off_flag),
            ID,
            SERVERS,
        );
    }
    for handle in handles {
        handle.join().unwrap();
    }
}
