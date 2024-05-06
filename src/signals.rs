use bevy::{
    ecs::{ component::ComponentId, storage::SparseSet },
    prelude::*,
    reflect::DynamicTuple,
};

use thiserror::Error;

/// # Signals framework
/// ## Types
/// Result type for handling error conditions in consumer code.
pub type SignalsResult<T> = Result<T, SignalsError>;

/// ## Enums
/// Read error.
#[derive(Error, Debug)]
pub enum SignalsError {
    #[error("Error reading signal {0}")] ReadError(Entity),
    #[error["Signal does not exist"]] NoSignalError,
}

// ## Traits
/// An item of data backed by a Bevy entity with a set of subscribers.
/// Additional methods in UntypedObservable would be here but you can't have generic trait objects.
pub trait Immutable: Send + Sync + 'static {
    type DataType: Copy + PartialEq + Send + Sync + 'static;

    /// Called by a consumer to provide a new value for the lazy update system to merge.
    fn merge_next(&mut self, next: Self::DataType);

    /// Get the current value.
    fn read(&self) -> Self::DataType;

    /// Get the current value, subscribing an entity if provided (mostly used internally).
    fn value(&mut self, caller: Entity) -> Self::DataType;
}

#[reflect_trait]
pub trait UntypedObservable {
    /// Called by a lazy update system to apply the new value of a signal.
    /// This is a main thing to implement if you're trying to use reflection.
    /// The ref impl uses this to update the Immutable values without knowing the type.
    /// These are also part of sending a Signal.
    ///
    /// Get the list of subscriber Entities that may need notification.
    fn get_subscribers(&self) -> Vec<Entity>;

    /// This method merges the next_value and returns get_subscribers().
    fn merge(&mut self) -> Vec<Entity>;

    /// Called by a lazy update system to refresh the subscribers.
    fn merge_subscribers(&mut self);

    /// Called by an Effect or Memo indirectly by reading the current value.
    fn subscribe(&mut self, entity: Entity);
}

/// A Propagator function aggregates (merges) data from multiple cells to store in a bound cell.
/// Compared to the MIT model, these Propagators pull data into a cell they are bound to.
/// MIT Propagators are conceptually more independent and closer to a push-based flow.
/// This Propagator merges the values of cells denoted by the entity vector into the target entity.
/// It should call value instead of read to make sure it is re-subscribed to its sources!
/// If the target entity is not supplied, the function is assumed to execute side effects only.
pub trait PropagatorFn: Send + Sync + FnMut(DynamicTuple) -> DynamicTuple {}
impl<T: Send + Sync + FnMut(DynamicTuple) -> DynamicTuple> PropagatorFn for T {}

// TODO provide a to_effect to allow a propagator to be used as an effect

/// This is the same basic thing but this fn just runs side-effects so no value is returned
pub trait EffectFn: Send + Sync + FnMut(DynamicTuple) -> SignalsResult<()> {}
impl<T: Send + Sync + FnMut(DynamicTuple) -> SignalsResult<()>> EffectFn for T {}

/// ## Component Structs
/// An Immutable is known as a cell in a propagator network. It may also be referred to as state.
/// Using the label Immutable because Cell and State often mean other things.
/// Mutable is used by futures-signals for the same data-wrapping purpose, but in our case, the
/// cells are mutated by sending a signal explicitly (i.e. adding a SendSignal component).
///
/// Some convenience types provided: ImmutableBool, ImmutableInt, ImmutableFloat, ImmutableString.
///
/// The subscriber set is built from the sources/triggers of computed memos and effects, so it does
/// not have to be serialized, which is good because the SparseSet doesn't seem to do Reflect.
///
/// This Immutable is lazy. Other forms are left as an exercise for the reader.
#[derive(Component, Reflect)]
#[reflect(Component, UntypedObservable)]
pub struct LazyImmutable<T: Copy + PartialEq + Send + Sync + 'static> {
    data: T,
    next_value: Option<T>,
    #[reflect(ignore)]
    subscribers: EntitySet,
    #[reflect(ignore)]
    next_subscribers: EntitySet,
}

impl<T: Copy + PartialEq + Send + Sync + 'static> LazyImmutable<T> {
    pub fn new(data: T) -> Self {
        Self { data, next_value: None, subscribers: empty_set(), next_subscribers: empty_set() }
    }
}

impl<T: Copy + PartialEq + Send + Sync + 'static> Immutable for LazyImmutable<T> {
    type DataType = T;

    // TODO add a get that returns a result after safely calling read

    // TODO add a get_value that returns a result after safely calling value

    fn merge_next(&mut self, next: T) {
        self.next_value = Some(next);
    }

    fn read(&self) -> Self::DataType {
        self.data
    }

    fn value(&mut self, caller: Entity) -> Self::DataType {
        self.subscribe(caller);
        self.read()
    }
}

impl<T: Copy + PartialEq + Send + Sync + 'static> UntypedObservable for LazyImmutable<T> {
    fn get_subscribers(&self) -> Vec<Entity> {
        let mut subs = Vec::<Entity>::new();

        // copy the subscribers into the output vector
        subs.extend(self.subscribers.indices());
        info!("-found subs {:?}", self.subscribers);
        subs
    }

    fn merge(&mut self) -> Vec<Entity> {
        let mut subs = Vec::<Entity>::new();

        // update the Immutable data value
        if let Some(next) = self.next_value {
            trace!("next exists");

            // only fire the rest of the process if the data actually changed
            if self.data != next {
                info!("data != next");
                self.data = next;

                // copy the subscribers into the output vector
                subs = self.get_subscribers();

                // clear the local subscriber set which will be replenished by each subscriber if
                // it calls the value method later
                self.subscribers.clear();
            }
        }
        self.next_value = None;
        subs
    }

    fn merge_subscribers(&mut self) {
        for subscriber in self.next_subscribers.iter() {
            self.subscribers.insert(*subscriber.0, ());
        }
        self.next_subscribers.clear();
    }

    fn subscribe(&mut self, entity: Entity) {
        self.next_subscribers.insert(entity, ());
    }
}

/// An ImmutableComponentId allows us to dereference a generic Immutable without knowing its type.
#[derive(Component)]
pub struct ImmutableComponentId {
    pub component_id: ComponentId,
}

/// A SendSignal component marks an Immutable cell as having a next_value.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct SendSignal;

/// A Propagator component is the aggregating propagator function and its sources/triggers list.
#[derive(Component)]
pub struct Propagator {
    pub propagator: Box<dyn PropagatorFn>,
    pub sources: Vec<Entity>,
}

/// A ComputeMemo component marks an Immutable that needs to be computed.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct ComputeMemo;

/// An effect is a Propagator endpoint that returns no value and just runs side-effects.
#[derive(Component)]
pub struct Effect {
    pub effect: Box<dyn EffectFn>,
    pub triggers: Vec<Entity>,
}

/// A DeferredEffect component marks an Effect function that needs to run.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct DeferredEffect;

/// Marks a Propagator as needing to subscribe to its dependencies.
/// This normally only happens within the framework internals on create.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct RebuildSubscribers;

/// ## Utilities

/// Set of unique Entities
pub type EntitySet = SparseSet<Entity, ()>;

/// Create an empty sparse set for storing Entities by ID.
pub fn empty_set() -> EntitySet {
    EntitySet::new()
}
