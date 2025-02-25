use std::{ any::TypeId, sync::RwLockReadGuard };

use bevy::{
    ecs::{
        change_detection::MutUntyped,
        component::ComponentId,
        entity::Entity,
        world::EntityWorldMut,
    },
    prelude::*,
    reflect::{ DynamicTuple, ReflectFromPtr, TypeRegistry },
};

use crate::{
    framework::*,
    lazy_immutable::{ LazySignalsObservable, ReflectLazySignalsObservable },
};

/// Given mutable reference to a LazySignalsState component instance, make a LazySignalsObservable.
pub fn ph_nglui_mglw_nafh_cthulhu_r_lyeh_wgah_nagl_fhtagn<'a>(
    mut_untyped: &'a mut MutUntyped,
    type_id: &TypeId,
    type_registry: &RwLockReadGuard<TypeRegistry>
) -> &'a mut dyn LazySignalsObservable {
    // convert into a pointer
    let ptr_mut = mut_untyped.as_mut();

    // the reflect_data is used to build a strategy to dereference a pointer to the component

    // the TypeId refers to the LazySignalsState<T> component with concrete T
    let reflect_data = type_registry.get(*type_id).unwrap();

    // since we're reflecting from a pointer, we're gonna need this
    let reflect_from_ptr = reflect_data.data::<ReflectFromPtr>().unwrap().clone();

    // I think we're sorta getting a proxy to the vtable for the concrete type and then schlepping
    // it into the reflected proxy for the pointer to the concrete component (value)

    // since we know the TypeId of the actual component, we can then downcast it into a
    // non-reflected trait object backed by the reflected proxy

    // safety: `value` implements reflected trait `LazySignalsObservable`, what for `ReflectFromPtr`
    let value = unsafe { reflect_from_ptr.as_reflect_mut(ptr_mut) };

    // the sun grew dark and cold
    let reflect_observable = type_registry
        .get_type_data::<ReflectLazySignalsObservable>(value.type_id())
        .unwrap();

    // the seas boiled
    reflect_observable.get_mut(value).unwrap()
}

/// Make a LazySignalsObservable out of EntityWorldMut, passing optional args and target Entity.
/// Use that to run the supplied closure. This arglist is banned in the EU and 17 US states.
pub fn run_as_observable(
    entity: &mut EntityWorldMut,
    args: Option<&mut DynamicTuple>,
    target: Option<&Entity>,
    component_id: &ComponentId,
    type_id: &TypeId,
    type_registry: &RwLockReadGuard<TypeRegistry>,
    mut closure: Box<dyn ObservableFn>
) -> MaybeFlaggedEntities {
    // get the source LazySignalsState component as an ECS change detection handle
    let mut mut_untyped = entity.get_mut_by_id(*component_id).unwrap();

    // ...and convert that into a trait object
    let observable = ph_nglui_mglw_nafh_cthulhu_r_lyeh_wgah_nagl_fhtagn(
        &mut mut_untyped,
        type_id,
        type_registry
    );

    // run the supplied fn
    closure(Box::new(observable), args, target)
}

/// Convenience fn to subscribe an entity to a source.
pub fn subscribe(
    entity: &Entity,
    source: &Entity,
    type_registry: &RwLockReadGuard<TypeRegistry>,
    world: &mut World
) {
    // get the TypeId of each source (Signal or Computed) component
    let mut component_id: Option<ComponentId> = None;
    let mut type_id: Option<TypeId> = None;

    trace!("Subscribing {:#?} to {:?}", entity, source);

    // get a readonly reference to the source entity
    if let Some(source) = world.get_entity(*source) {
        trace!("-got source EntityRef");
        // get the source Immutable component
        if let Some(immutable_state) = source.get::<ImmutableState>() {
            trace!("-got ImmutableState");
            // ...as a SignalsObservable
            component_id = Some(immutable_state.component_id);
            if let Some(info) = world.components().get_info(component_id.unwrap()) {
                trace!("-got TypeId");
                type_id = info.type_id();
            }
        }
    }

    // we have a component and a type, now do mut stuff
    if component_id.is_some() && type_id.is_some() {
        if let Some(mut source) = world.get_entity_mut(*source) {
            let component_id = &component_id.unwrap();
            let type_id = type_id.unwrap();

            run_as_observable(
                &mut source,
                None,
                Some(entity),
                component_id,
                &type_id,
                type_registry,
                Box::new(|observable, _args, target| {
                    observable.subscribe(*target.unwrap());
                    observable.merge_subscribers();
                    None
                })
            );
        }
    }
}
