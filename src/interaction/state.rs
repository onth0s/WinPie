use crate::geometry::{evaluate_commit, evaluate_hover, GeometryConfig, MouseResolution, Point, Sector};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Idle,
    Active {
        anchor: Point,
        hover: Option<Sector>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionEvent {
    WinEscDown(Point),
    MouseMove(Point),
    LButtonDown(Point),
    RButtonDown(Point),
    FatalError,
    // Note: Key releases (WinUp, EscUp) produce no state change
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InteractionEffect {
    None,
    Activated { anchor: Point },
    HoverChanged { from: Option<Sector>, to: Option<Sector> },
    Committed(Sector),
    Cancelled,
    NoOpInDeadzone,
}

#[derive(Debug, Clone)]
pub struct InteractionFsm {
    pub state: State,
    pub config: GeometryConfig,
}

impl InteractionFsm {
    pub fn new(config: GeometryConfig) -> Self {
        Self {
            state: State::Idle,
            config,
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self.state, State::Active { .. })
    }

    pub fn active_anchor(&self) -> Option<Point> {
        match self.state {
            State::Active { anchor, .. } => Some(anchor),
            State::Idle => None,
        }
    }

    pub fn hover_selection(&self) -> Option<Sector> {
        match self.state {
            State::Active { hover, .. } => hover,
            State::Idle => None,
        }
    }

    /// Pure transition function following STATE_MACHINE.yaml
    pub fn transition(&mut self, event: InteractionEvent) -> InteractionEffect {
        match (self.state, event) {
            (State::Idle, InteractionEvent::WinEscDown(anchor)) => {
                self.state = State::Active {
                    anchor,
                    hover: None,
                };
                InteractionEffect::Activated { anchor }
            }

            // Already active: WinEscDown does not reactivate or create secondary wheels
            (State::Active { .. }, InteractionEvent::WinEscDown(_)) => {
                InteractionEffect::None
            }

            (State::Active { anchor, hover }, InteractionEvent::MouseMove(cursor)) => {
                let new_hover = evaluate_hover(anchor, cursor, &self.config);
                if new_hover != hover {
                    self.state = State::Active {
                        anchor,
                        hover: new_hover,
                    };
                    InteractionEffect::HoverChanged {
                        from: hover,
                        to: new_hover,
                    }
                } else {
                    InteractionEffect::None
                }
            }

            (State::Active { anchor, .. }, InteractionEvent::LButtonDown(cursor)) => {
                let resolution = evaluate_commit(anchor, cursor, &self.config);
                match resolution {
                    MouseResolution::Commit(sector) => {
                        self.state = State::Idle;
                        InteractionEffect::Committed(sector)
                    }
                    MouseResolution::NoOp => {
                        // Remains active!
                        InteractionEffect::NoOpInDeadzone
                    }
                    MouseResolution::Cancel => {
                        self.state = State::Idle;
                        InteractionEffect::Cancelled
                    }
                }
            }

            (State::Active { .. }, InteractionEvent::RButtonDown(_)) => {
                self.state = State::Idle;
                InteractionEffect::Cancelled
            }

            (State::Active { .. }, InteractionEvent::FatalError) => {
                self.state = State::Idle;
                InteractionEffect::Cancelled
            }

            // Unhandled events while Idle produce no effect
            (State::Idle, _) => InteractionEffect::None,
        }
    }
}
