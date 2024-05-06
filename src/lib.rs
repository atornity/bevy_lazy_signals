use bevy::prelude::*;

mod arcane_wizardry;

pub mod commands;

pub mod factory;

pub mod reference_impl;
use reference_impl::*;

pub mod signals;
use signals::*;

pub mod prelude {
    pub use crate::{ factory::*, reference_impl::*, SignalsPlugin, SignalsResource };
}

/// A reference implementation follows. A consumer can replace any or all pieces and provide a new plugin.
///
// Convenience typedefs.
pub type ImmutableBool = LazyImmutable<bool>;
pub type ImmutableInt = LazyImmutable<u32>;
pub type ImmutableFloat = LazyImmutable<f64>;
pub type ImmutableStr = LazyImmutable<&'static str>;

/// Plugin to initialize the resource and system schedule.
pub struct SignalsPlugin;

impl Plugin for SignalsPlugin {
    fn build(&self, app: &mut App) {
        // NOTE: the user application will need to register each custom Immutable<T> for reflection

        // add the systems to process signals, memos, and effects
        app.init_resource::<SignalsResource>()
            // custom Immutable types must be manually registered
            .register_type::<ImmutableBool>()
            .register_type::<ImmutableInt>()
            .register_type::<ImmutableFloat>()
            .register_type::<ImmutableStr>()
            //.register_component_as::<dyn LazyMergeable, LazyImmutable<>>()
            .add_systems(
                PostUpdate, // could be Preupdate or whatever else (probably not Update)
                // before() ensures each system's changes will be applied before the next is called
                (
                    init_subscribers.before(send_signals),
                    send_signals.before(calculate_memos),
                    calculate_memos.before(apply_deferred_effects),
                    apply_deferred_effects,
                )
            );
    }
}

/// Shared reactive context resource.
#[derive(Resource)]
pub struct SignalsResource {
    /// Tracks the currently running iteration (immutable once the iteration starts).
    pub running: EntitySet,

    /// Tracks what will run after the end of the current iteration.
    pub next_running: EntitySet,

    /// Tracks which memos have already been added to a running set.
    pub processed: EntitySet,

    /// Tracks which Signals and Memos actually have changed data.
    pub changed: EntitySet,

    /// Tracks Effects to evaluate for processing.
    pub deferred: EntitySet,

    /// Tracks Effects that are still running and should not be re-triggered.
    pub effects: EntitySet,
}

/// This is a singleton that represents the "global state." It is used during internal updates.
impl SignalsResource {
    /// Call this at the start of each run to make sure everything is fresh.
    fn init(&mut self) {
        self.running.clear();
        self.next_running.clear();
        self.processed.clear();
        self.changed.clear();
        self.deferred.clear();
        // self.effects.clear(); // don't clear this, need.. to remember... what is going on
    }

    // if there is anext_running set, move it into the running set and empty it
    pub fn merge_running(&mut self) -> bool {
        if self.next_running.is_empty() {
            false
        } else {
            for index in self.next_running.indices() {
                self.running.insert(index, ());
            }
            self.next_running.clear();
            true
        }
    }
}

impl Default for SignalsResource {
    fn default() -> Self {
        Self {
            running: empty_set(),
            next_running: empty_set(),
            processed: empty_set(),
            changed: empty_set(),
            deferred: empty_set(),
            effects: empty_set(),
        }
    }
}
