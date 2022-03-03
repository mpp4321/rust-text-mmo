use std::{net::SocketAddr, sync::Arc, collections::{HashSet, HashMap}, slice::SliceIndex};

use lazy_static::lazy_static;
use regex::Regex;
use tokio::sync::Mutex;

pub fn to_arc_mutex<T>(owned: T) -> Arc<Mutex<T>> {
    Arc::new(Mutex::new(owned))
}

pub type ClientPointer = Arc<Mutex<ClientState>>;

pub struct ClientState {
    pub addr: SocketAddr,
    pub is_edit_mode: bool,
    pub current_room: RoomAddr,
    pub name: String,
}

pub type RoomAddr = String;

pub enum GameAction {
    None,
    PrintText(String)
}

impl GameAction {
    pub fn handle(&self) -> String {
        match *self {
            Self::PrintText(ref some) =>  {
                return some.clone();
            }
            _ => { String::from("Unhandled") }
        }
    }

    pub fn parse_from_string(string: String) -> GameAction {
        use GameAction::*;

        lazy_static! {
            static ref PRINT_TEXT_REGEX: Regex = Regex::new("PrintText (.+)").unwrap();
        }

        if PRINT_TEXT_REGEX.is_match(string.as_str()) {
            return PrintText(PRINT_TEXT_REGEX.captures(&string).unwrap().get(1).expect("Regex error").as_str().into())
        }

        GameAction::None
    }
}

pub struct GameObject {
    pub display: String,
    pub actions: HashMap<String, GameAction>
}

pub struct Room {
    pub addr: RoomAddr,
    // Clients by their address
    pub clients: HashSet<SocketAddr>,
    pub links: Vec<RoomAddr>,
    pub objects: HashMap<String, GameObject>
}

impl ClientState {
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            is_edit_mode: false,
            current_room: "nexus".into(),
            name: String::new()
        }
    }

    pub fn to_pointer(self) -> ClientPointer 
    {
        to_arc_mutex(self)
    }
}

pub struct ServerState {
    pub client_states: Arc<Mutex<Vec<ClientPointer>>>,
    pub rooms: HashMap<RoomAddr, Arc<Mutex<Room>>>,
}

impl ServerState {
    pub fn get_room(&self, addr: &RoomAddr) -> Option<Arc<Mutex<Room>>> {
        self.rooms.get(addr).map(|a| a.clone())
    }
}
