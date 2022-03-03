#![feature(async_closure)]

mod states;

use std::{net::{SocketAddrV4, SocketAddr}, sync::Arc, collections::{HashMap, HashSet}};
use lazy_static::lazy_static;
use regex::Regex;
use states::{ServerState, ClientState, ClientPointer, Room, RoomAddr, to_arc_mutex};
use tokio::{net::{TcpListener, TcpStream}, io::{BufReader, AsyncBufReadExt, AsyncWriteExt}, sync::Mutex};

macro_rules! escaped {
    ($exp:expr) => {
        format!("{}\n\r", $exp).as_bytes()
    }
}

async fn process_client_command(_input: String, _addr: SocketAddr, _server_state: Arc<ServerState>, _my_client: ClientPointer) -> String {
    //Manage server game state here

    lazy_static! {
        // touch <name> <action>
        static ref OBJECT_USE_REGEX: Regex = Regex::new("touch (.+) (.+)").unwrap();
    }

    match &_input[.._input.find(" ").unwrap_or(0)] {
        "touch" => {
            if !OBJECT_USE_REGEX.is_match(&_input) {
                return String::from("touch <object name> <action>");
            }
            let captures = OBJECT_USE_REGEX.captures(&_input).unwrap();
            let object_name = captures.get(1).unwrap().as_str();
            let object_action = captures.get(2).unwrap().as_str();
            let room = _server_state.get_room(&_my_client.lock().await.current_room);
            if room.is_none() {
                return String::from("You belong to an invalid room.");
            }
            let room_un = room.unwrap();
            let room_ref = room_un.lock().await;
            if let Some(object) = room_ref.objects.get(object_name) {
                let some_action = object.actions.get(object_action);
                if let Some(action) = some_action {
                    return action.handle();
                } else {
                    return "The object does not have that action".into();
                }
            } else {
                return "The object does not exist".into();
            }
        },
        _ => {}
    }

    return String::from("HI");
}

async fn process(mut _socket: TcpStream, addr: SocketAddr, server_state: Arc<ServerState>) {
    let (read, mut write) = _socket.split();
    let mut reader = BufReader::new(read);
    let client_state = ClientState::new(addr).to_pointer();
    server_state.client_states.lock().await.push(
        client_state.clone()
    );
    write.write(escaped!("\x1B[2J")).await.unwrap();
    write.write(escaped!("Welcome to the server.")).await.unwrap();
    loop {
        let mut string_input = String::new();
        reader.read_line(&mut string_input).await.expect("Read error");
        if string_input == "quit" {
            server_state.client_states.lock().await.retain(|a| a.blocking_lock().addr != addr);
            break;
        }
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
        clients: HashSet::new(),
        links: vec![],
        objects: HashMap::new(),
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
