use bmd_speededitor::Key;
use std::collections::HashMap;
use std::sync::mpsc;
use std::thread;
use tinyjson::JsonValue;
use websockets::{Frame, WebSocket};

#[derive(Debug)]
enum AppMessage {
    DoricoStatus(HashMap<String, JsonValue>),
    DorocoSelectionChanged(HashMap<String, JsonValue>),
    SpeedJog(u8, i32),
    SpeedKey(Key, bool),
}

#[derive(Debug)]
enum NoteInputActive {
    True,
    False,
}

#[tokio::main]
async fn main() {
    //---------------------------------------------
    // Set up app internal message channel
    let (app_message_sender, app_message_reciever) = mpsc::channel::<AppMessage>();
    let sender_kbd = app_message_sender.clone();
    let sender_jog = app_message_sender.clone();

    //---------------------------------------------
    // Set up websocket connection to Dorico Remote API
    let dorico_websocket = WebSocket::connect("ws://127.0.0.1:4560").await.unwrap();
    // Split the websocket into read and write entities
    let (mut dorico_ws_read, mut dorico_ws_write) = dorico_websocket.split();

    println!(">>> Dorico will now show a dialog box asking for permission to connect. ");
    println!(">>> Please accept by pressing the OK-button.");

    //---------------------------------------------
    // Send the handshake message to Dorico
    dorico_ws_write
        .send_text(r#"{"message": "connect","clientName": "TestClient","handshakeVersion": "1.0"}"#.into())
        .await
        .unwrap();

    //---------------------------------------------
    // As response, Dorico will send a session token
    let session_token = if let Frame::Text { payload: received_msg, .. } = dorico_ws_read.receive().await.unwrap() {
        let parsed: JsonValue = received_msg.as_str().parse().unwrap();
        let map: HashMap<_, _> = parsed.try_into().unwrap();
        let message = match map.get("message").unwrap() {
            JsonValue::String(x) => x.to_string(),
            _ => {
                panic!("Expected a string");
            }
        };
        match message.as_str() {
            "sessiontoken" => match map.get("sessionToken").unwrap() {
                JsonValue::String(_session_token) => _session_token.to_string(),
                _ => panic!("Expected a string 1"),
            },
            _ => panic!("Expected a string 2"),
        }
    } else {
        panic!("No session_token received");
    };

    //---------------------------------------------
    // Send the accept request message to Dorico
    dorico_ws_write.send_text(dorico_accept_message(&session_token).into()).await.unwrap();

    //---------------------------------------------
    // As response, Dorico will send a connected message
    let _connected = if let Frame::Text { payload: received_msg, .. } = dorico_ws_read.receive().await.unwrap() {
        let parsed: JsonValue = received_msg.as_str().parse().unwrap();
        let map: HashMap<_, _> = parsed.try_into().unwrap();
        let message = match map.get("code").unwrap() {
            JsonValue::String(x) => x.to_string(),
            _ => panic!("Expected a string 4"),
        };
        match message.as_str() {
            "kConnected" => true,
            _ => false,
        }
    } else {
        panic!("No connected message received from Dorico");
    };

    //---------------------------------------------
    // Set up a listener for Dorico messages such as status and sectionchanges
    let _handle_dorico_socket = tokio::spawn(async move {
        loop {
            let frame = dorico_ws_read.receive().await.unwrap();
            match frame {
                Frame::Text { payload: received_msg, .. } => {
                    let parsed: JsonValue = received_msg.as_str().parse().unwrap();
                    let map: HashMap<_, _> = parsed.try_into().unwrap();
                    let message = match map.get("message").unwrap() {
                        JsonValue::String(x) => x.to_string(),
                        _ => {
                            panic!("Expected a string");
                        }
                    };

                    match message.as_str() {
                        "status" => {
                            app_message_sender.send(AppMessage::DoricoStatus(map)).unwrap();
                        }
                        "selectionchanged" => {
                            app_message_sender.send(AppMessage::DorocoSelectionChanged(map)).unwrap();
                        }
                        "response" => {
                            //
                        }
                        _ => {
                            dbg!(&message);
                        }
                    }
                }
                _ => (),
            }
        }
    });

    //--------------------------------------------------------------------
    // Set up the Blackmagic Speed Editor Keyboard
    let mut speed_editor_kbd = bmd_speededitor::new().unwrap();

    speed_editor_kbd.on_connected(|| {
        println!(">>> Connected to the Speed Editor device");
        Ok(())
    });
    speed_editor_kbd.on_disconnected(|| {
        println!(">>> Disconnected from the device");
        Ok(())
    });
    speed_editor_kbd.on_keys(|keys| {
        println!("current keys are: {:?}", keys);
        Ok(())
    });
    speed_editor_kbd.on_key(move |key, down| {
        sender_kbd.send(AppMessage::SpeedKey(key, down)).unwrap();
        Ok(())
    });
    speed_editor_kbd.on_jog(move |mode, value| {
        sender_jog.send(AppMessage::SpeedJog(mode, value)).unwrap();
        Ok(())
    });
    speed_editor_kbd.on_unknown(|data| {
        println!("unknown event: {:?}", data);
        Ok(())
    });

    let _handle_speed_editor = thread::spawn(move || {
        speed_editor_kbd.run().expect("Expected Speed Editor to run");
    });

    //--------------------------------------------------------------------
    // Set up listener for application messages

    let mut jog_delta = 0;
    let mut input_note_active: NoteInputActive = NoteInputActive::False;

    for app_message in app_message_reciever {
        match app_message {
            AppMessage::SpeedKey(key, down) => match down {
                true => match key {
                    Key::Shtl => {
                        let _ = &dorico_ws_write
                            .send_text(dorico_command("Window.SwitchMode?WindowMode=kWriteMode", &session_token).into())
                            .await
                            .unwrap();

                        let _ = &dorico_ws_write.send_text(dorico_command("NoteInput.Enter?Set=true", &session_token).into()).await.unwrap();
                    }
                    Key::Cam4 => {
                        let _ = &dorico_ws_write
                            .send_text(dorico_command("NoteInput.NoteValue?LogDuration=kQuaver", &session_token).into())
                            .await
                            .unwrap();
                    }
                    Key::Cam5 => {
                        let _ = &dorico_ws_write
                            .send_text(dorico_command("NoteInput.NoteValue?LogDuration=kCrotchet", &session_token).into())
                            .await
                            .unwrap();
                    }
                    Key::Cam6 => {
                        let _ = &dorico_ws_write
                            .send_text(dorico_command("NoteInput.NoteValue?LogDuration=kMinim", &session_token).into())
                            .await
                            .unwrap();
                    }
                    Key::Roll => {
                        let _ = &dorico_ws_write.send_text(dorico_command("NoteInput.SlurStart", &session_token).into()).await.unwrap();
                    }
                    Key::StopPlay => {
                        let _ = &dorico_ws_write
                            .send_text(dorico_command("Play.StartOrStop?PlayFromLocation=kSelection", &session_token).into())
                            .await
                            .unwrap();
                    }
                    _ => {
                        dbg!(&key);
                    }
                },
                false => match key {
                    Key::Roll => {
                        let _ = &dorico_ws_write.send_text(dorico_command("NoteInput.SlurStop", &session_token).into()).await.unwrap();
                    }
                    _ => {
                        dbg!(&key);
                    }
                },
            },

            AppMessage::SpeedJog(_mode, value) => {
                // println!("value:{}, jog_delta:{}", value, jog_delta);
                jog_delta += value;

                match input_note_active {
                    NoteInputActive::True => match jog_delta {
                        200.. => {
                            let _ = &dorico_ws_write.send_text(dorico_command("NoteInput.MoveRight", &session_token).into()).await.unwrap();
                            jog_delta = 0;
                        }
                        ..-200 => {
                            let _ = &dorico_ws_write.send_text(dorico_command("NoteInput.MoveLeft", &session_token).into()).await.unwrap();
                            jog_delta = 0;
                        }
                        _ => {}
                    },
                    NoteInputActive::False => match jog_delta {
                        600.. => {
                            let _ = &dorico_ws_write.send_text(dorico_command("EventEdit.NavigateRight", &session_token).into()).await.unwrap();
                            jog_delta = 0;
                        }
                        ..-600 => {
                            let _ = &dorico_ws_write.send_text(dorico_command("EventEdit.NavigateLeft", &session_token).into()).await.unwrap();
                            jog_delta = 0
                        }
                        _ => {}
                    },
                };
            }
            AppMessage::DoricoStatus(status_map) => {
                // dbg!(&status_map);

                let map_note_input_active = status_map.get("noteInputActive");

                match map_note_input_active {
                    Some(&JsonValue::Boolean(true)) => {
                        input_note_active = NoteInputActive::True;
                    }
                    Some(&JsonValue::Boolean(false)) => {
                        input_note_active = NoteInputActive::False;
                    }
                    _ => {}
                }

                // dbg!(&input_note_active);
            }
            AppMessage::DorocoSelectionChanged(_selection_map) => {
                // dbg!(&selection_map);
            }
        }
    }
}

fn dorico_accept_message(session_token: &str) -> String {
    format!("{{\"message\": \"acceptsessiontoken\",\"sessionToken\":\"{session_token}\"}}")
}

fn dorico_command(msg: &str, session_token: &str) -> String {
    format!("{{\"message\": \"command\",\"sessionToken\":\"{session_token}\",\"command\":\"{msg}\"}}")
}
