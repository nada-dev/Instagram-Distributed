use common_lib::utils;
use std::{thread,io};
use std::fmt::format;
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

static SERVERS: [&str; 3] = 
[
    "10.40.48.77", 
    "10.40.33.56",
    "10.40.48.14", 
];
//
fn main() {

    let ip = "10.40.34.156"; //my ip 
    let records = Arc::new(Mutex::new(HashMap::<(SocketAddr, String), i32>::new()));

    utils::register_client_in_dos(&ip,SERVERS, records); //online in dos

    // low resolution myimages
    for i in 0..2 {
         let image_name = format!("image{}.png", i);
         utils::convert_to_low_resolution(&image_name, 50, 50);
     }

    //thread to listen to incoming requests 
    let mut handles = vec![];
    let handle1 = thread::spawn(move || {
        utils::send_my_img()
    });
    handles.push(handle1);
    loop{
    //option to encrypt or chat with friend 

        println!("HELLO ! What do you plan to do today ? \n 1- Encrypt some AMAZING images ? \n 2- Share With Friends?\n 3- View existing images");
        let mut input_text = String::new();
        // Read the line from standard input (the terminal)
        match io::stdin().read_line(&mut input_text) {
            Ok(_) => {
            let input_text=input_text.trim();

                if input_text=="1" //encryption 
                {
                    //add encrypt code 
                    println!("You typed: {}", input_text.trim());
                    //add encrypt code 
                    println!("Which image do you want to encode");
                    let mut img_id = String::new();
                    // Read the line from standard input (the terminal)
                    match io::stdin().read_line(&mut img_id) {
            
                        Ok(_) => 
                        {
                            let id=img_id.trim();
                            utils::encoding(&id);
                        }
                        Err(e) => 
                        {
                            println!("Error: {}", e);
                        }
                    }

                }

                else if input_text=="2" //open DOS
                {
                    let mut n =1;
                    let online_friends= utils::whos_online(SERVERS); //list of online friends from dos 
                    for ip in &online_friends
                    {
                        let friendIP=println!("Friend {} IP is {}",n,ip);
                        n=n+1;
                    }

                    println!("what is the number of the friend you would like to see their images ?");

                    let mut friend_num = String::new();
                    // Read the line from standard input (the terminal)
                    match io::stdin().read_line(&mut friend_num) {
                        Ok(_) => 
                        {
                            let friend_num=friend_num.trim();
                            match friend_num.parse::<i32>() {
                                Ok(n) => { //friend is chosen 
                                    let ip_friend=&online_friends[(n-1)as usize];//ippppppppppppppppppppp
                                    let friend_addr: SocketAddr = format!("{}:9999",ip_friend).parse().expect("Failed to parse server address");

                                    let records_clone = Arc::clone(&records);
                                    let handle1 = thread::spawn(move ||  //thread to start comunicating with friend 
                                    {
                                        utils::request_img_from_client(friend_addr, SERVERS,records_clone);
                                    }).join().unwrap();

                                },
                                Err(e) => println!("Failed to parse friend IP: {}", e),
                            }

                        }
                        Err(e) => {
                            println!("Error: {}", e);
                        }
                    }




                }
                else if input_text == "3"{
                    let records_lock = records.lock().unwrap();
                    for ((socket_addr, string), value) in records_lock.iter() {
                        println!("SocketAddr: {}, Image ID: {}, Number of available views: {}", socket_addr, string, value);
                    }

                    println!("Please choose the image you want in this format: ip_id:");
                    let mut chosen = String::new();
                    // Read the line from standard input (the terminal)
                    match io::stdin().read_line(&mut chosen) {
                        Ok(_) => {
                            let chosen=chosen.trim();
                            let mut ip_id: Vec<String> = chosen.split('_').map(|s| s.to_string()).collect();
                            let ip_addr = format!("{}:9999", &ip_id[0]).parse().expect("Failed to parse ip from map");
                            utils::check_num_of_views(ip_addr, ip_id[1], records_lock, format!("decrypted_{}_{}.png", ip_id[1], &ip_id[0]));
                        }
                        Err(_) => todo!(),
                    }
                }
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
    for handle in handles {
        handle.join().unwrap();
    }
}