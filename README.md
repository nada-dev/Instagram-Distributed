# A Cloud P2P environment for  controlled sharing of images 
Link to repo: https://github.com/nada-dev/Instagram-Distributed-.git


## Project Description
This project is a Peer-to-Peer Instagram-like application based on a distributed cloud for
encryption in Rust. It is developed based on a literature surveying of various algorithms, class
discussions, and implementation of distributed systems principles such as distributed election,
multithreading, interprocess communication, etc. 

# User Manual
## 1- Needed Libraries
  All these libraries are added under dependencies in the cargo.toml found in the
  common lib file.
  1. queues = "1.0.2"
  2. steganography = "1.0.2"
  3. image = "0.21.0"
  4. photon-rs = "0.3.2"
  5. tokio = { version = "1", features = ["full"] }
  6. serde = { version = "1.0", features = ["derive"] }
  7. serde_json = "1.0"
## 2- Compilation Procedures
Before compilation, the IPs in the clients and servers need to be updated.
Also, we need to confirm that each server has a unique ID. Then we simply open
the terminal, navigate to the src file of the code, and type “cargo run ”.


## The repository structure:
  The project is of the common lib file which contains all the main functions and
  dependencies used by both client and server. Therefore it should be in the same
  folder as the server/client project folder. In addition, the program is made up of 3
  files/cargo packets:
  1. P2P: this file contains the client code, the images that will be shared, and
  the cover image used in encryption. It is the same code that works with all
  clients
  2. Server: this file contains the server’s code. It works on all servers except
  one.
  3. Server with token: this code is identical to the server code, but the server
  contains the token once the program starts. Thus it runs only on 1 server.

