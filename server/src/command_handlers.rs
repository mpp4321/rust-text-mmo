use std::sync::Arc;

use regex::Regex;

use crate::{lazy_static, states::{ServerState, ClientPointer, GameObject, GameAction}};

pub async fn add_object(input: &String, server_state: Arc<ServerState>, client: ClientPointer) -> String {
    lazy_static! {
        static ref ADD_OBJECT_REGEX: Regex = Regex::new("\\add (.+)").unwrap();
    }

    // TODO make this a macro
    if !ADD_OBJECT_REGEX.is_match(&input) {
        return String::from("add <object name>");
    }

    let captures = ADD_OBJECT_REGEX.captures(&input).unwrap();
    let object_name = captures.get(1).unwrap().as_str();

    let room = server_state.get_room(&client.lock().await.current_room);
    if let Some(room) = room {
        let mut room = room.lock().await;
        room.objects.insert(object_name.into(), GameObject::new(object_name.into()));
        return format!{"Added"};
    }
    return format!{"Not in a room?"};
}

pub async fn describe_object(input: &String, server_state: Arc<ServerState>, client: ClientPointer) -> String {
    lazy_static! {
        // We have quotes here because object name may contain spaces
        static ref DESCRIBE_OBJECT: Regex = Regex::new("\\describe \"(.+)\" (.+)").unwrap();
    }

    if !DESCRIBE_OBJECT.is_match(&input) {
        return String::from("describe \"<object name>\" <description>");
    }

    let captures = DESCRIBE_OBJECT.captures(&input).unwrap();
    let object_name = captures.get(1).unwrap().as_str();
    let object_description = captures.get(2).unwrap().as_str();

    let room = server_state.get_room(&client.lock().await.current_room);
    if let Some(room) = room {
        let mut room = room.lock().await;
        let game_object_ref = room.objects.get_mut(object_name.into());
        if let Some(game_object_ref) = game_object_ref {
            game_object_ref.display = object_description.into();
            return format!{"Done"};
        }
        return format!{"Not a valid object."};
    }
    return format!{"Not in a room?"};
}

pub async fn add_action(input: &String, server_state: Arc<ServerState>, client: ClientPointer) -> String {
    lazy_static! {
        // We have quotes here because object name may contain spaces, : to name the action
        static ref ADD_ACTION: Regex = Regex::new("\\action \"(.+)\":(.+) (.+)").unwrap();
    }

    if !ADD_ACTION.is_match(&input) {
        return String::from("action \"<object name>\":<action name> <action string>");
    }

    let captures = ADD_ACTION.captures(&input).unwrap();
    let object_name = captures.get(1).unwrap().as_str();
    let action_name = captures.get(2).unwrap().as_str();
    let action_string = captures.get(3).unwrap().as_str();

    let room = server_state.get_room(&client.lock().await.current_room);
    if let Some(room) = room {
        let mut room = room.lock().await;
        let game_object_ref = room.objects.get_mut(object_name.into());
        if let Some(game_object_ref) = game_object_ref {
            game_object_ref.actions.insert(action_name.into(),
                GameAction::parse_from_string(action_string.into()));
            return format!{"Done"};
        }
        return format!{"Not a valid object."};
    }
    return format!{"Not in a room?"};
}

pub async fn handle_touch(_input: &String, _server_state: Arc<ServerState>, _my_client: ClientPointer) -> String {
    lazy_static! {
        // i for interact, used to be touch changed so its not so annoying to type
        // i <name> <action>
        static ref OBJECT_USE_REGEX: Regex = Regex::new("i (.+) (.+)").unwrap();
    }

    if !OBJECT_USE_REGEX.is_match(_input) {
        return String::from("i <object name> <action>");
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
