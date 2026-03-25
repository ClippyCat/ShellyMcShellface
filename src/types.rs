use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusState {
    Connected,
    PtyExited,
    PtyError,
    UserQuit,
}

#[derive(Clone, Debug)]
pub enum SseEvent {
    Input { text: String },
    Output { text: String },
    Status { state: StatusState, code: Option<i32> },
}

impl SseEvent {
    pub fn output(text: impl Into<String>) -> Self {
        SseEvent::Output { text: text.into() }
    }

    pub fn input(text: impl Into<String>) -> Self {
        SseEvent::Input { text: text.into() }
    }

    pub fn connected() -> Self {
        SseEvent::Status { state: StatusState::Connected, code: None }
    }

    pub fn pty_exited(code: Option<i32>) -> Self {
        let emit_code = code.filter(|&c| c != 0);
        SseEvent::Status { state: StatusState::PtyExited, code: emit_code }
    }

    pub fn pty_error() -> Self {
        SseEvent::Status { state: StatusState::PtyError, code: None }
    }

    pub fn user_quit() -> Self {
        SseEvent::Status { state: StatusState::UserQuit, code: None }
    }

    /// Returns (event_type, json_data_string) for SSE wire format.
    pub fn to_sse_parts(&self) -> (String, String) {
        match self {
            SseEvent::Output { text } => {
                let data = serde_json::json!({ "text": text }).to_string();
                ("output".into(), data)
            }
            SseEvent::Input { text } => {
                let data = serde_json::json!({ "text": text }).to_string();
                ("input".into(), data)
            }
            SseEvent::Status { state, code } => {
                let data = serde_json::json!({ "state": state, "code": code }).to_string();
                ("status".into(), data)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_event_serialises_to_sse_parts() {
        let ev = SseEvent::output("hello\n");
        let (event_type, data) = ev.to_sse_parts();
        assert_eq!(event_type, "output");
        let json: serde_json::Value = serde_json::from_str(&data).unwrap();
        assert_eq!(json["text"], "hello\n");
    }

    #[test]
    fn test_input_event_serialises_to_sse_parts() {
        let ev = SseEvent::input("ls -la");
        let (event_type, data) = ev.to_sse_parts();
        assert_eq!(event_type, "input");
        let json: serde_json::Value = serde_json::from_str(&data).unwrap();
        assert_eq!(json["text"], "ls -la");
    }

    #[test]
    fn test_status_connected_serialises() {
        let ev = SseEvent::connected();
        let (event_type, data) = ev.to_sse_parts();
        assert_eq!(event_type, "status");
        let json: serde_json::Value = serde_json::from_str(&data).unwrap();
        assert_eq!(json["state"], "connected");
    }

    #[test]
    fn test_status_pty_exited_with_code() {
        let ev = SseEvent::pty_exited(Some(1));
        let (_, data) = ev.to_sse_parts();
        let json: serde_json::Value = serde_json::from_str(&data).unwrap();
        assert_eq!(json["state"], "pty_exited");
        assert_eq!(json["code"], 1);
    }

    #[test]
    fn test_status_pty_exited_code_zero_has_no_code_field() {
        let ev = SseEvent::pty_exited(Some(0));
        let (_, data) = ev.to_sse_parts();
        let json: serde_json::Value = serde_json::from_str(&data).unwrap();
        assert_eq!(json["code"], serde_json::Value::Null);
    }

    #[test]
    fn test_status_user_quit_serialises() {
        let ev = SseEvent::user_quit();
        let (event_type, data) = ev.to_sse_parts();
        assert_eq!(event_type, "status");
        let json: serde_json::Value = serde_json::from_str(&data).unwrap();
        assert_eq!(json["state"], "user_quit");
        assert_eq!(json["code"], serde_json::Value::Null);
    }
}
