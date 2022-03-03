use std::sync::Arc;

use regex::Regex;

use crate::{lazy_static, states::{ServerState, ClientPointer}};

pub async fn handle_touch(_input: &String, _server_state: Arc<ServerState>, _my_client: ClientPointer) -> String {
    lazy_static! {
        // touch <name> <action>
        static ref OBJECT_USE_REGEX: Regex = Regex::new("touch (.+) (.+)").unwrap();
    }

    if !OBJECT_USE_REGEX.is_match(_input) {
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
            return action.handle(_my_client.clone(), room_un.clone());
        } else {
            return "The object does not have that action".into();
        }
    } else {
        return "The object does not exist".into();
    }
}

pub async fn look(_input: &String, _server_state: Arc<ServerState>, _my_client: ClientPointer) -> String {
    lazy_static! {
        // touch <name> <action>
        static ref LOOK_REGEX: Regex = Regex::new("look (.+)").unwrap();
    }

    let room = _server_state.get_room(&_my_client.lock().await.current_room);
    if room.is_none() {
        return String::from("You belong to an invalid room.");
    }
    let room_un = room.unwrap();
    let room_ref = room_un.lock().await;
    if !LOOK_REGEX.is_match(_input) {
        return room_ref.display.clone();
    }
    let captures = LOOK_REGEX.captures(&_input).unwrap();
    let object_name = captures.get(1).unwrap().as_str();
    if let Some(object) = room_ref.objects.get(object_name) {
        return object.display.clone();
    } else {
        return "The object does not exist".into();
    }
}
