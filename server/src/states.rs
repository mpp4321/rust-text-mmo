use std::{net::SocketAddr, sync::Arc, collections::{HashSet, HashMap}, fs::OpenOptions, io::{BufReader, BufWriter, Write, Read}};

use dyon::Runtime;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{ser::SerializeStruct, Deserializer};
use serde_derive::{Serialize, Deserialize};
use tokio::sync::Mutex;

pub fn to_arc_mutex<T>(owned: T) -> Arc<Mutex<T>> {
    Arc::new(Mutex::new(owned))
}

pub fn hmmutex<'de, D>(d: D) -> Result<Arc<std::sync::Mutex<HashMap<String, String>>>, D::Error>
where
    D: Deserializer<'de>,
{
    let map_val: HashMap<String, String> = serde::de::Deserialize::deserialize(d)?;
    return Ok(Arc::new(std::sync::Mutex::new(map_val)));
}

pub type ClientPointer = Arc<Mutex<ClientState>>;

pub type RoomPointer = Arc<Mutex<Room>>;

#[derive(Deserialize)]
pub struct ClientState {
    pub addr: Option<SocketAddr>,

    pub is_edit_mode: bool,
    pub current_room: RoomAddr,
    pub name: String,
    // Should work...
    #[serde(deserialize_with = "hmmutex")]
    pub client_script_states: Arc<std::sync::Mutex<HashMap<String, String>>>
}

impl serde::Serialize for ClientState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        let mut client_state = serializer.serialize_struct("ClientState", 4)?;
        client_state.serialize_field("is_edit_mode", &self.is_edit_mode)?;
        client_state.serialize_field("current_room", &self.current_room)?;
        client_state.serialize_field("name", &self.name)?;
        client_state.serialize_field("client_script_states",
                                     &self.client_script_states.lock()
                                     .map(|a| a.clone())
                                     .unwrap_or(HashMap::<String, String>::new()))?;
        client_state.end()
    }
}

pub type RoomAddr = String;

#[derive(Serialize, Deserialize, Clone)]
pub enum GameAction {
    None,
    PrintText(String),
    RunScript(String)
}

impl GameAction {
    pub async fn handle(&self, _client_state: ClientPointer, _room: RoomPointer, _runtime: Arc<Mutex<Runtime>>) -> String {
        match *self {
            Self::PrintText(ref some) =>  {
                return some.clone();
            },
            Self::RunScript(ref some) =>  {
                let return_type = crate::dyon_inter::load_and_run(&format!{ "dyon/{}.dyon", some }, _client_state.clone(), &_runtime).await;
                if return_type.is_err() {
                    return format! { "Code error, script did not return a String" };
                }
                let return_type = return_type.unwrap().0.unwrap();
                if let dyon::Variable::Str(arc_str) = return_type {
                    return (*arc_str).clone();
                }
                format!{ "Script returned non-string type in home fn" }
            }
            _ => { String::from("Unhandled") }
        }
    }

    pub fn parse_from_string(string: String) -> GameAction {
        use GameAction::*;

        lazy_static! {
            static ref PRINT_TEXT_REGEX: Regex = Regex::new("PrintText (.+)").unwrap();
            static ref RUN_SCRIPT_REGEX: Regex = Regex::new("RunScript (.+)").unwrap();
        }

        if PRINT_TEXT_REGEX.is_match(string.as_str()) {
            return PrintText(PRINT_TEXT_REGEX.captures(&string).unwrap().get(1).expect("Regex error").as_str().into())
        }

        if RUN_SCRIPT_REGEX.is_match(string.as_str()) {
            return RunScript(RUN_SCRIPT_REGEX.captures(&string).unwrap().get(1).expect("Regex error").as_str().into())
        }

        GameAction::None
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GameObject {
    pub display: String,
    pub name: String,
    pub actions: HashMap<String, GameAction>
}

impl GameObject {
    pub fn new(name: String) -> Self {
        Self {
            name: name.clone(),
            display: name,
            actions: HashMap::new()
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Room {
    pub addr: RoomAddr,
    pub display: String,
    // Clients by their address
    pub clients: HashSet<SocketAddr>,
    pub links: Vec<RoomAddr>,
    pub objects: HashMap<String, GameObject>
}

impl Room {
    fn new(addr: RoomAddr) -> Self {
        Self {
            addr,
            ..Default::default()
        }
    }
}

impl ClientState {

    pub fn new(addr: Option<SocketAddr>) -> Self {
        Self {
            addr,
            is_edit_mode: false,
            current_room: "nexus".into(),
            name: String::new(),
            client_script_states: Arc::new(std::sync::Mutex::new(HashMap::<String, String>::new()))
        }
    }

    pub fn to_pointer(self) -> ClientPointer 
    {
        to_arc_mutex(self)
    }
}

pub struct ServerState {
    pub client_states: Arc<Mutex<Vec<ClientPointer>>>,
    pub rooms: Mutex<HashMap<RoomAddr, Arc<Mutex<Room>>>>,
    pub runtime: Arc<Mutex<dyon::Runtime>>
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
                display: "The room is quiet... Except for a [@Csign].".into(),
                clients: HashSet::new(),
                links: vec![],
                objects: {
                    let mut some_hash = HashMap::new();
                    some_hash.insert("sign".into(), GameObject {
                        display: "Just a sign, I wonder what it says{@Cread}.".into(),
                        name: "sign".into(),
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
            rooms: Mutex::new(map),
            runtime: to_arc_mutex(dyon::Runtime::new()),
        }
    }

    pub async fn save_client(&self, client_state: ClientPointer) -> std::io::Result<()> {
        let mut options = OpenOptions::new();
        let path: String = format!{"database/{}.json", client_state.lock().await.name};
        options.create(true).write(true);
        std::fs::create_dir_all("database/").expect("Failed to create database directory!");
        let mut database_file = options.open(&path).expect(&format!{"Failed to open {}", path});
        database_file.write(
            serde_json::to_string(&*client_state.lock().await).unwrap().as_bytes()
        )?;
        Ok(())
    }

    pub async fn load_client(&self, name: String) -> Option<ClientState> {
        let mut options = OpenOptions::new();
        let path: String = format!{"database/{}.json", name};
        options.create(false).read(true);
        std::fs::create_dir_all("database/").expect("Failed to create database directory!");
        let database_file = options.open(&path);
        if database_file.is_err() {
            return None;
        }
        let mut database_file = database_file.unwrap();
        let mut string_buf = String::new();
        database_file.read_to_string(&mut string_buf).expect("Failed to read in client");
        return serde_json::from_str(&string_buf).map(|a| Some(a)).unwrap_or(None);
    }

    pub fn save(&self) -> std::io::Result<()> {
        let mut options = OpenOptions::new();
        const PATH: &'static str = "database/world.json";
        options.create(true).write(true);
        std::fs::create_dir_all("database/").expect("Failed to create database directory!");
        let database_file = options.open(PATH).expect(&format!{"Failed to open {}", PATH});
        let mut database_file = BufWriter::new(database_file);
        let map: HashMap<String, Room> = self.rooms.try_lock().expect("Failed to acquire rooms").iter()
            .map(|(a,b)| (a.clone(), b.try_lock().unwrap().clone())).collect();
        database_file.write(serde_json::to_string(&map).unwrap().as_bytes())?;
        Ok(())
    }

    pub fn get_room(&self, addr: &RoomAddr) -> Option<Arc<Mutex<Room>>> {
        let locked_room = self.rooms.try_lock();
        if locked_room.is_err() {
            return None;
        }
        locked_room.unwrap().get(addr).map(|a| a.clone())
    }

    pub fn new_room(&self, addr: &RoomAddr) {
        let locked_room = self.rooms.try_lock();
        if locked_room.is_err() {
            return;
        }
        locked_room.unwrap().insert(
            addr.clone(), Arc::new(Mutex::new(Room::new(addr.clone())))
        );
    }
}
