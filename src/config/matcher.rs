use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
pub struct ContextMatcher {
    #[serde(default, alias = "process_name")]
    pub process: Option<String>,
    #[serde(default)]
    pub window_class: Option<String>,
    #[serde(default)]
    pub window_title: Option<String>,
    #[serde(default)]
    pub window_title_contains: Option<String>,
    #[serde(default)]
    pub clipboard_has_text: Option<bool>,
    #[serde(default)]
    pub clipboard_has_files: Option<bool>,
}

impl ContextMatcher {
    /// Specificity ranking score for resolving highest-priority profile (Phase 7).
    pub fn specificity_score(&self) -> u32 {
        let mut score = 0;
        if self.process.is_some() { score += 10; }
        if self.window_class.is_some() { score += 10; }
        if self.window_title.is_some() { score += 20; }
        if self.window_title_contains.is_some() { score += 15; }
        if self.clipboard_has_text.is_some() { score += 5; }
        if self.clipboard_has_files.is_some() { score += 5; }
        score
    }

    /// Matches against full InvocationContext.
    pub fn matches_invocation(&self, inv: &crate::context::InvocationContext) -> bool {
        let ctx = crate::context::WindowContext::from(inv);
        if !self.matches(&ctx) {
            return false;
        }

        if let Some(want_text) = self.clipboard_has_text {
            if inv.clipboard.has_text != want_text {
                return false;
            }
        }

        if let Some(want_files) = self.clipboard_has_files {
            if inv.clipboard.has_files != want_files {
                return false;
            }
        }

        true
    }

    /// Returns true if the window context matches all specified non-empty criteria.
    pub fn matches(&self, ctx: &crate::context::WindowContext) -> bool {
        if let Some(target_proc) = &self.process {
            if !target_proc.is_empty() {
                let p_clean = target_proc.trim().to_lowercase();
                let actual = ctx.process_name.trim().to_lowercase();
                let matches_proc = if p_clean.ends_with(".exe") {
                    actual == p_clean || actual.ends_with(&format!("\\{}", p_clean))
                } else {
                    actual == p_clean || actual == format!("{}.exe", p_clean) || actual.contains(&p_clean)
                };
                if !matches_proc {
                    return false;
                }
            }
        }

        if let Some(target_class) = &self.window_class {
            if !target_class.is_empty() && !ctx.window_class.eq_ignore_ascii_case(target_class.trim()) {
                return false;
            }
        }

        if let Some(target_title) = &self.window_title {
            if !target_title.is_empty() {
                let t_clean = target_title.to_lowercase();
                if !ctx.window_title.to_lowercase().contains(&t_clean) {
                    return false;
                }
            }
        }

        if let Some(contains_str) = &self.window_title_contains {
            if !contains_str.is_empty() {
                let c_clean = contains_str.to_lowercase();
                if !ctx.window_title.to_lowercase().contains(&c_clean) {
                    return false;
                }
            }
        }

        // If at least one criteria was defined
        self.process.is_some()
            || self.window_class.is_some()
            || self.window_title.is_some()
            || self.window_title_contains.is_some()
            || self.clipboard_has_text.is_some()
            || self.clipboard_has_files.is_some()
    }
}
