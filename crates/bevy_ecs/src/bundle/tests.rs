use crate::{
    archetype::{Archetype, ArchetypeCreated, ArchetypeId},
    lifecycle::HookContext,
    prelude::*,
    world::DeferredWorld,
};
use alloc::{vec, vec::Vec};

#[derive(Component)]
struct A;

#[derive(Component)]
#[component(on_add = a_on_add, on_insert = a_on_insert, on_discard = a_on_discard, on_remove = a_on_remove)]
struct AMacroHooks;

fn a_on_add(mut world: DeferredWorld, _: HookContext) {
    world.resource_mut::<R>().assert_order(0);
}

fn a_on_insert(mut world: DeferredWorld, _: HookContext) {
    world.resource_mut::<R>().assert_order(1);
}

fn a_on_discard(mut world: DeferredWorld, _: HookContext) {
    world.resource_mut::<R>().assert_order(2);
}

fn a_on_remove(mut world: DeferredWorld, _: HookContext) {
    world.resource_mut::<R>().assert_order(3);
}

#[derive(Component)]
struct B;

#[derive(Component)]
struct C;

#[derive(Component)]
struct D;

#[derive(Component, Eq, PartialEq, Debug)]
struct V(&'static str); // component with a value

#[derive(Resource, Default)]
struct R(usize);

impl R {
    #[track_caller]
    fn assert_order(&mut self, count: usize) {
        assert_eq!(count, self.0);
        self.0 += 1;
    }
}

#[derive(Bundle)]
#[bundle(ignore_from_components)]
struct BundleNoExtract {
    b: B,
    no_from_comp: crate::spawn::SpawnRelatedBundle<ChildOf, Spawn<C>>,
}

#[test]
fn can_spawn_bundle_without_extract() {
    let mut world = World::new();
    let id = world
        .spawn(BundleNoExtract {
            b: B,
            no_from_comp: Children::spawn(Spawn(C)),
        })
        .id();

    assert!(world.entity(id).get::<Children>().is_some());
}

#[test]
fn component_hook_order_spawn_despawn() {
    let mut world = World::new();
    world.init_resource::<R>();
    world
        .register_component_hooks::<A>()
        .on_add(|mut world, _| world.resource_mut::<R>().assert_order(0))
        .on_insert(|mut world, _| world.resource_mut::<R>().assert_order(1))
        .on_discard(|mut world, _| world.resource_mut::<R>().assert_order(2))
        .on_remove(|mut world, _| world.resource_mut::<R>().assert_order(3));

    let entity = world.spawn(A).id();
    world.despawn(entity);
    assert_eq!(4, world.resource::<R>().0);
}

#[test]
fn component_hook_order_spawn_despawn_with_macro_hooks() {
    let mut world = World::new();
    world.init_resource::<R>();

    let entity = world.spawn(AMacroHooks).id();
    world.despawn(entity);

    assert_eq!(4, world.resource::<R>().0);
}

#[test]
fn component_hook_order_insert_remove() {
    let mut world = World::new();
    world.init_resource::<R>();
    world
        .register_component_hooks::<A>()
        .on_add(|mut world, _| world.resource_mut::<R>().assert_order(0))
        .on_insert(|mut world, _| world.resource_mut::<R>().assert_order(1))
        .on_discard(|mut world, _| world.resource_mut::<R>().assert_order(2))
        .on_remove(|mut world, _| world.resource_mut::<R>().assert_order(3));

    let mut entity = world.spawn_empty();
    entity.insert(A);
    entity.remove::<A>();
    entity.flush();
    assert_eq!(4, world.resource::<R>().0);
}

#[test]
fn component_hook_order_replace() {
    let mut world = World::new();
    world
        .register_component_hooks::<A>()
        .on_discard(|mut world, _| world.resource_mut::<R>().assert_order(0))
        .on_insert(|mut world, _| {
            if let Some(mut r) = world.get_resource_mut::<R>() {
                r.assert_order(1);
            }
        });

    let entity = world.spawn(A).id();
    world.init_resource::<R>();
    let mut entity = world.entity_mut(entity);
    entity.insert(A);
    entity.insert_if_new(A); // this will not trigger on_discard or on_insert
    entity.flush();
    assert_eq!(2, world.resource::<R>().0);
}

#[test]
fn component_hook_order_recursive() {
    let mut world = World::new();
    world.init_resource::<R>();
    world
        .register_component_hooks::<A>()
        .on_add(|mut world, context| {
            world.resource_mut::<R>().assert_order(0);
            world.commands().entity(context.entity).insert(B);
        })
        .on_remove(|mut world, context| {
            world.resource_mut::<R>().assert_order(2);
            world.commands().entity(context.entity).remove::<B>();
        });

    world
        .register_component_hooks::<B>()
        .on_add(|mut world, context| {
            world.resource_mut::<R>().assert_order(1);
            world.commands().entity(context.entity).remove::<A>();
        })
        .on_remove(|mut world, _| {
            world.resource_mut::<R>().assert_order(3);
        });

    let entity = world.spawn(A).flush();
    let entity = world.get_entity(entity).unwrap();
    assert!(!entity.contains::<A>());
    assert!(!entity.contains::<B>());
    assert_eq!(4, world.resource::<R>().0);
}

#[test]
fn component_hook_order_recursive_multiple() {
    let mut world = World::new();
    world.init_resource::<R>();
    world
        .register_component_hooks::<A>()
        .on_add(|mut world, context| {
            world.resource_mut::<R>().assert_order(0);
            world.commands().entity(context.entity).insert(B).insert(C);
        });

    world
        .register_component_hooks::<B>()
        .on_add(|mut world, context| {
            world.resource_mut::<R>().assert_order(1);
            world.commands().entity(context.entity).insert(D);
        });

    world
        .register_component_hooks::<C>()
        .on_add(|mut world, _| {
            world.resource_mut::<R>().assert_order(3);
        });

    world
        .register_component_hooks::<D>()
        .on_add(|mut world, _| {
            world.resource_mut::<R>().assert_order(2);
        });

    world.spawn(A).flush();
    assert_eq!(4, world.resource::<R>().0);
}

#[test]
fn insert_if_new() {
    let mut world = World::new();
    let id = world.spawn(V("one")).id();
    let mut entity = world.entity_mut(id);
    entity.insert_if_new(V("two"));
    entity.insert_if_new((A, V("three")));
    entity.flush();
    // should still contain "one"
    let entity = world.entity(id);
    assert!(entity.contains::<A>());
    assert_eq!(entity.get(), Some(&V("one")));
}

#[derive(Component, Debug, Eq, PartialEq)]
#[component(storage = "SparseSet")]
pub struct SparseV(&'static str);

#[derive(Component, Debug, Eq, PartialEq)]
#[component(storage = "SparseSet")]
pub struct SparseA;

#[test]
fn sparse_set_insert_if_new() {
    let mut world = World::new();
    let id = world.spawn(SparseV("one")).id();
    let mut entity = world.entity_mut(id);
    entity.insert_if_new(SparseV("two"));
    entity.insert_if_new((SparseA, SparseV("three")));
    entity.flush();
    // should still contain "one"
    let entity = world.entity(id);
    assert!(entity.contains::<SparseA>());
    assert_eq!(entity.get(), Some(&SparseV("one")));
}

#[test]
fn new_archetype_created() {
    let mut world = World::new();
    #[derive(Resource, Default)]
    struct Count(u32);
    world.init_resource::<Count>();
    world.add_observer(|_t: On<ArchetypeCreated>, mut count: ResMut<Count>| {
        count.0 += 1;
    });

    let mut e = world.spawn((A, B));
    e.insert(C);
    e.remove::<A>();
    e.insert(A);
    e.insert(A);

    assert_eq!(world.resource::<Count>().0, 3);
}

#[test]
fn new_archetype_created_triggered_first() {
    let mut world = World::new();
    #[derive(Resource, Default)]
    struct Log(Vec<(Option<ArchetypeId>, &'static str)>);
    world.init_resource::<Log>();
    world.add_observer(|t: On<ArchetypeCreated>, mut log: ResMut<Log>| {
        log.0.push((Some(t.event().0), "Archetype created"));
    });
    world.add_observer(|t: On<Insert<A>>, mut log: ResMut<Log>| {
        log.0.push((
            t.trigger().new_archetype.map(Archetype::id),
            "Bundle inserted",
        ));
    });

    let archetype = world.spawn(A).archetype().id();
    assert_eq!(
        world.resource::<Log>().0,
        vec![
            (Some(archetype), "Archetype created"),
            (Some(archetype), "Bundle inserted")
        ]
    );
}

#[derive(Bundle)]
#[expect(unused, reason = "tests the output of the derive macro is valid")]
struct Ignore {
    #[bundle(ignore)]
    foo: i32,
    #[bundle(ignore)]
    bar: i32,
}

#[test]
fn table_table_mutually_exclusive_component_removes_other() {
    let mut world = World::new();
    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component, Default)]
    struct CompB;

    world.register_mutually_exclusive_components::<(CompA, CompB)>();

    let e = world.spawn(CompA).id();
    world.entity_mut(e).insert(CompB);
    assert!(!world.entity(e).contains::<CompA>());
    world.entity_mut(e).insert(CompA);
    assert!(!world.entity(e).contains::<CompB>());
}

#[test]
fn sparse_sparse_mutually_exclusive_component_removes_other() {
    let mut world = World::new();
    #[derive(Component, Default)]
    #[component(storage = "SparseSet")]
    struct CompA;

    #[derive(Component, Default)]
    #[component(storage = "SparseSet")]
    struct CompB;

    world.register_mutually_exclusive_components::<(CompA, CompB)>();

    let e = world.spawn(CompA).id();
    world.entity_mut(e).insert(CompB);
    assert!(!world.entity(e).contains::<CompA>());
    world.entity_mut(e).insert(CompA);
    assert!(!world.entity(e).contains::<CompB>());
}

#[test]
fn table_sparse_mutually_exclusive_component_removes_other() {
    let mut world = World::new();
    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component, Default)]
    #[component(storage = "SparseSet")]
    struct CompB;

    world.register_mutually_exclusive_components::<(CompA, CompB)>();

    let e = world.spawn(CompA).id();
    world.entity_mut(e).insert(CompB);
    assert!(!world.entity(e).contains::<CompA>());
    world.entity_mut(e).insert(CompA);
    assert!(!world.entity(e).contains::<CompB>());
}

#[test]
fn sparse_table_mutually_exclusive_component_removes_other() {
    let mut world = World::new();
    #[derive(Component, Default)]
    #[component(storage = "SparseSet")]
    struct CompA;

    #[derive(Component, Default)]
    struct CompB;

    world.register_mutually_exclusive_components::<(CompA, CompB)>();

    let e = world.spawn(CompA).id();
    world.entity_mut(e).insert(CompB);
    assert!(!world.entity(e).contains::<CompA>());
    world.entity_mut(e).insert(CompA);
    assert!(!world.entity(e).contains::<CompB>());
}

#[test]
fn mutually_exclusive_component_hooks_run() {
    let mut world = World::new();
    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component, Default)]
    struct CompB;

    #[derive(Resource, Default, PartialEq, Eq, Debug)]
    struct Counter {
        add: u8,
        insert: u8,
        discard: u8,
        remove: u8,
    }

    world.init_resource::<Counter>();
    world.register_mutually_exclusive_components::<(CompA, CompB)>();
    world
        .register_component_hooks::<CompA>()
        .on_add(|mut world, _| world.resource_mut::<Counter>().add += 1)
        .on_insert(|mut world, _| world.resource_mut::<Counter>().insert += 1)
        .on_discard(|mut world, _| world.resource_mut::<Counter>().discard += 1)
        .on_remove(|mut world, _| world.resource_mut::<Counter>().remove += 1);

    let e = world.spawn(CompA).id();

    world.entity_mut(e).insert(CompB);
    assert!(!world.entity(e).contains::<CompA>());
    assert_eq!(
        &Counter {
            add: 1,
            insert: 1,
            discard: 1,
            remove: 1
        },
        world.resource::<Counter>()
    );

    world.entity_mut(e).insert(CompA);
    assert!(!world.entity(e).contains::<CompB>());
    assert_eq!(
        &Counter {
            add: 2,
            insert: 2,
            discard: 1,
            remove: 1
        },
        world.resource::<Counter>()
    );
}

#[test]
#[should_panic]
fn mutually_exclusive_spawn_panics() {
    let mut world = World::new();
    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component, Default)]
    struct CompB;

    world.register_mutually_exclusive_components::<(CompA, CompB)>();

    world.spawn((CompA, CompB));
}

#[test]
fn mutually_exclusive_required_removes_other() {
    let mut world = World::new();

    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component, Default)]
    struct CompB;

    #[derive(Component, Default)]
    struct CompC;

    // Depth 1
    world.register_mutually_exclusive_components::<(CompA, CompB)>();
    world.register_required_components::<CompB, CompC>();

    #[derive(Component, Default)]
    struct CompD;

    #[derive(Component, Default)]
    struct CompE;

    #[derive(Component, Default)]
    struct CompF;

    // Depth 2
    world.register_mutually_exclusive_components::<(CompA, CompF)>();
    world.register_required_components::<CompD, CompE>();
    world.register_required_components::<CompE, CompF>();

    let e = world.spawn(CompA).id();
    world.entity_mut(e).insert(CompB);
    assert!(!world.entity(e).contains::<CompA>());
    assert!(world.entity(e).contains::<CompB>());
    assert!(world.entity(e).contains::<CompC>());

    let e = world.spawn(CompA).id();
    world.entity_mut(e).insert(CompD);
    assert!(!world.entity(e).contains::<CompA>());
    assert!(world.entity(e).contains::<CompF>());
}

#[test]
#[should_panic]
fn registering_required_as_mutually_exclusive_panics() {
    let mut world = World::new();
    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component, Default)]
    struct CompB;

    world.register_required_components::<CompA, CompB>();
    world.register_mutually_exclusive_components::<(CompA, CompB)>();
}

#[test]
#[should_panic]
fn registering_mutually_exclusive_as_required_panics() {
    let mut world = World::new();
    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component, Default)]
    struct CompB;

    world.register_mutually_exclusive_components::<(CompA, CompB)>();
    world.register_required_components::<CompA, CompB>();
}

#[test]
#[should_panic]
fn registering_mutually_exclusive_after_any_archetype_contains_target_components_panics() {
    let mut world = World::new();
    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component, Default)]
    struct CompB;

    world.spawn(CompA);
    world.register_mutually_exclusive_components::<(CompA, CompB)>();
}

#[test]
#[should_panic]
fn registering_mutually_exclusive_as_indirect_required_panics() {
    let mut world = World::new();
    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component, Default)]
    struct CompB;

    #[derive(Component, Default)]
    struct CompC;

    world.register_mutually_exclusive_components::<(CompB, CompC)>();
    world.register_required_components::<CompA, CompB>();
    world.register_required_components::<CompA, CompC>();
}

#[test]
fn mutually_exclusive_with_same_required() {
    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component, Default)]
    struct CompB;

    #[derive(Component, Default)]
    struct CompC;

    // Required first
    let mut world = World::new();
    world.register_required_components::<CompA, CompC>();
    world.register_required_components::<CompB, CompC>();
    world.register_mutually_exclusive_components::<(CompA, CompB)>();

    // Mutually exclusive first
    let mut world = World::new();
    world.register_mutually_exclusive_components::<(CompA, CompB)>();
    world.register_required_components::<CompA, CompC>();
    world.register_required_components::<CompB, CompC>();
}

#[test]
#[should_panic]
fn mutually_exclusive_within_required_panics_on_spawn() {
    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component, Default)]
    struct CompB;

    #[derive(Component, Default)]
    struct CompC;

    #[derive(Component, Default)]
    struct CompD;

    #[derive(Component, Default)]
    struct CompE;

    let mut world = World::new();
    world.register_mutually_exclusive_components::<(CompC, CompE)>();
    world.register_required_components::<CompA, CompC>();
    world.register_required_components::<CompB, CompD>();
    world.register_required_components::<CompD, CompE>();

    world.spawn((CompA, CompB));
}

#[test]
#[should_panic]
fn mutually_exclusive_after_bundle_register_panics_on_spawn() {
    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component, Default)]
    struct CompB;

    let mut world = World::new();
    world.register_bundle::<(CompA, CompB)>();
    world.register_mutually_exclusive_components::<(CompA, CompB)>();

    world.spawn((CompA, CompB));
}

#[test]
#[should_panic(expected = "has mutually exclusive components")]
fn mutually_exclusive_with_existing_component_on_archetype_should_panic() {
    #[derive(Component, Default)]
    struct Grounded;

    #[derive(Component, Default)]
    struct Airborne;

    #[derive(Component, Default)]
    #[require(Airborne)]
    struct Jumping;

    let mut world = World::new();
    world.register_mutually_exclusive_components::<(Grounded, Airborne)>();

    let player = world.spawn(Grounded).id();
    world.entity_mut(player).insert((Grounded, Jumping));
}

#[test]
fn mutually_exclusive_removal_triggers_removed_components_message() {
    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component, Default)]
    #[component(storage = "SparseSet")]
    struct CompB;

    #[derive(Component, Default)]
    struct CompC;

    #[derive(Component, Default)]
    #[component(storage = "SparseSet")]
    struct CompD;

    let mut world = World::new();
    let a_id = world.register_component::<CompA>();
    let b_id = world.register_component::<CompB>();

    world.register_mutually_exclusive_components::<(CompA, CompC)>();
    world.register_mutually_exclusive_components::<(CompB, CompC)>();
    world.register_mutually_exclusive_components::<(CompB, CompD)>();

    // Table move
    let e1 = world.spawn((CompA, CompB)).id();
    world.entity_mut(e1).insert(CompC);
    assert_eq!(
        world
            .removed_components()
            .get(a_id)
            .unwrap()
            .iter_current_update_messages()
            .cloned()
            .map(Into::into)
            .collect::<Vec<Entity>>(),
        vec![e1]
    );
    assert_eq!(
        world
            .removed_components()
            .get(b_id)
            .unwrap()
            .iter_current_update_messages()
            .cloned()
            .map(Into::into)
            .collect::<Vec<Entity>>(),
        vec![e1]
    );
    world.removed_components.update();

    // Archetype move
    let e2 = world.spawn((CompA, CompB)).id();
    world.entity_mut(e2).insert(CompD);
    assert_eq!(
        world
            .removed_components()
            .get(b_id)
            .unwrap()
            .iter_current_update_messages()
            .cloned()
            .map(Into::into)
            .collect::<Vec<Entity>>(),
        vec![e2]
    );
}

#[test]
fn mutually_exclusive_removal_removed_components_message_has_not_duplicates() {
    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component, Default)]
    struct CompB;

    #[derive(Component, Default)]
    struct CompC;

    let mut world = World::new();
    let a_id = world.register_component::<CompA>();

    world.register_mutually_exclusive_components::<(CompA, CompB)>();
    world.register_mutually_exclusive_components::<(CompA, CompC)>();

    let e1 = world.spawn(CompA).id();
    world.entity_mut(e1).insert((CompB, CompC));
    assert_eq!(
        world
            .removed_components()
            .get(a_id)
            .unwrap()
            .iter_current_update_messages()
            .cloned()
            .map(Into::into)
            .collect::<Vec<Entity>>(),
        vec![e1]
    );
}

#[test]
#[should_panic]
fn mutually_exclusive_with_existing_required_component_panics() {
    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component, Default)]
    struct CompB;

    #[derive(Component, Default)]
    #[require(CompA)]
    struct CompC;

    #[derive(Component, Default)]
    #[require(CompB)]
    struct CompD;

    let mut world = World::new();
    world.register_mutually_exclusive_components::<(CompA, CompB)>();

    let e = world.spawn(CompA).id();
    world.entity_mut(e).insert((CompC, CompD));
}

fn insert_if_new_keeps_existing_mutually_exclusive<
    Existing: Component + Default,
    New: Component + Default,
>() {
    let mut world = World::new();
    world.register_mutually_exclusive_components::<(Existing, New)>();

    let e = world.spawn(Existing::default()).id();
    world.entity_mut(e).insert_if_new(New::default());
    assert!(world.entity(e).contains::<Existing>());
    assert!(!world.entity(e).contains::<New>());

    world.commands().entity(e).insert_if_new(New::default());
    world.flush();
    assert!(world.entity(e).contains::<Existing>());
    assert!(!world.entity(e).contains::<New>());

    let e = world.spawn_empty().id();
    world.entity_mut(e).insert_if_new(New::default());
    assert!(world.entity(e).contains::<New>());
}

#[test]
fn insert_if_new_keeps_existing_mutually_exclusive_component() {
    #[derive(Component, Default)]
    struct TableA;

    #[derive(Component, Default)]
    struct TableB;

    #[derive(Component, Default)]
    #[component(storage = "SparseSet")]
    struct SparseA;

    #[derive(Component, Default)]
    #[component(storage = "SparseSet")]
    struct SparseB;

    insert_if_new_keeps_existing_mutually_exclusive::<TableA, TableB>();
    insert_if_new_keeps_existing_mutually_exclusive::<SparseA, SparseB>();
    insert_if_new_keeps_existing_mutually_exclusive::<TableA, SparseB>();
    insert_if_new_keeps_existing_mutually_exclusive::<SparseA, TableB>();
}

#[test]
fn insert_if_new_skipped_component_runs_no_hooks_and_is_dropped() {
    use alloc::sync::Arc;

    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component)]
    struct CompB(#[expect(dead_code, reason = "only dropped")] Arc<()>);

    #[derive(Component)]
    #[component(storage = "SparseSet")]
    struct CompC(#[expect(dead_code, reason = "only dropped")] Arc<()>);

    #[derive(Resource, Default)]
    struct Counter(u8);

    let mut world = World::new();
    world.init_resource::<Counter>();
    world.register_mutually_exclusive_components::<(CompA, CompB)>();
    world.register_mutually_exclusive_components::<(CompA, CompC)>();
    world
        .register_component_hooks::<CompA>()
        .on_discard(|mut world, _| world.resource_mut::<Counter>().0 += 1)
        .on_remove(|mut world, _| world.resource_mut::<Counter>().0 += 1);
    world
        .register_component_hooks::<CompB>()
        .on_add(|mut world, _| world.resource_mut::<Counter>().0 += 1)
        .on_insert(|mut world, _| world.resource_mut::<Counter>().0 += 1);

    let value = Arc::new(());
    let e = world.spawn(CompA).id();
    world
        .entity_mut(e)
        .insert_if_new((CompB(value.clone()), CompC(value.clone())));

    assert!(world.entity(e).contains::<CompA>());
    assert!(!world.entity(e).contains::<CompB>());
    assert!(!world.entity(e).contains::<CompC>());
    assert_eq!(world.resource::<Counter>().0, 0);
    assert_eq!(Arc::strong_count(&value), 1);
}

#[test]
fn insert_if_new_adds_components_that_are_not_mutually_exclusive() {
    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component, Default)]
    struct CompB;

    #[derive(Component, Default)]
    struct CompC;

    let mut world = World::new();
    world.register_mutually_exclusive_components::<(CompA, CompB)>();

    let e = world.spawn(CompA).id();
    world.entity_mut(e).insert_if_new((CompB, CompC));
    assert!(world.entity(e).contains::<CompA>());
    assert!(!world.entity(e).contains::<CompB>());
    assert!(world.entity(e).contains::<CompC>());
}

#[test]
fn insert_if_new_skips_required_components_of_skipped_component() {
    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component, Default)]
    #[require(CompC)]
    struct CompB;

    #[derive(Component, Default)]
    struct CompC;

    #[derive(Component, Default)]
    #[require(CompE)]
    struct CompD;

    #[derive(Component, Default)]
    #[require(CompF)]
    struct CompE;

    #[derive(Component, Default)]
    struct CompF;

    #[derive(Component, Default)]
    #[require(CompC)]
    struct CompG;

    let mut world = World::new();
    world.register_mutually_exclusive_components::<(CompA, CompB)>();
    world.register_mutually_exclusive_components::<(CompA, CompF)>();

    let e = world.spawn(CompA).id();
    world.entity_mut(e).insert_if_new(CompB);
    assert!(world.entity(e).contains::<CompA>());
    assert!(!world.entity(e).contains::<CompB>());
    assert!(!world.entity(e).contains::<CompC>());

    // Adding `CompD` would add `CompF`, which would remove `CompA`.
    let e = world.spawn(CompA).id();
    world.entity_mut(e).insert_if_new(CompD);
    assert!(world.entity(e).contains::<CompA>());
    assert!(!world.entity(e).contains::<CompD>());
    assert!(!world.entity(e).contains::<CompE>());
    assert!(!world.entity(e).contains::<CompF>());

    // `CompC` is still required by `CompG`, which is not skipped.
    let e = world.spawn(CompA).id();
    world.entity_mut(e).insert_if_new((CompB, CompG));
    assert!(world.entity(e).contains::<CompA>());
    assert!(!world.entity(e).contains::<CompB>());
    assert!(world.entity(e).contains::<CompC>());
    assert!(world.entity(e).contains::<CompG>());
}

#[test]
fn insert_if_new_does_not_add_required_component_that_removes_existing() {
    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component, Default)]
    #[require(CompC)]
    struct CompB;

    #[derive(Component, Default)]
    struct CompC;

    let mut world = World::new();
    world.register_mutually_exclusive_components::<(CompA, CompC)>();

    let e = world.spawn(CompB).id();
    world.entity_mut(e).remove::<CompC>().insert(CompA);

    world.entity_mut(e).insert_if_new(CompB);
    assert!(world.entity(e).contains::<CompA>());
    assert!(world.entity(e).contains::<CompB>());
    assert!(!world.entity(e).contains::<CompC>());

    world.entity_mut(e).insert(CompB);
    assert!(!world.entity(e).contains::<CompA>());
    assert!(world.entity(e).contains::<CompC>());
}

#[test]
fn insert_modes_cache_separate_archetype_edges() {
    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component, Default)]
    struct CompB;

    let mut world = World::new();
    world.register_mutually_exclusive_components::<(CompA, CompB)>();

    let e1 = world.spawn(CompA).id();
    world.entity_mut(e1).insert_if_new(CompB);
    assert!(world.entity(e1).contains::<CompA>());

    let e2 = world.spawn(CompA).id();
    world.entity_mut(e2).insert(CompB);
    assert!(!world.entity(e2).contains::<CompA>());
    assert!(world.entity(e2).contains::<CompB>());

    let e3 = world.spawn(CompA).id();
    world.entity_mut(e3).insert_if_new(CompB);
    assert!(world.entity(e3).contains::<CompA>());
    assert!(!world.entity(e3).contains::<CompB>());

    let e4 = world.spawn(CompA).id();
    let e5 = world.spawn_empty().id();
    world.insert_batch_if_new([(e4, CompB), (e5, CompB)]);
    assert!(world.entity(e4).contains::<CompA>());
    assert!(!world.entity(e4).contains::<CompB>());
    assert!(world.entity(e5).contains::<CompB>());
}

#[test]
fn insert_modes_share_archetype_edge_without_skipped_components() {
    use crate::bundle::InsertMode;

    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component, Default)]
    struct CompB;

    #[derive(Component, Default)]
    struct CompC;

    #[derive(Component, Default)]
    struct CompD;

    let mut world = World::new();
    world.register_mutually_exclusive_components::<(CompA, CompB)>();
    let bundle_b = world.register_bundle::<CompB>().id();
    let bundle_c = world.register_bundle::<CompC>().id();
    let bundle_d = world.register_bundle::<CompD>().id();

    let e = world.spawn(CompA).id();
    let archetype_id = world.entity(e).archetype().id();
    let edge = |world: &World, bundle_id, insert_mode| {
        world.archetypes()[archetype_id]
            .edges()
            .get_archetype_after_bundle_insert(bundle_id, insert_mode)
    };

    // `CompC` removes no components, so the edge cached by `InsertMode::Replace` is shared.
    world.entity_mut(e).insert(CompC);
    assert!(edge(&world, bundle_c, InsertMode::Replace).is_some());
    assert_eq!(
        edge(&world, bundle_c, InsertMode::Keep),
        edge(&world, bundle_c, InsertMode::Replace)
    );

    // The same holds for an edge cached by `InsertMode::Keep`.
    let e = world.spawn(CompA).id();
    world.entity_mut(e).insert_if_new(CompD);
    assert!(edge(&world, bundle_d, InsertMode::Keep).is_some());
    assert_eq!(
        edge(&world, bundle_d, InsertMode::Replace),
        edge(&world, bundle_d, InsertMode::Keep)
    );

    // `CompB` removes `CompA`, so `InsertMode::Keep` needs its own edge.
    let e = world.spawn(CompA).id();
    world.entity_mut(e).insert(CompB);
    assert!(edge(&world, bundle_b, InsertMode::Replace).is_some());
    assert_eq!(edge(&world, bundle_b, InsertMode::Keep), None);

    let e = world.spawn(CompA).id();
    world.entity_mut(e).insert_if_new(CompB);
    assert_eq!(edge(&world, bundle_b, InsertMode::Keep), Some(archetype_id));
    assert!(world.entity(e).contains::<CompA>());
    assert!(!world.entity(e).contains::<CompB>());
}

#[test]
#[should_panic(expected = "has mutually exclusive components")]
fn insert_if_new_bundle_with_mutually_exclusive_components_panics() {
    #[derive(Component, Default)]
    struct Grounded;

    #[derive(Component, Default)]
    struct Airborne;

    #[derive(Component, Default)]
    #[require(Airborne)]
    struct Jumping;

    let mut world = World::new();
    world.register_mutually_exclusive_components::<(Grounded, Airborne)>();

    let player = world.spawn(Grounded).id();
    world.entity_mut(player).insert_if_new((Grounded, Jumping));
}

#[test]
fn clone_if_new_skips_mutually_exclusive_component() {
    use crate::{bundle::InsertMode, entity::EntityCloner};

    #[derive(Component, Clone, Default)]
    struct CompA;

    #[derive(Component, Clone, Default)]
    struct CompB;

    #[derive(Component, Clone, Default)]
    struct CompC;

    let mut world = World::new();
    world.register_mutually_exclusive_components::<(CompA, CompB)>();
    let source = world.spawn((CompB, CompC)).id();

    let target = world.spawn(CompA).id();
    EntityCloner::build_opt_out(&mut world)
        .insert_mode(InsertMode::Keep)
        .clone_entity(source, target);
    assert!(world.entity(target).contains::<CompA>());
    assert!(!world.entity(target).contains::<CompB>());
    assert!(world.entity(target).contains::<CompC>());

    let target = world.spawn(CompA).id();
    EntityCloner::build_opt_in(&mut world)
        .allow_if_new::<(CompB, CompC)>()
        .clone_entity(source, target);
    assert!(world.entity(target).contains::<CompA>());
    assert!(!world.entity(target).contains::<CompB>());
    assert!(world.entity(target).contains::<CompC>());

    let target = world.spawn(CompA).id();
    EntityCloner::build_opt_out(&mut world).clone_entity(source, target);
    assert!(!world.entity(target).contains::<CompA>());
    assert!(world.entity(target).contains::<CompB>());
}
