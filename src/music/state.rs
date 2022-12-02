use songbird::input::Metadata;

#[derive(Debug)]
pub struct MusicState {
    pub metadata: Option<Box<Metadata>>,
    pub queue_names: Vec<QueueItem>,
    pub looping: bool,
    pub shuffling: bool
}

#[derive(Debug)]
pub struct QueueItem {
    pub title: String,
    pub index: usize
}

#[derive(PartialOrd, PartialEq, Eq)]
pub enum QueueAction {
    HardNext,
    SoftNext,
    Previous,
    SelectedNext,
}

impl From<&str> for QueueAction {
    fn from(value: &str) -> Self {
        match value {
            "next" => QueueAction::HardNext,
            "prev" => QueueAction::Previous,
            _ => QueueAction::SoftNext
        }
    }
}