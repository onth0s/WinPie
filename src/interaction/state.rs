use crate::geometry::{evaluate_commit, evaluate_hover, GeometryConfig, Point, Resolution, Sector};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Idle,
    Active {
        anchor: Point,
        hover: Option<Sector>,
    },
    WaitRelease,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionEvent {
    WinEscDown(Point),
    MouseMove(Point),
    LButtonDown(Point, bool /* keys_held */),
    RButtonDown(Point, bool /* keys_held */),
    WinUp(Point, bool /* keys_held */),
    AllKeysUp,
    FatalError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InteractionEffect {
    None,
    Activated { anchor: Point },
    HoverChanged { from: Option<Sector>, to: Option<Sector> },
    Committed(Sector),
    Cancelled,
    Rearmed,
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

    pub fn is_waiting_release(&self) -> bool {
        matches!(self.state, State::WaitRelease)
    }

    pub fn active_anchor(&self) -> Option<Point> {
        match self.state {
            State::Active { anchor, .. } => Some(anchor),
            _ => None,
        }
    }

    pub fn hover_selection(&self) -> Option<Sector> {
        match self.state {
            State::Active { hover, .. } => hover,
            _ => None,
        }
    }

    /// Pure transition function adhering to STATE_MACHINE.yaml
    pub fn transition(&mut self, event: InteractionEvent) -> InteractionEffect {
        match (self.state, event) {
            (State::Idle, InteractionEvent::WinEscDown(anchor)) => {
                self.state = State::Active {
                    anchor,
                    hover: None,
                };
                InteractionEffect::Activated { anchor }
            }

            // In WaitRelease or Active: ignore activation attempts
            (State::WaitRelease, InteractionEvent::WinEscDown(_)) => InteractionEffect::None,
            (State::Active { .. }, InteractionEvent::WinEscDown(_)) => InteractionEffect::None,

            // All keys up re-arms to Idle
            (State::WaitRelease, InteractionEvent::AllKeysUp) => {
                self.state = State::Idle;
                InteractionEffect::Rearmed
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

            (State::Active { anchor, .. }, InteractionEvent::LButtonDown(cursor, keys_held)) => {
                let resolution = evaluate_commit(anchor, cursor, &self.config);
                self.state = if keys_held { State::WaitRelease } else { State::Idle };
                match resolution {
                    Resolution::Commit(sector) => InteractionEffect::Committed(sector),
                    Resolution::Cancel => InteractionEffect::Cancelled,
                }
            }

            // On Win release: if cursor is in valid bounds, commit; otherwise cancel
            (State::Active { anchor, .. }, InteractionEvent::WinUp(cursor, keys_held)) => {
                let resolution = evaluate_commit(anchor, cursor, &self.config);
                self.state = if keys_held { State::WaitRelease } else { State::Idle };
                match resolution {
                    Resolution::Commit(sector) => InteractionEffect::Committed(sector),
                    Resolution::Cancel => InteractionEffect::Cancelled,
                }
            }

            (State::Active { .. }, InteractionEvent::RButtonDown(_, keys_held)) => {
                self.state = if keys_held { State::WaitRelease } else { State::Idle };
                InteractionEffect::Cancelled
            }

            (State::Active { .. }, InteractionEvent::FatalError) => {
                self.state = State::Idle;
                InteractionEffect::Cancelled
            }

            // Unhandled events while Idle or WaitRelease produce no effect
            _ => InteractionEffect::None,
        }
    }
}
