#![feature(async_closure)]

mod states;
mod command_handlers;

use std::{net::{SocketAddrV4, SocketAddr}, sync::Arc, collections::{HashMap, HashSet}};
use command_handlers::{handle_touch, look};
use lazy_static::lazy_static;
use states::{ServerState, ClientState, ClientPointer, Room, RoomAddr, to_arc_mutex, GameObject, GameAction};
use tokio::{net::{TcpListener, TcpStream}, io::{BufReader, AsyncBufReadExt, AsyncWriteExt}, sync::Mutex};

macro_rules! escaped {
    ($exp:expr) => {
        format!("{}\n\r", $exp).as_bytes()
    }
}

async fn process_builder_command(_input: String, _addr: SocketAddr, _server_state: Arc<ServerState>, _my_client: ClientPointer) -> String {

    match &_input[.._input.find(" ").unwrap_or(_input.len())] {
        "\\add" => {
            //TODO add object to room
        }
        _ => {}
    }

    "Nice builder command.".into()
}

async fn process_client_command(input: String, addr: SocketAddr, server_state: Arc<ServerState>, my_client: ClientPointer) -> String {
    //Manage server game state here

    if input.starts_with("\\") {
        return process_builder_command(input, addr, server_state, my_client).await;
    }

    let temp_usize = input.find(" ").unwrap_or(input.len());
    println!("{}", temp_usize);
    match &input[..temp_usize] {
        "touch" => {
            return handle_touch(&input, server_state, my_client).await;
        },
        "look" => {
            return look(&input, server_state, my_client).await;
        }
        _ => {}
    }

    return String::from("The room is quiet.");
}

async fn process(mut _socket: TcpStream, addr: SocketAddr, server_state: Arc<ServerState>) {
    let (read, mut write) = _socket.split();
    let mut reader = BufReader::new(read);
    let client_state = ClientState::new(addr).to_pointer();
    server_state.client_states.lock().await.push(
        client_state.clone()
    );
    //write.write(escaped!("\x1B[2J")).await.unwrap();
    write.write(escaped!("@DWelcome to the server.@")).await.unwrap();
    loop {
        let mut string_input = String::new();
        reader.read_line(&mut string_input).await.expect("Read error");
        if string_input == "quit" {
            server_state.client_states.lock().await.retain(|a| a.blocking_lock().addr != addr);
            break;
        }
        let string_input = string_input.replace(|a| a == '\r' || a == '\n', "");
        println!("{}", string_input);
        let response = process_client_command(string_input.clone(), addr, server_state.clone(), client_state.clone()).await;
        write.write(escaped! {response}).await.expect("Write error");
    }
    server_state.client_states.lock().await.retain(|a| a.blocking_lock().addr != addr);
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let addr: SocketAddrV4 = "127.0.0.1:8080".parse().unwrap();
    let server = TcpListener::bind(addr).await?;
    
    let mut rooms: HashMap<RoomAddr, Arc<Mutex<Room>>> = HashMap::new();

    rooms.insert("nexus".into(), to_arc_mutex(Room {
        addr: "nexus".into(),
        display: "The room is quiet... Except for a [@Csign@].".into(),
        clients: HashSet::new(),
        links: vec![],
        objects: {
            let mut some_hash = HashMap::new();
            some_hash.insert("sign".into(), GameObject {
                display: "Just a sign, I wonder what it says{@Cread@}.".into(),
                actions: {
                    HashMap::from([
                        ("read".into(), GameAction::PrintText("You're gay.".into()))
                    ])
                }
            });
            some_hash
        },
    }));

    let server_state = Arc::new(ServerState {
        client_states: Arc::new(Mutex::new(vec![])),
        rooms,
    });
    loop {
        let server_state = server_state.clone();
        let (socket, addr) = server.accept().await?;
        tokio::spawn(async move {
            process(socket, addr, server_state).await;
        });
    }
}
