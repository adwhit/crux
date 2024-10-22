use anyhow::Result;
use crux_core::{macros::Effect, render::Render, testing::AppTester, Command};
use serde::{Deserialize, Serialize};

use crate::{
    error::KeyValueError, value::Value, KeyValue, KeyValueOperation, KeyValueResponse,
    KeyValueResult,
};

#[derive(Default)]
pub struct App;

#[derive(Debug, Serialize, Deserialize)]
pub enum Event {
    Get,
    Set,
    Delete,
    Exists,
    ListKeys,
    GetThenSet,

    GetResponse(Result<Option<Vec<u8>>, KeyValueError>),
    SetResponse(Result<Option<Vec<u8>>, KeyValueError>),
    ExistsResponse(Result<bool, KeyValueError>),
    ListKeysResponse(Result<(Vec<String>, u64), KeyValueError>),
}

#[derive(Debug, Default)]
pub struct Model {
    pub value: i32,
    pub keys: Vec<String>,
    pub cursor: u64,
    pub successful: bool,
}

#[derive(Serialize, Deserialize, Default)]
pub struct ViewModel {
    pub result: String,
}

impl crux_core::App for App {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;

    type Capabilities = Capabilities;

    fn update(&self, event: Event, model: &mut Model, caps: &Capabilities) -> Command<Event> {
        let key = "test".to_string();
        match event {
            Event::Get => caps.key_value.get(key, Event::GetResponse),
            Event::Set => caps
                .key_value
                .set(key, 42i32.to_ne_bytes().to_vec(), Event::SetResponse),
            Event::Delete => caps.key_value.delete(key, Event::SetResponse),
            Event::Exists => caps.key_value.exists(key, Event::ExistsResponse),
            Event::ListKeys => {
                caps.key_value
                    .list_keys("test:".to_string(), 0, Event::ListKeysResponse)
            }

            Event::GetThenSet => {
                let kv = caps.key_value.clone();

                let fut = async move {
                    let Result::Ok(Some(value)) = kv.get_async("test_num".to_string()).await else {
                        panic!("expected get response with a value");
                    };

                    let num = i32::from_ne_bytes(value.try_into().unwrap());
                    let result = kv
                        .set_async("test_num".to_string(), (num + 1).to_ne_bytes().to_vec())
                        .await;

                    Command::event(Event::SetResponse(result))
                };
                Command::effect(fut)
            }

            Event::GetResponse(Ok(Some(value))) => {
                let (int_bytes, _rest) = value.split_at(std::mem::size_of::<i32>());
                model.value = i32::from_ne_bytes(int_bytes.try_into().unwrap());
                Command::none()
            }

            Event::GetResponse(Ok(None)) => {
                panic!("expected value");
            }

            Event::SetResponse(Ok(_response)) => {
                model.successful = true;
                caps.render.render()
            }

            Event::ExistsResponse(Ok(_response)) => {
                model.successful = true;
                caps.render.render()
            }

            Event::ListKeysResponse(Ok((keys, cursor))) => {
                model.keys = keys;
                model.cursor = cursor;
                caps.render.render()
            }

            Event::GetResponse(Err(error)) => {
                panic!("error: {:?}", error);
            }
            Event::SetResponse(Err(error)) => {
                panic!("error: {:?}", error);
            }
            Event::ExistsResponse(Err(error)) => {
                panic!("Error: {:?}", error);
            }
            Event::ListKeysResponse(Err(error)) => {
                panic!("Error: {:?}", error);
            }
        }
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        ViewModel {
            result: format!("Success: {}, Value: {}", model.successful, model.value),
        }
    }
}

#[derive(Effect)]
pub struct Capabilities {
    pub key_value: KeyValue,
    pub render: Render,
}

#[test]
fn test_get() {
    let app = AppTester::<App, _>::default();
    let mut model = Model::default();

    let updated = app.update(Event::Get, &mut model);

    let effect = updated.into_effects().next().unwrap();
    let Effect::KeyValue(mut request) = effect else {
        panic!("Expected KeyValue effect");
    };

    let KeyValueOperation::Get { key } = request.operation.clone() else {
        panic!("Expected get operation");
    };

    assert_eq!(key, "test");

    let updated = app
        .resolve(
            &mut request,
            KeyValueResult::Ok {
                response: KeyValueResponse::Get {
                    value: 42i32.to_ne_bytes().to_vec().into(),
                },
            },
        )
        .unwrap();

    let event = updated.events.into_iter().next().unwrap();
    app.update(event, &mut model);

    assert_eq!(model.value, 42);
}

#[test]
fn test_set() {
    let app = AppTester::<App, _>::default();
    let mut model = Model::default();

    let updated = app.update(Event::Set, &mut model);

    let effect = updated.into_effects().next().unwrap();
    let Effect::KeyValue(mut request) = effect else {
        panic!("Expected KeyValue effect");
    };

    let KeyValueOperation::Set { key, value } = request.operation.clone() else {
        panic!("Expected set operation");
    };

    assert_eq!(key, "test");
    assert_eq!(value, 42i32.to_ne_bytes().to_vec());

    let updated = app
        .resolve(
            &mut request,
            KeyValueResult::Ok {
                response: KeyValueResponse::Set {
                    previous: Value::None,
                },
            },
        )
        .unwrap();

    let event = updated.events.into_iter().next().unwrap();
    app.update(event, &mut model);

    assert!(model.successful);
}

#[test]
fn test_delete() {
    let app = AppTester::<App, _>::default();
    let mut model = Model::default();

    let updated = app.update(Event::Delete, &mut model);

    let effect = updated.into_effects().next().unwrap();
    let Effect::KeyValue(mut request) = effect else {
        panic!("Expected KeyValue effect");
    };

    let KeyValueOperation::Delete { key } = request.operation.clone() else {
        panic!("Expected delete operation");
    };

    assert_eq!(key, "test");

    let updated = app
        .resolve(
            &mut request,
            KeyValueResult::Ok {
                response: KeyValueResponse::Delete {
                    previous: Value::None,
                },
            },
        )
        .unwrap();

    let event = updated.events.into_iter().next().unwrap();
    app.update(event, &mut model);

    assert!(model.successful);
}

#[test]
fn test_exists() {
    let app = AppTester::<App, _>::default();
    let mut model = Model::default();

    let updated = app.update(Event::Exists, &mut model);

    let effect = updated.into_effects().next().unwrap();
    let Effect::KeyValue(mut request) = effect else {
        panic!("Expected KeyValue effect");
    };

    let KeyValueOperation::Exists { key } = request.operation.clone() else {
        panic!("Expected exists operation");
    };

    assert_eq!(key, "test");

    let updated = app
        .resolve(
            &mut request,
            KeyValueResult::Ok {
                response: KeyValueResponse::Exists { is_present: true },
            },
        )
        .unwrap();

    let event = updated.events.into_iter().next().unwrap();
    app.update(event, &mut model);

    assert!(model.successful);
}

#[test]
fn test_list_keys() {
    let app = AppTester::<App, _>::default();
    let mut model = Model::default();

    let updated = app.update(Event::ListKeys, &mut model);

    let effect = updated.into_effects().next().unwrap();
    let Effect::KeyValue(mut request) = effect else {
        panic!("Expected KeyValue effect");
    };

    let KeyValueOperation::ListKeys { prefix, cursor } = request.operation.clone() else {
        panic!("Expected list keys operation");
    };

    assert_eq!(prefix, "test:");
    assert_eq!(cursor, 0);

    let updated = app
        .resolve(
            &mut request,
            KeyValueResult::Ok {
                response: KeyValueResponse::ListKeys {
                    keys: vec!["test:1".to_string(), "test:2".to_string()],
                    next_cursor: 2,
                },
            },
        )
        .unwrap();

    let event = updated.events.into_iter().next().unwrap();
    app.update(event, &mut model);

    assert_eq!(model.keys, vec!["test:1".to_string(), "test:2".to_string()]);
    assert_eq!(model.cursor, 2);
}

#[test]
pub fn test_kv_async() -> Result<()> {
    let app = AppTester::<App, _>::default();
    let mut model = Model::default();

    let update = app.update(Event::GetThenSet, &mut model);

    let effect = update.into_effects().next().unwrap();
    let Effect::KeyValue(mut request) = effect else {
        panic!("Expected KeyValue effect");
    };

    let KeyValueOperation::Get { key } = request.operation.clone() else {
        panic!("Expected get operation");
    };

    assert_eq!(key, "test_num");

    let update = app
        .resolve(
            &mut request,
            KeyValueResult::Ok {
                response: KeyValueResponse::Get {
                    value: 17u32.to_ne_bytes().to_vec().into(),
                },
            },
        )
        .unwrap();

    let effect = update.into_effects().next().unwrap();
    let Effect::KeyValue(mut request) = effect else {
        panic!("Expected KeyValue effect");
    };

    let KeyValueOperation::Set { key, value } = request.operation.clone() else {
        panic!("Expected get operation");
    };

    assert_eq!(key, "test_num".to_string());
    assert_eq!(value, 18u32.to_ne_bytes().to_vec());

    let update = app
        .resolve(
            &mut request,
            KeyValueResult::Ok {
                response: KeyValueResponse::Set {
                    previous: Value::None,
                },
            },
        )
        .unwrap();

    let event = update.events.into_iter().next().unwrap();
    app.update(event, &mut model);

    assert!(model.successful);

    Ok(())
}

#[test]
fn test_kv_operation_debug_repr() {
    {
        // get
        let op = KeyValueOperation::Get {
            key: "my key".into(),
        };
        let repr = format!("{op:?}");
        assert_eq!(repr, r#"Get { key: "my key" }"#);
    }

    {
        // set small
        let op = KeyValueOperation::Set {
            key: "my key".into(),
            value: b"my value".to_vec(),
        };
        let repr = format!("{op:?}");
        assert_eq!(repr, r#"Set { key: "my key", value: "my value" }"#);
    }

    {
        // set big
        let op = KeyValueOperation::Set {
            key: "my key".into(),
            value:
                // we check that we handle unicode boundaries correctly
                "abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstu😀😀😀😀😀😀".as_bytes().to_vec(),
        };
        let repr = format!("{op:?}");
        assert_eq!(
            repr,
            r#"Set { key: "my key", value: "abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstu😀😀"... }"#
        );
    }

    {
        // set binary
        let op = KeyValueOperation::Set {
            key: "my key".into(),
            value: vec![255, 255],
        };
        let repr = format!("{op:?}");
        assert_eq!(
            repr,
            r#"Set { key: "my key", value: <binary data - 2 bytes> }"#
        );
    }
}
