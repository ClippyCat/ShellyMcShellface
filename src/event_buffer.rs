use std::sync::Mutex;
use crate::types::SseEvent;

pub struct EventBuffer {
    events: Mutex<Vec<SseEvent>>,
}

impl EventBuffer {
    pub fn new() -> Self {
        EventBuffer { events: Mutex::new(Vec::new()) }
    }

    pub fn push(&self, event: SseEvent) {
        self.events.lock().unwrap().push(event);
    }

    /// Returns a snapshot clone of all events in insertion order.
    pub fn snapshot(&self) -> Vec<SseEvent> {
        self.events.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SseEvent;

    #[test]
    fn test_new_buffer_is_empty() {
        let buf = EventBuffer::new();
        assert_eq!(buf.snapshot().len(), 0);
    }

    #[test]
    fn test_push_and_snapshot_preserves_order() {
        let buf = EventBuffer::new();
        buf.push(SseEvent::connected());
        buf.push(SseEvent::output("line1\n"));
        buf.push(SseEvent::input("ls"));
        buf.push(SseEvent::output("file.txt\n"));
        let snap = buf.snapshot();
        assert_eq!(snap.len(), 4);
        assert!(matches!(snap[0], SseEvent::Status { .. }));
        assert!(matches!(snap[1], SseEvent::Output { .. }));
        assert!(matches!(snap[2], SseEvent::Input { .. }));
        assert!(matches!(snap[3], SseEvent::Output { .. }));
    }

    #[test]
    fn test_snapshot_is_a_clone() {
        let buf = EventBuffer::new();
        buf.push(SseEvent::output("a\n"));
        let snap1 = buf.snapshot();
        buf.push(SseEvent::output("b\n"));
        let snap2 = buf.snapshot();
        assert_eq!(snap1.len(), 1);
        assert_eq!(snap2.len(), 2);
    }
}
