use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::{io::ErrorKind, sync::Arc};

use dyon::{error, Runtime, Module, Dfn, Type, load, dyon_fn, Variable, runtime::Flow, ast, embed::PushVariable};
use dyon::{dyon_macro_items, RustObject};
use dyon::dyon_fn_pop;
use tokio::sync::Mutex;

use crate::states::ClientPointer;

pub async fn load_and_run(path: &String, client_state: ClientPointer, runtime: &Arc<Mutex<Runtime>>) -> std::io::Result<(Option<Variable>, Flow)> {
    let dyon_module= load_module(path)?;
    let dyon_module = Arc::new(dyon_module);
    let find = dyon_module.find_function(&Arc::new("home".into()), 0);
    let data = client_state.try_lock().expect("Failed to lock data").client_script_states.clone();
    let call_res = runtime.lock().await.call(&ast::Call {
        args: vec![
                ast::Expression::Variable(
                    Box::new((range::Range::empty(0), (data as RustObject).push_var()))
                )
              ],
        f_index: find,
        custom_source: None,
        info: Box::new(ast::CallInfo {
            name: Arc::new("home".into()),
            alias: None,
            source_range: range::Range::empty(0)
        })
    }, &dyon_module);
    if let Err(_) = call_res {
        return Err(std::io::Error::new(ErrorKind::Other, "Dyon failed to run file"));
    }
    Ok(call_res.unwrap())
}

dyon_fn! {
    fn test_func() {
    }
}

fn get_client_state<'a>(v: &'a std::sync::MutexGuard<'_, dyn std::any::Any>) -> Option<&'a HashMap<String, String>> {
    if let Some(val) = (*v.deref()).downcast_ref::<HashMap<String, String>>() {
        return Some(val);
    }
    None
}

fn get_client_state_mut<'a>(v: &'a mut std::sync::MutexGuard<'_, dyn std::any::Any>) -> Option<&'a mut HashMap<String, String>> {
    if let Some(val) = (*v.deref_mut()).downcast_mut::<HashMap<String, String>>() {
        return Some(val);
    }
    None
}

dyon_fn! {
    fn num(val: String) -> Option<f64> {
        let parsed = val.parse::<f64>();
        if let Ok(parsed) = parsed {
            return Some(parsed);
        }
        return None;
    }
}

dyon_fn! {
    fn set_state(a: RustObject, key: String, value: String) {
        let mut g: std::sync::MutexGuard<'_, dyn std::any::Any> = a.lock().unwrap();
        let state = get_client_state_mut(&mut g);
        if let Some(state) = state {
            state.insert(key, value);
        }
    }
}

dyon_fn! {
    fn get_state(a: RustObject, key: String) -> Option<String> {
        let g: std::sync::MutexGuard<'_, dyn std::any::Any> = a.lock().unwrap();
        let state = get_client_state(&g);
        if let Some(state) = state {
            return state.get(&key).map(|a| a.clone());
        }
        return None;
    }
}

fn load_module(path: &String) -> std::io::Result<dyon::Module> {
    let mut module = Module::new();

    let type_n = Type::AdHoc(Arc::new("StateObject".into()), Box::new(Type::Any)); 
    module.add_str("get_state", get_state, Dfn::nl(vec![type_n.clone(), Type::Str], Type::Option(Box::new(Type::Str))));
    module.add_str("set_state", set_state, Dfn::nl(vec![type_n.clone(), Type::Str, Type::Str], Type::Void));
    module.add_str("num", num, Dfn::nl(vec![Type::Str], Type::Option(Box::new(Type::F64))));
    
    module.add_str("test_func", test_func, Dfn::nl(vec![], Type::Void));

    if error(load(path, &mut module)) {
        return Err(std::io::Error::new(ErrorKind::InvalidData, "dyon script not valid or not found dyon/loader.dyon"));
    }

    Ok(module)
}
