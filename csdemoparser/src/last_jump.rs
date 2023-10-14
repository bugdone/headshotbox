use demo_format::Tick;
use std::collections::HashMap;

#[derive(Default)]
pub(crate) struct LastJump<U> {
    /// Maps player user_id to last jump tick.
    jumped_last: HashMap<U, Tick>,
}

impl<U: Eq + PartialEq + std::hash::Hash> LastJump<U> {
    pub(crate) fn record_jump(&mut self, user_id: U, tick: Tick) {
        self.jumped_last.insert(user_id, tick);
    }

    pub(crate) fn ticks_since_last_jump(
        &self,
        user_id: U,
        tick: Tick,
        tick_interval: f32,
    ) -> Option<Tick> {
        let &jumped_last = self.jumped_last.get(&user_id)?;
        const JUMP_DURATION: f64 = 0.75;
        if tick_interval > 0_f32
            && jumped_last as f64 >= tick as f64 - JUMP_DURATION / tick_interval as f64
        {
            Some(tick - jumped_last)
        } else {
            None
        }
    }
}
