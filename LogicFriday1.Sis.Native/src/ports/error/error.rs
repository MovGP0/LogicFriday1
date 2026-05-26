use std::sync::{Mutex, OnceLock};

const INITIAL_ERROR_CAPACITY: usize = 100;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ErrorBuffer
{
    message: String,
}

impl ErrorBuffer
{
    pub fn new() -> Self
    {
        Self::with_capacity(INITIAL_ERROR_CAPACITY)
    }

    pub fn with_capacity(capacity: usize) -> Self
    {
        Self
        {
            message: String::with_capacity(capacity),
        }
    }

    pub fn append(&mut self, message: impl AsRef<str>)
    {
        self.message.push_str(message.as_ref());
    }

    pub fn as_str(&self) -> &str
    {
        &self.message
    }

    pub fn len(&self) -> usize
    {
        self.message.len()
    }

    pub fn is_empty(&self) -> bool
    {
        self.message.is_empty()
    }

    pub fn clear(&mut self)
    {
        self.message.clear();
    }

    pub fn into_string(self) -> String
    {
        self.message
    }
}

#[derive(Debug, Default)]
struct GlobalErrorState
{
    buffer: Option<ErrorBuffer>,
}

impl GlobalErrorState
{
    fn init(&mut self)
    {
        self.buffer = Some(ErrorBuffer::new());
    }

    fn append(&mut self, message: impl AsRef<str>)
    {
        let buffer = self.buffer.get_or_insert_with(ErrorBuffer::new);
        buffer.append(message);
    }

    fn string(&self) -> Option<String>
    {
        self.buffer
            .as_ref()
            .map(|buffer| buffer.as_str().to_owned())
    }

    fn cleanup(&mut self)
    {
        self.buffer = None;
    }
}

fn global_state() -> &'static Mutex<GlobalErrorState>
{
    static STATE: OnceLock<Mutex<GlobalErrorState>> = OnceLock::new();

    STATE.get_or_init(|| Mutex::new(GlobalErrorState::default()))
}

pub fn error_init()
{
    let mut state = global_state()
        .lock()
        .expect("SIS error buffer lock was poisoned");
    state.init();
}

pub fn error_append(message: impl AsRef<str>)
{
    let mut state = global_state()
        .lock()
        .expect("SIS error buffer lock was poisoned");
    state.append(message);
}

pub fn error_string() -> Option<String>
{
    let state = global_state()
        .lock()
        .expect("SIS error buffer lock was poisoned");
    state.string()
}

pub fn error_cleanup()
{
    let mut state = global_state()
        .lock()
        .expect("SIS error buffer lock was poisoned");
    state.cleanup();
}

#[cfg(test)]
mod tests
{
    use super::*;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn new_error_buffer_starts_empty()
    {
        let buffer = ErrorBuffer::new();

        assert_eq!("", buffer.as_str());
        assert_eq!(0, buffer.len());
        assert!(buffer.is_empty());
    }

    #[test]
    fn error_buffer_appends_in_order_and_grows()
    {
        let mut buffer = ErrorBuffer::with_capacity(1);
        let long_message = "x".repeat(256);

        buffer.append("prefix: ");
        buffer.append(&long_message);

        assert_eq!(format!("prefix: {long_message}"), buffer.as_str());
        assert_eq!(264, buffer.len());
    }

    #[test]
    fn error_buffer_clear_keeps_buffer_initialized()
    {
        let mut buffer = ErrorBuffer::new();

        buffer.append("error");
        buffer.clear();

        assert_eq!(Some("".to_owned()), Some(buffer.into_string()));
    }

    #[test]
    fn global_error_lifecycle_matches_sis_usage()
    {
        let _guard = TEST_LOCK
            .lock()
            .expect("test lock was poisoned");

        error_cleanup();
        assert_eq!(None, error_string());

        error_init();
        assert_eq!(Some("".to_owned()), error_string());

        error_append("first");
        error_append(" second");
        assert_eq!(Some("first second".to_owned()), error_string());

        error_init();
        assert_eq!(Some("".to_owned()), error_string());

        error_cleanup();
        assert_eq!(None, error_string());
    }

    #[test]
    fn global_append_initializes_missing_buffer()
    {
        let _guard = TEST_LOCK
            .lock()
            .expect("test lock was poisoned");

        error_cleanup();
        error_append("late");

        assert_eq!(Some("late".to_owned()), error_string());
    }
}
