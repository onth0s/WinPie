use crate::config::{MenuDefinition, MenuItem};
use crate::geometry::{evaluate_commit, evaluate_hover, GeometryConfig, Point, Resolution, Sector};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    Idle,
    Active {
        anchor: Point,
        hover: Option<Sector>,
    },
    ModalMenu {
        anchor: Point,
        nav_stack: Vec<MenuDefinition>,
        highlighted_key: Option<char>,
    },
    WaitRelease,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InteractionEvent {
    WinEscDown(Point),
    MouseMove(Point),
    LButtonDown(Point, bool /* keys_held */),
    RButtonDown(Point, bool /* keys_held */),
    WinUp(Point, bool /* keys_held */),
    AllKeysUp,

    // Modal Menu events
    MenuSpawn { anchor: Point, menu: MenuDefinition },
    MenuKeyDown(char),
    MenuKeyUp(char, bool /* keys_held */),
    MenuHover(Option<char>),
    MenuSelect(char, bool /* keys_held */),
    MenuTab { shift: bool },
    MenuEsc(bool /* keys_held */),

    FatalError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InteractionEffect {
    None,
    Activated { anchor: Point },
    HoverChanged { from: Option<Sector>, to: Option<Sector> },
    Committed(Sector),

    // Modal Menu effects
    MenuSpawned { anchor: Point, menu: MenuDefinition },
    MenuHighlightChanged { key: Option<char>, item: Option<MenuItem> },
    MenuDrillDown { menu: MenuDefinition },
    MenuBacktracked { current_menu: MenuDefinition },
    MenuResetToRoot { current_menu: MenuDefinition },
    MenuExecuted { command: String, label: String },

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

    pub fn is_modal_menu(&self) -> bool {
        matches!(self.state, State::ModalMenu { .. })
    }

    pub fn is_waiting_release(&self) -> bool {
        matches!(self.state, State::WaitRelease)
    }

    pub fn active_anchor(&self) -> Option<Point> {
        match self.state {
            State::Active { anchor, .. } => Some(anchor),
            State::ModalMenu { anchor, .. } => Some(anchor),
            _ => None,
        }
    }

    pub fn hover_selection(&self) -> Option<Sector> {
        match self.state {
            State::Active { hover, .. } => hover,
            _ => None,
        }
    }

    pub fn current_menu(&self) -> Option<&MenuDefinition> {
        match &self.state {
            State::ModalMenu { nav_stack, .. } => nav_stack.last(),
            _ => None,
        }
    }

    pub fn menu_nav_stack(&self) -> Option<&[MenuDefinition]> {
        match &self.state {
            State::ModalMenu { nav_stack, .. } => Some(nav_stack.as_slice()),
            _ => None,
        }
    }

    pub fn highlighted_menu_key(&self) -> Option<char> {
        match &self.state {
            State::ModalMenu { highlighted_key, .. } => *highlighted_key,
            _ => None,
        }
    }

    /// Pure transition function adhering to STATE_MACHINE.yaml & Submenu Contracts
    pub fn transition(&mut self, event: InteractionEvent) -> InteractionEffect {
        match (&mut self.state, event) {
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
            (State::ModalMenu { .. }, InteractionEvent::WinEscDown(_)) => InteractionEffect::None,

            // All keys up re-arms to Idle
            (State::WaitRelease, InteractionEvent::AllKeysUp) => {
                self.state = State::Idle;
                InteractionEffect::Rearmed
            }

            (State::Active { anchor, hover }, InteractionEvent::MouseMove(cursor)) => {
                let anchor_pt = *anchor;
                let current_hover = *hover;
                let new_hover = evaluate_hover(anchor_pt, cursor, &self.config);
                if new_hover != current_hover {
                    self.state = State::Active {
                        anchor: anchor_pt,
                        hover: new_hover,
                    };
                    InteractionEffect::HoverChanged {
                        from: current_hover,
                        to: new_hover,
                    }
                } else {
                    InteractionEffect::None
                }
            }

            (State::Active { anchor, .. }, InteractionEvent::LButtonDown(cursor, keys_held)) => {
                let anchor_pt = *anchor;
                let resolution = evaluate_commit(anchor_pt, cursor, &self.config);
                match resolution {
                    Resolution::Commit(sector) => {
                        // Caller checks whether sector spawns a menu via MenuSpawn event
                        self.state = if keys_held { State::WaitRelease } else { State::Idle };
                        InteractionEffect::Committed(sector)
                    }
                    Resolution::Cancel => {
                        self.state = if keys_held { State::WaitRelease } else { State::Idle };
                        InteractionEffect::Cancelled
                    }
                }
            }

            // On Win release: if cursor is in valid bounds, commit; otherwise cancel
            (State::Active { anchor, .. }, InteractionEvent::WinUp(cursor, keys_held)) => {
                let anchor_pt = *anchor;
                let resolution = evaluate_commit(anchor_pt, cursor, &self.config);
                match resolution {
                    Resolution::Commit(sector) => {
                        self.state = if keys_held { State::WaitRelease } else { State::Idle };
                        InteractionEffect::Committed(sector)
                    }
                    Resolution::Cancel => {
                        self.state = if keys_held { State::WaitRelease } else { State::Idle };
                        InteractionEffect::Cancelled
                    }
                }
            }

            (State::Active { .. }, InteractionEvent::RButtonDown(_, keys_held)) => {
                self.state = if keys_held { State::WaitRelease } else { State::Idle };
                InteractionEffect::Cancelled
            }

            // Spawn modal menu
            (_, InteractionEvent::MenuSpawn { anchor, menu }) => {
                self.state = State::ModalMenu {
                    anchor,
                    nav_stack: vec![menu.clone()],
                    highlighted_key: None,
                };
                InteractionEffect::MenuSpawned { anchor, menu }
            }

            // Modal Menu KeyDown (preview / highlight)
            (State::ModalMenu { anchor, nav_stack, highlighted_key }, InteractionEvent::MenuKeyDown(ch)) => {
                let key = ch.to_ascii_lowercase();
                if let Some(current) = nav_stack.last() {
                    if let Some(item) = current.items.get(&key) {
                        if *highlighted_key != Some(key) {
                            *highlighted_key = Some(key);
                            return InteractionEffect::MenuHighlightChanged {
                                key: Some(key),
                                item: Some(item.clone()),
                            };
                        }
                    }
                }
                let _ = anchor;
                InteractionEffect::None
            }

            // Modal Menu Mouse Hover (row hover preview / highlight / clear)
            (State::ModalMenu { nav_stack, highlighted_key, .. }, InteractionEvent::MenuHover(opt_key)) => {
                let lower_opt = opt_key.map(|c| c.to_ascii_lowercase());
                if *highlighted_key != lower_opt {
                    *highlighted_key = lower_opt;
                    let item = lower_opt.and_then(|k| nav_stack.last()?.items.get(&k).cloned());
                    InteractionEffect::MenuHighlightChanged {
                        key: lower_opt,
                        item,
                    }
                } else {
                    InteractionEffect::None
                }
            }

            // Modal Menu Direct Selection (mouse click or direct trigger)
            (State::ModalMenu { anchor, nav_stack, highlighted_key }, InteractionEvent::MenuSelect(ch, keys_held)) => {
                let key = ch.to_ascii_lowercase();
                let anchor_pt = *anchor;
                if let Some(current) = nav_stack.last() {
                    if let Some(item) = current.items.get(&key) {
                        if let Some(child_menu) = &item.menu {
                            let child = child_menu.clone();
                            nav_stack.push(child.clone());
                            *highlighted_key = None;
                            return InteractionEffect::MenuDrillDown { menu: child };
                        } else if let Some(cmd) = &item.command {
                            let command = cmd.clone();
                            let label = item.label.clone();
                            self.state = if keys_held { State::WaitRelease } else { State::Idle };
                            return InteractionEffect::MenuExecuted { command, label };
                        }
                    }
                }
                let _ = anchor_pt;
                InteractionEffect::None
            }

            // Modal Menu KeyUp (commit selection / drill-down / leaf action)
            (State::ModalMenu { anchor, nav_stack, highlighted_key }, InteractionEvent::MenuKeyUp(ch, keys_held)) => {
                let key = ch.to_ascii_lowercase();
                let anchor_pt = *anchor;
                if *highlighted_key == Some(key) {
                    if let Some(current) = nav_stack.last() {
                        if let Some(item) = current.items.get(&key) {
                            if let Some(child_menu) = &item.menu {
                                let child = child_menu.clone();
                                nav_stack.push(child.clone());
                                *highlighted_key = None;
                                return InteractionEffect::MenuDrillDown { menu: child };
                            } else if let Some(cmd) = &item.command {
                                let command = cmd.clone();
                                let label = item.label.clone();
                                self.state = if keys_held { State::WaitRelease } else { State::Idle };
                                return InteractionEffect::MenuExecuted { command, label };
                            }
                        }
                    }
                    *highlighted_key = None;
                    return InteractionEffect::MenuHighlightChanged {
                        key: None,
                        item: None,
                    };
                }
                let _ = anchor_pt;
                InteractionEffect::None
            }

            // Modal Menu Tab / Shift+Tab Navigation
            (State::ModalMenu { nav_stack, highlighted_key, .. }, InteractionEvent::MenuTab { shift }) => {
                *highlighted_key = None;
                if shift {
                    // Shift+Tab -> Backtrack one level
                    if nav_stack.len() > 1 {
                        nav_stack.pop();
                        let current = nav_stack.last().unwrap().clone();
                        InteractionEffect::MenuBacktracked { current_menu: current }
                    } else {
                        InteractionEffect::None
                    }
                } else {
                    // Tab -> Loop back to root menu in stack
                    if nav_stack.len() > 1 {
                        nav_stack.truncate(1);
                        let current = nav_stack[0].clone();
                        InteractionEffect::MenuResetToRoot { current_menu: current }
                    } else {
                        InteractionEffect::None
                    }
                }
            }

            // Modal Menu Escape / Right-Click (unconditional cancel)
            (State::ModalMenu { .. }, InteractionEvent::MenuEsc(keys_held))
            | (State::ModalMenu { .. }, InteractionEvent::RButtonDown(_, keys_held)) => {
                self.state = if keys_held { State::WaitRelease } else { State::Idle };
                InteractionEffect::Cancelled
            }

            (State::Active { .. } | State::ModalMenu { .. }, InteractionEvent::FatalError) => {
                self.state = State::Idle;
                InteractionEffect::Cancelled
            }

            // Unhandled events produce no effect
            _ => InteractionEffect::None,
        }
    }
}
