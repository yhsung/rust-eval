#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cell {
    pub pci: u16,
    pub rsrp_dbm: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimerId {
    T300,
    T301,
    T311,
    T319,
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
    pub pending_completion: Option<Completion>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Completion {
    Setup,
    Resume,
    Reestablishment,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resuming {
    pub context: Connected,
    pub t319: Timer,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectingCell {
    pub previous_cell: Cell,
    pub t311: Timer,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reestablishing {
    pub selected_cell: Cell,
    pub t301: Timer,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RrcState {
    Idle(Idle),
    Connecting(Connecting),
    Connected(Connected),
    Inactive(Connected),
    Resuming(Resuming),
    SelectingCell(SelectingCell),
    Reestablishing(Reestablishing),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateKind {
    Idle,
    Connecting,
    Connected,
    Inactive,
    Resuming,
    SelectingCell,
    Reestablishing,
}

impl RrcState {
    pub fn kind(&self) -> StateKind {
        match self {
            Self::Idle(_) => StateKind::Idle,
            Self::Connecting(_) => StateKind::Connecting,
            Self::Connected(_) => StateKind::Connected,
            Self::Inactive(_) => StateKind::Inactive,
            Self::Resuming(_) => StateKind::Resuming,
            Self::SelectingCell(_) => StateKind::SelectingCell,
            Self::Reestablishing(_) => StateKind::Reestablishing,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RlfCause {
    OutOfSync,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    RrcSetupRequest { transaction_id: u8 },
    RrcSetup { c_rnti: u16 },
    T300Expiry,
    RrcSetupComplete,
    RrcReject,
    RrcRelease { suspend: bool },
    RrcResumeRequest,
    RrcResume,
    RrcResumeComplete,
    T319Expiry,
    RrcReestablishmentRequest { cell: Cell },
    RrcReestablishment { c_rnti: u16 },
    RrcReestablishmentComplete,
    T311Expiry,
    T301Expiry,
    RadioLinkFailure { cause: RlfCause, serving_cell: Cell },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    RrcSetupRequest,
    RrcSetup,
    T300Expiry,
    RrcSetupComplete,
    RrcReject,
    RrcRelease,
    RrcResumeRequest,
    RrcResume,
    RrcResumeComplete,
    T319Expiry,
    RrcReestablishmentRequest,
    RrcReestablishment,
    RrcReestablishmentComplete,
    T311Expiry,
    T301Expiry,
    RadioLinkFailure,
}

impl Event {
    pub fn kind(&self) -> EventKind {
        match self {
            Self::RrcSetupRequest { .. } => EventKind::RrcSetupRequest,
            Self::RrcSetup { .. } => EventKind::RrcSetup,
            Self::T300Expiry => EventKind::T300Expiry,
            Self::RrcSetupComplete => EventKind::RrcSetupComplete,
            Self::RrcReject => EventKind::RrcReject,
            Self::RrcRelease { .. } => EventKind::RrcRelease,
            Self::RrcResumeRequest => EventKind::RrcResumeRequest,
            Self::RrcResume => EventKind::RrcResume,
            Self::RrcResumeComplete => EventKind::RrcResumeComplete,
            Self::T319Expiry => EventKind::T319Expiry,
            Self::RrcReestablishmentRequest { .. } => EventKind::RrcReestablishmentRequest,
            Self::RrcReestablishment { .. } => EventKind::RrcReestablishment,
            Self::RrcReestablishmentComplete => EventKind::RrcReestablishmentComplete,
            Self::T311Expiry => EventKind::T311Expiry,
            Self::T301Expiry => EventKind::T301Expiry,
            Self::RadioLinkFailure { .. } => EventKind::RadioLinkFailure,
        }
    }
}
