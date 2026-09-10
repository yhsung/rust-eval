#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cell {
    pub pci: u16,
    pub rsrp_dbm: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimerId {
    T300,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Timer {
    pub id: TimerId,
    pub is_running: bool,
}

impl Timer {
    pub fn start(id: TimerId) -> Self {
        Self {
            id,
            is_running: true,
        }
    }

    pub fn stop(mut self) -> Self {
        self.is_running = false;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Idle {
    pub selected_cell: Cell,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Connecting {
    pub selected_cell: Cell,
    pub t300: Timer,
    pub transaction_id: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Connected {
    pub serving_cell: Cell,
    pub c_rnti: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RrcState {
    Idle(Idle),
    Connecting(Connecting),
    Connected(Connected),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateKind {
    Idle,
    Connecting,
    Connected,
}

impl RrcState {
    pub fn kind(&self) -> StateKind {
        match self {
            Self::Idle(_) => StateKind::Idle,
            Self::Connecting(_) => StateKind::Connecting,
            Self::Connected(_) => StateKind::Connected,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RlfCause {
    OutOfSync,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    StartConnection { transaction_id: u8 },
    SetupComplete { c_rnti: u16 },
    T300Expiry,
    Release,
    RadioLinkFailure { cause: RlfCause, serving_cell: Cell },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    StartConnection,
    SetupComplete,
    T300Expiry,
    Release,
    RadioLinkFailure,
}

impl Event {
    pub fn kind(&self) -> EventKind {
        match self {
            Self::StartConnection { .. } => EventKind::StartConnection,
            Self::SetupComplete { .. } => EventKind::SetupComplete,
            Self::T300Expiry => EventKind::T300Expiry,
            Self::Release => EventKind::Release,
            Self::RadioLinkFailure { .. } => EventKind::RadioLinkFailure,
        }
    }
}
