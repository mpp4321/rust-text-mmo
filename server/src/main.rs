#![feature(async_closure)]

mod dyon_inter;
mod states;
mod command_handlers;

use std::{net::{SocketAddrV4, SocketAddr}, sync::Arc};
use command_handlers::{handle_touch, look, add_object, describe_object, add_action, upload_script};
use lazy_static::lazy_static;
use states::{ServerState, ClientState, ClientPointer};
use tokio::{net::{TcpListener, TcpStream}, io::{BufReader, AsyncBufReadExt, AsyncWriteExt}};

macro_rules! escaped {
    ($exp:expr) => {
        format!("{}\n\r", $exp).as_bytes()
    }
}

async fn process_builder_command(input: String, _addr: SocketAddr, server_state: Arc<ServerState>, my_client: ClientPointer) -> String {
    match &input[..input.find(" ").unwrap_or(input.len())] {
        "\\script" => {
            return upload_script(&input).await;
        },
        "\\add" => {
            return add_object(&input, server_state, my_client).await;
        },
        "\\describe" => {
            return describe_object(&input, server_state, my_client).await;
        },
        "\\action" => {
            return add_action(&input, server_state, my_client).await;
        },
        "\\save" => {
            server_state.save().expect("Failed to save server!!");
            return format!{"Nice save!"};
        },
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
        "i" => {
            return handle_touch(&input, server_state, my_client).await;
        },
        "look" => {
            return look(&input, server_state, my_client).await;
        },
        "help" => {
            return format!{
                "{}\n{}\n{}",
                "i - interacts with object: i {object} {action}",
                "look - reads the object's display text: look {object}",
                "help - you're here"
            };
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
        let response = process_client_command(string_input.clone(), addr, server_state.clone(), client_state.clone()).await;
        write.write(escaped! {response}).await.expect("Write error");
    }
    server_state.client_states.lock().await.retain(|a| a.blocking_lock().addr != addr);
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let addr: SocketAddrV4 = "127.0.0.1:8080".parse().unwrap();
    let server = TcpListener::bind(addr).await?;

    let server_state = Arc::new(ServerState::new());
    //dyon_inter::load_and_run(&"dyon/test.dyon".into(), &server_state.runtime).await?;
    loop {
        let server_state = server_state.clone();
        let (socket, addr) = server.accept().await?;
        tokio::spawn(async move {
            process(socket, addr, server_state).await;
        });
    }
}
