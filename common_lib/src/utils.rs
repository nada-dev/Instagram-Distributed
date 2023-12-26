extern crate steganography;
extern crate photon_rs;
extern crate image;
use std::fs::File;
use std::io::Write;
use std::collections::HashMap;
use std::convert::TryInto; 
use std::net::{UdpSocket, SocketAddr};
use std::sync::{Arc, Mutex};
use std::str;
use std::time::Duration;
use image::{open,ImageOutputFormat, GenericImageView, DynamicImage, ImageBuffer, Rgba};
use steganography::encoder::*;
use std::io::Cursor;
use std::io::Read;
use std::io;
use std::io::BufReader;
use std::thread;
use image::imageops::FilterType;
use std::process::Command;
use std::fs;
use std::path::Path;

const MAX_PACKET_SIZE: usize = 1000000;
const HEADER_SIZE: usize = 8; // Adjust according to your actual header size
const END_OF_TRANSMISSION: usize = usize::MAX;
const CHUNK_SIZE: usize = 65008;
const ACK_TIMEOUT: Duration = Duration::from_millis(500);


//Encoding 
pub fn encoding(id :&str)
{
//encodingchosen image 
//load the cover image 
let cover_image_path = "cover.png";
let cover_image = image::open(cover_image_path).expect("Failed to open cover image");
//Load the secret image 
let secret_image_path=format!("image{}.png",id);
let secret_image = image::open(secret_image_path).expect("Failed to open secret image");
//Embed the secret image within the cover image
let encoded_image = encodeimage(cover_image, secret_image);
//Save the encoded image to a file
let encoded_image_path = format!("encrypted_image{}.png",id);
encoded_image.save(encoded_image_path.clone()).expect("Failed to save encoded image");
}

fn encodeimage(cover_image: DynamicImage, secret_image: DynamicImage) -> DynamicImage {

    let (width, height) = cover_image.dimensions();
    let secret_image = secret_image.resize_exact(width, height, image::imageops::FilterType::Lanczos3);
   
    let mut cover_buffer = cover_image.to_rgba();
    let secret_buffer = secret_image.to_rgba();
   
    for (x, y, cover_pixel) in cover_buffer.enumerate_pixels_mut() {
    let secret_pixel = secret_buffer.get_pixel(x, y);
   
    let (r, _g, _b, _a) = (cover_pixel[0], cover_pixel[1], cover_pixel[2], cover_pixel[3]);
    let (hr, hg, hb, _ha) = (secret_pixel[0], secret_pixel[1], secret_pixel[2], secret_pixel[3]);
   
    cover_pixel[0] = (r & 0xF0) | (hr >> 4);
    cover_pixel[1] = (_g & 0xF0) | (hg >> 4);
    cover_pixel[2] = (_b & 0xF0) | (hb >> 4);
    }
   
    DynamicImage::ImageRgba8(cover_buffer)
}


// This
pub fn send_image_to_client( client_addr: &SocketAddr, image_path: &str) -> io::Result<()> {   
   let socket = UdpSocket::bind(format!("0.0.0.0:{}", client_addr.port())).expect("Failed to bind socket");
   let file = File::open(image_path).expect("This image is not Encoded");
    let mut buf_reader = BufReader::new(file);
    let mut buffer = Vec::new();
    buf_reader.read_to_end(&mut buffer)?;

    socket.set_write_timeout(Some(Duration::from_millis(100)))?;
    socket.set_read_timeout(Some(ACK_TIMEOUT))?;

    for (i, chunk) in buffer.chunks(CHUNK_SIZE).enumerate() 
    {
        let mut packet = Vec::with_capacity(HEADER_SIZE + chunk.len());
        packet.extend_from_slice(&i.to_be_bytes()); // Add sequence number as header
        packet.extend_from_slice(chunk);

        loop {
            socket.send_to(&packet, client_addr)?;
            let mut ack_buffer = [0; HEADER_SIZE];
            match socket.recv_from(&mut ack_buffer) {
                Ok(_) => {
                    let ack_seq_number = usize::from_be_bytes(ack_buffer.try_into().unwrap());
                    if ack_seq_number == i {
                            break; // Correct ACK received, proceed to next chunk
                            }
                     }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                                    // Timeout; ACK not received, resend the packet
                                    continue;
                                   }
                Err(e) => return Err(e), // Some other error
            }
        }
    }

    // Send end-of-transmission notification
    let mut eot_packet = Vec::with_capacity(HEADER_SIZE);
    eot_packet.extend_from_slice(&END_OF_TRANSMISSION.to_be_bytes());
    socket.send_to(&eot_packet, client_addr)?;

   //  let mut buffer = [0; 512];
   //  let (size, _) = socket.recv_from(&mut buffer).expect("Failed to receive message");
    println!("Done Sending");
    Ok(())
}

pub fn receive_image_to_encode(socket: &UdpSocket , src_addr: &SocketAddr)  {
    let mut file_storage: HashMap<usize, Vec<u8>> = HashMap::new();
    let mut buffer = [0u8; MAX_PACKET_SIZE];
    let mut end_of_transmission_received = false;
    let mut image_name = String::new();
    let mut first_packet = true;

    while !end_of_transmission_received {
        match socket.recv_from(&mut buffer) {
            Ok((size,_)) => {
                let sequence_number = match buffer[..HEADER_SIZE].try_into() {
                    Ok(bytes) => usize::from_be_bytes(bytes),
                    Err(e) => {
                        eprintln!("Failed to convert header bytes: {}", e);
                        continue;
                    }
                };

                // Send ACK for the received chunk
                let ack_packet = sequence_number.to_be_bytes();
                if let Err(e) = socket.send_to(&ack_packet, &src_addr) {
                    eprintln!("Failed to send ACK: {}", e);
                    continue;
                }

                if sequence_number == END_OF_TRANSMISSION {
                    end_of_transmission_received = true;
                } else if first_packet && sequence_number == usize::MAX - 1 {
                    image_name = String::from_utf8_lossy(&buffer[HEADER_SIZE..size]).to_string();
                    first_packet = false;
                } else {
                    let image_data = &buffer[HEADER_SIZE..size];
                    file_storage.insert(sequence_number, image_data.to_vec());
                }
            },
            Err(e) => {
                eprintln!("Failed to receive data: {}", e);
                continue;
            }
        };
    }

     // Reassemble the image
    let mut complete_image = Vec::new();
    for i in 0..file_storage.len() {
        if let Some(chunk) = file_storage.remove(&i) {
            complete_image.extend_from_slice(&chunk);
        }
    }
    
     // Write the complete image to a file
    // Assuming image_name contains something like "image1"
    let output_file_name = format!("output_{}", image_name);

    // Write the complete image to a file with the new name
    let mut file = File::create(&output_file_name).expect("Failed to create file");
    file.write_all(&complete_image).expect("Failed to write to file");

   println!("Image Saved");

    // Load the secret image you want to hide
    let secret_image: DynamicImage = open(&output_file_name).expect("Secret image not found");    
    let cover_image_path = "cover.png";
    let cover_image = match open(cover_image_path) {
        Ok(img) => img,
        Err(e) => {
        eprintln!("Failed to open cover image at '{}': {:?}", cover_image_path, e);
        return; // Or handle the error as appropriate
        }
    };

    // Convert the secret image to bytes
    let mut secret_image_bytes = Cursor::new(Vec::new());
    secret_image.write_to(&mut secret_image_bytes, ImageOutputFormat::PNG).expect("Failed to write secret image to bytes");
    let secret_image_bytes = secret_image_bytes.into_inner();

    // Create an Encoder instance
    let encoder = Encoder::new(&secret_image_bytes, cover_image);
    // Encode the secret into the cover image
    let encoded_image = encoder.encode_alpha(); // Adjust this according to the actual encode method signature
    // Get the dimensions of the image and save the encoded image
    let (width, height) = encoded_image.dimensions();
    let img = DynamicImage::ImageRgba8(image::RgbaImage::from_raw(width, height, encoded_image.to_vec()).unwrap());

    // Assuming image_name is a String
    let encoded_image_path = format!("encrypted_{}", image_name);
    // Save and process the image

    img.save(&encoded_image_path).expect("Failed to save the image");
    println!("done Encoding \n");

    let src= format!("{}:{}", src_addr.ip(), src_addr.port() + 1).parse::<SocketAddr>().expect("Failed to parse server address");
    
    let values :i32 = 55;
    let message = values.to_be_bytes();
    socket.send_to(&message, &src).expect("Failed to send Ready");
   // Send the encoded image to the client
   send_image_to_client(&src, &encoded_image_path).expect("Failed to send encoded image to client");
  
}

// This
pub fn receive_image (socket: &UdpSocket, out_name: &str) {
    let mut file_storage: HashMap<usize, Vec<u8>> = HashMap::new();
    let mut buffer = [0u8; CHUNK_SIZE + HEADER_SIZE];
    let mut end_of_transmission_received = false;
    let mut src_img : SocketAddr;
    while !end_of_transmission_received {
        match socket.recv_from(&mut buffer) {
            Ok((size, src_img)) => {
            let sequence_number = match buffer[..HEADER_SIZE].try_into() {
                Ok(bytes) => usize::from_be_bytes(bytes),
                Err(e) => {
                    eprintln!("Failed to convert header bytes: {}", e);
                    continue;
                }
            };
            
            if sequence_number == END_OF_TRANSMISSION {
                end_of_transmission_received = true;
            } else {
                let image_data = &buffer[HEADER_SIZE..size];
                file_storage.insert(sequence_number, image_data.to_vec());
            }
            
            // Send ACK for the received chunk
            let ack_packet = sequence_number.to_be_bytes();
            if let Err(e) = socket.send_to(&ack_packet,&src_img) {
                eprintln!("Failed to send ACK: {}", e);
                continue;
            }
        },
        Err(e) => {
            eprintln!("Failed to receive data: {}", e);
            continue; // Continue the loop even if there's an error
        }
    };
}

    println!("Done receiving");

    // Reassemble and save the image as before
    let mut complete_image = Vec::new();
    for i in 0..file_storage.len() {
        if let Some(chunk) = file_storage.remove(&i) {
            complete_image.extend_from_slice(&chunk);
        }
    }

    let mut file = File::create(out_name).unwrap();
    file.write_all(&complete_image).unwrap();
    println!("Image completed!!!!!!!!!!!!!");
}

pub fn failure_token_handle(flag: Arc<Mutex<bool>>, next_token_add: &str){
    let token_port = 3333;
    let next_server: SocketAddr = format!("{}:{}", next_token_add, token_port).parse().expect("Failed to parse server address");
    let token_socket = UdpSocket::bind(format!("0.0.0.0:{}", token_port)).expect("Failed to bind socket");
    let msg = "ball";
    
    loop{
        let mut buffer = [0; 512];
        let (size, _) = token_socket.recv_from(&mut buffer).expect("Failed to receive message");
        let _ = str::from_utf8(&buffer[..size]).unwrap().trim().to_string();
        let mut token: std::sync::MutexGuard<'_, bool> = flag.lock().unwrap();
        *token = true;
        drop(token);
        //println!("I have the token now :( Yalaaaaahwy");
        thread::sleep(Duration::from_secs(2 as u64));
        token = flag.lock().unwrap();
        *token = false;
        drop(token);
        //println!("Released token now :)");
        token_socket.send_to(msg.as_bytes(), next_server).expect("Failed to send message");
    }
}

pub fn working_token_handle(off_server: Arc<Mutex<i32>>, work_flag: Arc<Mutex<bool>>, next_add: i32, next_next_add: i32, servers: [&str; 3]){
    let token_port = 6666;
    let token_socket = UdpSocket::bind(format!("0.0.0.0:{}", token_port)).expect("Failed to bind socket");
    let msg = "ball";

    loop{
        let mut buffer = [0; 512];
        let (size, _) = token_socket.recv_from(&mut buffer).expect("Failed to receive message");
        let _ = str::from_utf8(&buffer[..size]).unwrap().trim().to_string();
        let mut work_token = work_flag.lock().unwrap();
        *work_token = true;
        drop(work_token);
        thread::sleep(Duration::from_millis(500 as u64));
        work_token = work_flag.lock().unwrap();
        *work_token = false;
        drop(work_token);
        token_socket.send_to(msg.as_bytes(), format!("{}:{}", servers[next_add as usize], token_port)).expect("Failed to send message");
    }
}

pub fn send_offline (off_server: &str, online_servers: [&str; 2]){
    let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind socket");
    // send to all servers my current status (offline/online)
    for server in online_servers {
        let server_add: SocketAddr = server.parse().expect("Failed to parse server address");
        socket.send_to(off_server.as_bytes(), server_add).unwrap();
        thread::sleep(Duration::from_millis(5));
    }
}

pub fn who_offline_handler(off_server: Arc<Mutex<i32>>) {
    let offline_port = 2222;
    let offline_add = format!("0.0.0.0:{}", offline_port);
    let socket = UdpSocket::bind(offline_add).expect("Failed to bind socket");
    // listen for messages from other clients and print them out
    loop{
        let mut buffer = [0; 4];
        socket.recv_from(&mut buffer).expect("Failed to receive message");
        let mut off_server_id = off_server.lock().unwrap();
        *off_server_id = i32::from_be_bytes(buffer);
    }
}

pub fn handle_requests_v2(
    socket: &UdpSocket,
    online_servers: [&str; 2],
    work_flg: Arc<Mutex<bool>>,
    offline_flg: Arc<Mutex<bool>>,
    id: i32,
    servers: [&str; 3])
{
    let mut buffer = [0; 512];
   
    let mut message = String::from("");
    loop {
        let (size, src_addr) = socket.recv_from(&mut buffer).expect("Failed to receive message");
        message = str::from_utf8(&buffer[..size]).unwrap().trim().to_string();
        if message == "Request" {
            let work_token = work_flg.lock().unwrap();
            let offline_token = offline_flg.lock().unwrap();
            if *work_token == true {
                drop(work_token);
                // send back to the client
                let ack_message =(id).to_be_bytes();
                let _ = socket.send_to(&ack_message, &src_addr);
                receive_image_to_encode(&socket , &src_addr );
            }
            else {
                continue;
            }
        }
    }
}

// from client to server
pub fn register_client_in_dos(ip: &str, servers: [&str; 3],records: Arc<Mutex<HashMap<(SocketAddr, String), i32>>>){
    let dos_port = 7777;
    let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind dos socket");
    for server in servers {
        let server_add: SocketAddr = format!("{}:{}", server, dos_port).parse().expect("Failed to parse server address");
        socket.send_to(ip.as_bytes(), server_add).unwrap();
        thread::sleep(Duration::from_millis(5));
    }

    let mut buffer = [0; 512];
        let (size, _) = socket.recv_from(&mut buffer).expect("Failed to receive message");
        let result = str::from_utf8(&buffer[..size]).unwrap().trim().to_string();
        if result=="YES"
        {
            let (size, _) = socket.recv_from(&mut buffer).expect("Failed to receive message");
            let re_in_off=str::from_utf8(&buffer[..size]).unwrap().trim().to_string();
            let src_ip_views: Vec<String> = re_in_off.split(':')
                                  .map(|s| s.to_string())
                                  .collect();

            let id_views: Vec<String> = src_ip_views[1].split('_').map(|s| s.to_string()).collect();
            let mut records_guard = records.lock().unwrap();
            let new_addr: SocketAddr = format!("{}:9999", src_ip_views[0] ).parse().expect("Failed to parse sender address");
            records_guard.insert((new_addr, id_views[0].clone()), id_views[1].parse::<i32>().unwrap());
        }

}

// client asks server who's online:
pub fn whos_online(servers: [&str; 3]) -> Vec<String> {
    let port = 8888;
    let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind socket");
    let msg = "Who online";
    for server in servers {
        let server_add: SocketAddr = format!("{}:{}", server, port).parse().expect("Failed to parse server address");
        socket.send_to(msg.as_bytes(), server_add).unwrap();
        thread::sleep(Duration::from_millis(5));
    }

    let mut buffer = [0; 512];    
    let (size, _) = socket.recv_from(&mut buffer).expect("Failed to receive message");
    let ips = str::from_utf8(&buffer[..size]).unwrap().trim().to_string();
    let mut ips_vec: Vec<String> = ips.split('_')
                                  .map(|s| s.to_string())
                                  .collect();
    ips_vec.pop();
    return ips_vec;
}

pub fn store_in_dos(dos_map: Arc<Mutex<HashMap<String, bool>>>, off_map: Arc<Mutex<HashMap<String, String>>>) {
    let dos_port = 7777;
    let socket = UdpSocket::bind(format!("0.0.0.0:{}", dos_port)).expect("Failed to bind dos socket");
    
    loop {
        let mut buffer = [0; 512];
        let (size, src_addr) = socket.recv_from(&mut buffer).expect("Did not correctly receive data");
        let message = str::from_utf8(&buffer[..size]).unwrap().trim().to_string();

        let mut dos_map = dos_map.lock().unwrap();
        if message.starts_with("OFFLINE:") {
            let offline_client_ip = message.replace("OFFLINE:", "");
            dos_map.insert(offline_client_ip, false); // Set client status to offline
        } else {
            dos_map.insert(message.clone(), true); // Store new IP or update existing client's online status

            //check if in map , if yes update client views 
            let mut off_map = off_map.lock().unwrap();
            let key_exists = off_map.contains_key(&message.clone());
            if key_exists
            {
                socket.send_to("YES".as_bytes(),src_addr);
                thread::sleep(Duration::from_millis(5));
                let id_views=off_map.get(&message.clone()).cloned().unwrap();
                socket.send_to(id_views.as_bytes(),src_addr);
                thread::sleep(Duration::from_millis(5));
            }
            else {
                socket.send_to("NO".as_bytes(),src_addr);
            }

        }
    }
}

pub fn store_in_dos_off_for_views(dos_map: Arc<Mutex<HashMap<String, bool>>>,off_map: Arc<Mutex<HashMap<String, String>>>) {
    let dos_port = 7778;
    let socket = UdpSocket::bind(format!("0.0.0.0:{}", dos_port)).expect("Failed to bind dos socket");
    
    loop {
        let mut buffer = [0; 512];
        let (size, _addr) = socket.recv_from(&mut buffer).expect("Did not correctly receive data");
        let message = str::from_utf8(&buffer[..size]).unwrap().trim().to_string();

        let mut dos_map = dos_map.lock().unwrap();
        let offline_client_ip = message.replace("OFFLINE:", "");//ClientIP_MyIP_Id_views
        let cip_ip_id_view: Vec<String> = offline_client_ip.split('_').map(|s| s.to_string()).collect();
        dos_map.insert(cip_ip_id_view[0].clone(), false); // Set client status to offline
       
        let ip_id_view=format!("{}:{}_{}",cip_ip_id_view[1],cip_ip_id_view[2],cip_ip_id_view[3]);
        let mut off_map = off_map.lock().unwrap();
        off_map.insert(cip_ip_id_view[0].clone(),ip_id_view);
    }
}

// server respond who's online
pub fn send_who_online(dos_map: Arc<Mutex<HashMap<String, bool>>>){
    let dos_port: i32 = 8888;
    let socket = UdpSocket::bind(format!("0.0.0.0:{}", dos_port)).expect("Failed to bind dos socket");
    loop {
        let mut buffer = [0; 512];
        let (size,addr) = socket.recv_from(&mut buffer).expect("Did not correctly receive data");
        let msg = str::from_utf8(&buffer[..size]).unwrap().trim().to_string();
        if msg == "Who online".to_string() {
            let sender_addr: Vec<String>  = addr.to_string().split(':').map(|s| s.to_string()).collect();
            let sender_ip = sender_addr.get(0);
            let mut conc_ips = String::new();
            let mut dos_map = dos_map.lock().unwrap();        
 
            // loop over the map and concatinate ips 
            for (key, value) in dos_map.iter_mut(){
                // let def : &String;
                if let Some(ip) = sender_ip {
                    if ip != key {
                        if *value == true {
                            conc_ips.push_str(key);
                            conc_ips.push_str("_");
                        }
                    }
                }
            }
            socket.send_to(conc_ips.as_bytes(), addr).expect("Failed to send IPs");
        }
    }
}

// This
pub fn send_message(socket: &UdpSocket, target_address: &SocketAddr,msg: &str) {
    let message = format!("{}",msg);
    socket.send_to(message.as_bytes(), target_address).expect("Failed to send message");
}

pub fn convert_to_low_resolution(image_name: &str, new_width: u32, new_height: u32) {
    println!("converting {}", image_name);
    // Load the image
    let img = image::open(image_name).unwrap();
    // Resize the image to a lower resolution
    let resized = img.resize_exact(new_width, new_height, FilterType::Nearest);
    let output_file_name = format!("low_res_{}", image_name);
    resized.save(output_file_name).unwrap();
}

pub fn notify_server_client_offline(client_ip: String, servers: [&str; 3]) -> io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    let message = format!("OFFLINE:{}", client_ip);
    for server in servers.iter() {
        let server_addr: SocketAddr = format!("{}:{}", server, 7777).parse().expect("Failed to parse server address");
        socket.send_to(message.as_bytes(), server_addr).expect("Failed to parse server address");
    }
    Ok(())
}

pub fn notify_server_client_offline_for_views(client_ip: String, servers: [&str; 3],mess: String) -> io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    let message = format!("OFFLINE:{}_{}", client_ip,mess); //OFFLINE:HisIP_MyIP_Id_views 
    for server in servers.iter() {
        let server_addr: SocketAddr = format!("{}:{}", server, 7778).parse().expect("Failed to parse server address");
        socket.send_to(message.as_bytes(), server_addr).expect("Failed to parse server address"); 
    }
    Ok(())
}


pub fn send_my_img () {
    let imgs_port: i32 = 9999;
    let mut buffer = [0; 512];
    println!("Waiting for requests");
    
    let socket = UdpSocket::bind(format!("0.0.0.0:{}", imgs_port)).expect("Failed to bind dos socket");
    
    loop {
        let (size, src_addr) = socket.recv_from(&mut buffer).expect("Failed to receive message");
        let view_msg = str::from_utf8(&buffer[..size]).unwrap().trim().to_string();
        
        if view_msg == "view all".to_string() {
            
            send_message(&socket, &src_addr, "ACK");
            // send dynamic port to handle comminication
            for i in 0..2 {
                let image_name = format!("low_res_image{}.png", i);
                let _ = send_image_to_client(&src_addr, &image_name);
                thread::sleep(Duration::from_millis(200));
                }
            // waiting for other client to choose image:
            let (size, src_addr) = socket.recv_from(&mut buffer).expect("Failed to receive message");
            let id: String = str::from_utf8(&buffer[..size]).unwrap().trim().to_string();
            
            let encoded_image_path = format!("encrypted_image{}.png", id);
            //  encoded_image.save(encoded_image_path.clone()).expect("Failed to save encoded image");

            let views = "3";
            socket.send_to(&views.as_bytes(), src_addr).expect("Failed to send image views");
            let _ = send_image_to_client(&src_addr, &encoded_image_path);
        }
    }
}

pub fn request_img_from_client (friend_addr: SocketAddr, servers: [&str; 3],records: Arc<Mutex<HashMap<(SocketAddr, String), i32>>>) {
    let socket = UdpSocket::bind(format!("0.0.0.0:0")).expect("Failed to bind dos socket");
    let msg = "view all";
    send_message(&socket, &friend_addr, msg);
    let ip_friend = friend_addr.ip().to_string();      

            //***************Handling Offline CLinets****************
     
    socket.set_read_timeout(Some(Duration::from_secs(5))) // Timeout of 30 seconds
    .expect("Failed to set read timeout");

    let mut ack_buffer = [0; 512];
      
    // Wait for ACK from Client A
    match socket.recv_from(&mut ack_buffer) {
        Ok((size, _)) => {
            let ack_message = std::str::from_utf8(&ack_buffer[..size]).unwrap();
            if ack_message == "ACK" {
                println!("Client is online"); 
            } else {
                println!("unexpected response");
                // Handle unexpected response
            }
        },
        Err(e) => {
            if e.kind() == io::ErrorKind::WouldBlock || e.kind() == io::ErrorKind::TimedOut {
                println!("Client offline");
                //update the DOS of the server
                if let Err(err) = notify_server_client_offline(ip_friend, servers) {
                    println!("Updating DOS: {}", err);
                    return;
                }
                else {
                    println!("An error occurred while waiting for ACK: {}", e);
                    return;
                }
            }
        }
    }

    for i in 0..2 {
        let img_name = format!("image{}.png", i);
        receive_image (&socket, &format!("offered_{}", img_name));
        Command::new("xdg-open")
            .arg(format!("offered_{}", img_name))
            .spawn()
            .expect("failed to open image");
    }
    println!("Enter the number of the image you would like to gain access to \n");
    let mut id = "0"; //ignored value
    let mut im_id = String::new();
                // Read the line from standard input 
                match io::stdin().read_line(&mut im_id) {
                    Ok(_) => 
                    { 
                        id = im_id.trim();
                    },
                    Err(e) => {
                        println!("Error: {}", e);
                    }
                }
    let _ = socket.send_to(&id.as_bytes(), friend_addr);
    println!("SENT ID");
    let mut buffer = [0; 512]; 
    let (size, friend_addr) = socket.recv_from(&mut buffer).expect("Failed to receive message");
    let view_msg = str::from_utf8(&buffer[..size]).unwrap().trim().to_string();
    
    let img_name = "encrypted_image.png";
    receive_image(&socket, &img_name);
    
    // Decoding
    let secret_image = image::open(img_name).expect("Failed to open embedded image");
    let cover_image_path = "cover.png"; 
    let cover_image = image::open(cover_image_path).expect("Failed to open cover image");
    // Extract the hidden image from the embedded image
   // Save the extracted image to a file
     let extracted_image = decodeimage(secret_image, cover_image);

    let extracted_image_path = format!("decrypted_{}_{}.png",id, ip_friend);
    extracted_image.save(extracted_image_path.clone()).expect("Failed to save extracted image");
    
    let mut records_guard = records.lock().unwrap();
    records_guard.insert((friend_addr, id.to_string()), view_msg.parse::<i32>().unwrap());

    let records_clone = Arc::clone(&records);
    drop(records_guard);
    let (_, view) = check_num_of_views(friend_addr, id.to_string(), records_clone,extracted_image_path.clone());

    thread::sleep(Duration::from_millis(500));
}


pub fn decodeimage(secret_image: DynamicImage, cover_image: DynamicImage) -> DynamicImage 
{
    let secret_image_buffer = secret_image.to_rgba();
    let mut extracted_buffer = ImageBuffer::<Rgba<u8>, _>::new(secret_image.width(), secret_image.height());

    for (x, y, pixel) in secret_image_buffer.enumerate_pixels() {
        let cover_pixel = cover_image.get_pixel(x, y);

        let (r, g, b, a) = (cover_pixel[0], cover_pixel[1], cover_pixel[2], cover_pixel[3]);
        let (hr, hg, hb, ha) = ((pixel[0] & 0xF) << 4, (pixel[1] & 0xF) << 4, (pixel[2] & 0xF) << 4, 255);

        let rgba_pixel = Rgba([hr, hg, hb, ha]);
        extracted_buffer.put_pixel(x, y, rgba_pixel);
    }
    DynamicImage::ImageRgba8(extracted_buffer)
}

pub fn check_num_of_views(   
    ip: SocketAddr, 
    id: String, 
    records: Arc<Mutex<HashMap<(SocketAddr, String), i32>>>,
    extracted_image_path: String
) -> (bool, i32) 
{
    println!("inside check");
    let mut records = records.lock().unwrap();
    let key_exists = records.contains_key(&(ip