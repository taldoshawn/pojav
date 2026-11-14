//! Persisted drawer groups with runtime visibility and per-finger release ownership.
use super::*;
impl Settings {
    fn next_drawer_id(&self) -> u64 {
        self.custom_buttons
            .iter()
            .filter_map(|b| b.drawer_id)
            .max()
            .unwrap_or(0)
            .saturating_add(1)
    }
    pub fn add_drawer(&mut self) -> usize {
        let index = self.custom_buttons.len();
        let mut button = CustomButton::new("key.keyboard.unknown".into(), index);
        button.custom_label = "Gaveta".into();
        button.drawer_id = Some(self.next_drawer_id());
        for p in &mut button.layouts {
            p.use_goy_style = true;
            p.visible_in_menu = true;
        }
        self.custom_buttons.push(button);
        index
    }
    pub fn add_drawer_child(&mut self, index: usize) -> Option<usize> {
        let parent = self.custom_buttons.get(index)?;
        let drawer = parent.drawer_id?;
        let mut child = CustomButton::new("key.keyboard.unknown".into(), self.custom_buttons.len());
        child.custom_label = "Botão".into();
        child.parent_drawer = Some(drawer);
        child.layouts = parent.layouts;
        let count = self
            .custom_buttons
            .iter()
            .filter(|b| b.parent_drawer == Some(drawer))
            .count();
        for p in &mut child.layouts {
            p.center[0] = (p.center[0] + 0.12 * (count as f32 + 1.0)).min(0.92);
        }
        self.custom_buttons.push(child);
        Some(self.custom_buttons.len() - 1)
    }
    pub fn remove_custom_button(&mut self, index: usize) {
        if index >= self.custom_buttons.len() {
            return;
        }
        let drawer = self.custom_buttons.remove(index).drawer_id;
        if let Some(id) = drawer {
            self.custom_buttons.retain(|b| b.parent_drawer != Some(id));
        }
    }
    pub fn clone_custom_button(&mut self, index: usize) -> Option<usize> {
        let mut copy = self.custom_buttons.get(index)?.clone();
        let children: Vec<_> = copy
            .drawer_id
            .map(|id| {
                self.custom_buttons
                    .iter()
                    .filter(|b| b.parent_drawer == Some(id))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        let new_id = self.next_drawer_id();
        if copy.drawer_id.is_some() {
            copy.drawer_id = Some(new_id);
        }
        copy.custom_label = format!("{} cópia", copy.label());
        for p in &mut copy.layouts {
            p.center[1] = (p.center[1] + 0.12).min(0.92);
        }
        let new_index = self.custom_buttons.len();
        self.custom_buttons.push(copy);
        for mut child in children {
            child.parent_drawer = Some(new_id);
            for p in &mut child.layouts {
                p.center[1] = (p.center[1] + 0.12).min(0.92);
            }
            self.custom_buttons.push(child);
        }
        Some(new_index)
    }
}
impl TouchControls {
    pub(super) fn release_drawer_children(&mut self, drawer: u64) {
        let children: BTreeSet<_> = self
            .settings
            .custom_buttons
            .iter()
            .enumerate()
            .filter(|(_, b)| b.parent_drawer == Some(drawer))
            .map(|(i, _)| i)
            .collect();
        let fingers: Vec<_> = self
            .custom_swipe_btn
            .iter()
            .filter(|(_, i)| children.contains(i))
            .map(|(id, _)| *id)
            .collect();
        for id in fingers {
            self.release_custom_finger_key(id);
        }
        let tokens: BTreeSet<_> = children
            .into_iter()
            .flat_map(|i| self.settings.custom_buttons[i].active_tokens())
            .collect();
        for token in tokens {
            if self.latched_custom.remove(&token)
                && !self
                    .custom_fingers
                    .values()
                    .any(|keys| keys.contains(&token))
            {
                self.custom_events.push((token, false));
            }
        }
    }
}
