use bevy::{ prelude::*, reflect::{ DynamicTuple, GetTupleField } };

use crate::{ commands::LazySignalsCommandsExt, framework::* };

/// This is the reference user API, patterned after the TC39 proposal.

pub fn end_effect() -> LazySignalsResult<()> {
    Some(Ok(()))
}

/// Convenience function to get a field directly from a DynamicTuple.
pub fn get_field<T: LazySignalsData>(tuple: &DynamicTuple, index: usize) -> Option<&T> {
    tuple.get_field::<T>(index) // returns None if type doesn't match
}

pub fn make_effect_with<P: LazySignalsParams>(
    mut closure: Box<dyn Effect<P>>
) -> Box<dyn EffectContext> {
    Box::new(move |tuple, world| {
        info!("-running effect context with params {:?}", tuple);
        let result = closure(make_tuple::<P>(tuple), world);
        if let Some(Err(error)) = result {
            // TODO process errors
            error!("ERROR running effect: {}", error.to_string());
        }
    })
}

pub fn make_propagator_with<P: LazySignalsParams, R: LazySignalsData>(
    closure: Box<dyn Propagator<P, R>>
) -> Box<dyn PropagatorContext> {
    Box::new(move |tuple, entity, world| {
        info!("-running propagator context with params {:?}", tuple);
        let result = closure(make_tuple::<P>(tuple));
        if let Some(Err(error)) = result {
            // TODO process errors
            error!("ERROR running propagator: {}", error.to_string());
        }
        store_result(result, entity, world);
    })
}

/// Convenience function to convert DynamicTuples into a concrete type.
pub fn make_tuple<T: LazySignalsParams>(tuple: &DynamicTuple) -> T {
    <T as FromReflect>::from_reflect(tuple).unwrap()
}

/// Convenience function to store a result in an entity.
pub fn store_result<T: LazySignalsData>(data: Option<T>, entity: &Entity, world: &mut World) {
    let mut binding = world.entity_mut(*entity);
    let mut bucket = binding.get_mut::<LazyImmutable<T>>().unwrap();
    bucket.update(data.map(|data| Ok(data)));
}

/// ## Main Signal primitive factory.
/// Convenience functions for Signal creation and manipulation inspired by the TC39 proposal.
pub struct LazySignals;
impl LazySignals {
    pub fn computed<P: LazySignalsParams, R: LazySignalsData>(
        &self,
        propagator_closure: Box<dyn Propagator<P, R>>,
        sources: Vec<Entity>,
        commands: &mut Commands
    ) -> Entity {
        let entity = commands.spawn_empty().id();
        commands.create_computed::<P, R>(entity, make_propagator_with(propagator_closure), sources);
        entity
    }

    pub fn effect<P: LazySignalsParams>(
        &self,
        effect_closure: Box<dyn Effect<P>>,
        sources: Vec<Entity>,
        triggers: Vec<Entity>,
        commands: &mut Commands
    ) -> Entity {
        let entity = commands.spawn_empty().id();
        commands.create_effect::<P>(entity, make_effect_with(effect_closure), sources, triggers);
        entity
    }

    pub fn read<R: LazySignalsData>(
        &self,
        immutable: Option<Entity>,
        world: &World
    ) -> LazySignalsResult<R> {
        match immutable {
            Some(immutable) => {
                let entity = world.entity(immutable);
                match entity.get::<LazyImmutable<R>>() {
                    Some(observable) => observable.read(),

                    // TODO maybe add some kind of config option to ignore errors and return a default
                    None => Some(Err(LazySignalsError::ReadError(immutable))),
                }
            }
            None => Some(Err(LazySignalsError::NoSignalError)),
        }
    }

    pub fn send<T: LazySignalsData>(
        &self,
        signal: Option<Entity>,
        data: T,
        commands: &mut Commands
    ) {
        if let Some(signal) = signal {
            commands.send_signal::<T>(signal, data);
        }
    }

    pub fn state<T: LazySignalsData>(&self, data: T, commands: &mut Commands) -> Entity {
        let state = commands.spawn_empty().id();
        commands.create_state::<T>(state, data);
        state
    }

    pub fn trigger<T: LazySignalsData>(
        &self,
        signal: Option<Entity>,
        data: T,
        commands: &mut Commands
    ) {
        if let Some(signal) = signal {
            commands.trigger_signal::<T>(signal, data);
        }
    }

    pub fn value<R: LazySignalsData>(
        &self,
        immutable: Option<Entity>,
        caller: Entity,
        world: &mut World
    ) -> LazySignalsResult<R> {
        match immutable {
            Some(immutable) => {
                let mut entity = world.entity_mut(immutable);
                match entity.get_mut::<LazyImmutable<R>>() {
                    Some(mut observable) => { observable.value(caller) }

                    // TODO maybe add some kind of config option to ignore errors and return default
                    None => Some(Err(LazySignalsError::ReadError(immutable))),
                }
            }
            None => Some(Err(LazySignalsError::NoSignalError)),
        }
    }
}
