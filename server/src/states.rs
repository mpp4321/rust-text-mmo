use std::{net::SocketAddr, sync::Arc, collections::{HashSet, HashMap}, fs::OpenOptions, io::{BufReader, BufWriter, Write}};

use lazy_static::lazy_static;
use regex::Regex;
use serde_derive::{Serialize, Deserialize};
use tokio::sync::Mutex;

pub fn to_arc_mutex<T>(owned: T) -> Arc<Mutex<T>> {
    Arc::new(Mutex::new(owned))
}

pub type ClientPointer = Arc<Mutex<ClientState>>;
pub type RoomPointer = Arc<Mutex<Room>>;

pub struct ClientState {
    pub addr: SocketAddr,
    pub is_edit_mode: bool,
    pub current_room: RoomAddr,
    pub name: String,
}

pub type RoomAddr = String;

#[derive(Serialize, Deserialize, Clone)]
pub enum GameAction {
    None,
    PrintText(String)
}

impl GameAction {
    pub fn handle(&self, _client_state: ClientPointer, _room: RoomPointer) -> String {
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

#[derive(Serialize, Deserialize, Clone)]
pub struct GameObject {
    pub display: String,
    pub actions: HashMap<String, GameAction>
}

impl GameObject {
    pub fn new(name: String) -> Self {
        Self {
            display: name,
            actions: HashMap::new()
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Room {
    pub addr: RoomAddr,
    pub display: String,
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
    pub fn new() -> Self {
        // For now json database...
        let mut options = OpenOptions::new();
        const PATH: &'static str = "database/world.json";
        options.create(true).read(true).write(true);
        std::fs::create_dir_all("database/").expect("Failed to create database directory!");
        let database_file = options.open(PATH).expect(&format!{"Failed to open {}", PATH});
        let database_file = BufReader::new(database_file);
        // empty hashmap in case file is empty
        let map: HashMap<RoomAddr, Room> = serde_json::from_reader(database_file).unwrap_or(HashMap::new());
        let mut map: HashMap<RoomAddr, Arc<Mutex<Room>>> = map.into_iter()
            .map(|(a, b)| (a, to_arc_mutex(b))).collect();

        if map.len() == 0 {
            map.insert("nexus".into(), to_arc_mutex(Room {
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
                                ("read".into(), GameAction::PrintText("Good job, you learned how to interact with objects!".into()))
                            ])
                        }
                    });
                    some_hash
                },
            }));
        }

        Self {
            client_states: to_arc_mutex(vec![]),
            rooms: map
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let mut options = OpenOptions::new();
        const PATH: &'static str = "database/world.json";
        options.create(true).write(true);
        std::fs::create_dir_all("database/").expect("Failed to create database directory!");
        let database_file = options.open(PATH).expect(&format!{"Failed to open {}", PATH});
        let mut database_file = BufWriter::new(database_file);
        let map: HashMap<String, Room> = self.rooms.iter()
            .map(|(a,b)| (a.clone(), b.try_lock().unwrap().clone())).collect();
        database_file.write(serde_json::to_string(&map).unwrap().as_bytes())?;
        Ok(())
    }

    pub fn get_room(&self, addr: &RoomAddr) -> Option<Arc<Mutex<Room>>> {
        self.rooms.get(addr).map(|a| a.clone())
    }
}
