use std::any::TypeId;
use std::collections::HashMap;

use super::entity::EntityId;
use super::component::Component;
use super::storage::ComponentStorage;

/// Entity生存情報
struct EntityMeta {
    /// 現在の世代番号
    generation: u32,
    /// 生存フラグ
    alive: bool,
}

/// 型消去されたストレージのトレイト
trait AnyStorage: Send + Sync {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn remove(&mut self, entity: EntityId);
}

impl<T: Component> AnyStorage for ComponentStorage<T> {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn remove(&mut self, entity: EntityId) {
        ComponentStorage::remove(self, entity);
    }
}

/// ECSのメインコンテナ
/// 全てのEntity/Componentを管理する
pub struct World {
    /// Entity生存情報
    entities: Vec<EntityMeta>,
    /// 再利用可能なインデックス
    free_list: Vec<u32>,
    /// 型ごとのコンポーネントストレージ
    storages: HashMap<TypeId, Box<dyn AnyStorage>>,
}

impl World {
    /// 新しいWorldを作成
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
            free_list: Vec::new(),
            storages: HashMap::new(),
        }
    }

    /// 新規Entityを生成
    pub fn spawn(&mut self) -> EntityId {
        if let Some(index) = self.free_list.pop() {
            // 再利用: 世代番号をインクリメント
            let meta = &mut self.entities[index as usize];
            meta.generation += 1;
            meta.alive = true;
            EntityId::new(index, meta.generation)
        } else {
            // 新規割り当て
            let index = self.entities.len() as u32;
            self.entities.push(EntityMeta {
                generation: 1,
                alive: true,
            });
            EntityId::new(index, 1)
        }
    }

    /// Entityを削除
    /// 成功時true、既に削除済みまたは無効なEntityの場合false
    pub fn despawn(&mut self, entity: EntityId) -> bool {
        let index = entity.index() as usize;

        if index >= self.entities.len() {
            return false;
        }

        let meta = &mut self.entities[index];
        if !meta.alive || meta.generation != entity.generation() {
            return false;
        }

        meta.alive = false;
        self.free_list.push(entity.index());

        // 全ストレージからコンポーネントを削除
        for storage in self.storages.values_mut() {
            storage.remove(entity);
        }

        true
    }

    /// Entityが生存しているか確認
    pub fn is_alive(&self, entity: EntityId) -> bool {
        let index = entity.index() as usize;
        self.entities
            .get(index)
            .is_some_and(|meta| meta.alive && meta.generation == entity.generation())
    }

    /// コンポーネントを追加
    pub fn insert<T: Component>(&mut self, entity: EntityId, component: T) {
        if !self.is_alive(entity) {
            return;
        }

        let storage = self.get_or_create_storage::<T>();
        storage.insert(entity, component);
    }

    /// コンポーネントを取得（不変参照）
    pub fn get<T: Component>(&self, entity: EntityId) -> Option<&T> {
        if !self.is_alive(entity) {
            return None;
        }

        self.get_storage::<T>()?.get(entity)
    }

    /// コンポーネントを取得（可変参照）
    pub fn get_mut<T: Component>(&mut self, entity: EntityId) -> Option<&mut T> {
        if !self.is_alive(entity) {
            return None;
        }

        self.get_storage_mut::<T>()?.get_mut(entity)
    }

    /// コンポーネントを削除
    pub fn remove<T: Component>(&mut self, entity: EntityId) -> Option<T> {
        if !self.is_alive(entity) {
            return None;
        }

        self.get_storage_mut::<T>()?.remove(entity)
    }

    /// 全Entityをイテレート
    pub fn iter_entities(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.entities
            .iter()
            .enumerate()
            .filter(|(_, meta)| meta.alive)
            .map(|(index, meta)| EntityId::new(index as u32, meta.generation))
    }

    /// 生存Entity数を取得
    pub fn entity_count(&self) -> usize {
        self.entities.iter().filter(|meta| meta.alive).count()
    }

    /// 型に対応するストレージを取得または作成
    fn get_or_create_storage<T: Component>(&mut self) -> &mut ComponentStorage<T> {
        let type_id = TypeId::of::<T>();

        self.storages
            .entry(type_id)
            .or_insert_with(|| Box::new(ComponentStorage::<T>::new()))
            .as_any_mut()
            .downcast_mut::<ComponentStorage<T>>()
            .expect("type mismatch in storage")
    }

    /// 型に対応するストレージを取得（不変）
    fn get_storage<T: Component>(&self) -> Option<&ComponentStorage<T>> {
        let type_id = TypeId::of::<T>();
        self.storages
            .get(&type_id)?
            .as_any()
            .downcast_ref::<ComponentStorage<T>>()
    }

    /// 型に対応するストレージを取得（可変）
    fn get_storage_mut<T: Component>(&mut self) -> Option<&mut ComponentStorage<T>> {
        let type_id = TypeId::of::<T>();
        self.storages
            .get_mut(&type_id)?
            .as_any_mut()
            .downcast_mut::<ComponentStorage<T>>()
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Clone)]
    struct Position {
        x: f32,
        y: f32,
    }
    impl Component for Position {}

    #[derive(Debug, PartialEq, Clone)]
    struct Velocity {
        x: f32,
        y: f32,
    }
    impl Component for Velocity {}

    #[derive(Debug, PartialEq, Clone)]
    struct Name(String);
    impl Component for Name {}

    #[test]
    fn test_spawn() {
        let mut world = World::new();
        let e1 = world.spawn();
        let e2 = world.spawn();

        assert_eq!(e1.index(), 0);
        assert_eq!(e2.index(), 1);
        assert_eq!(e1.generation(), 1);
        assert_eq!(e2.generation(), 1);
    }

    #[test]
    fn test_despawn_and_reuse() {
        let mut world = World::new();
        let e1 = world.spawn();
        assert!(world.is_alive(e1));

        assert!(world.despawn(e1));
        assert!(!world.is_alive(e1));

        // 再利用: 同じindexだが世代が異なる
        let e2 = world.spawn();
        assert_eq!(e2.index(), e1.index());
        assert_eq!(e2.generation(), 2);

        // 古いEntityIdは無効
        assert!(!world.is_alive(e1));
        assert!(world.is_alive(e2));
    }

    #[test]
    fn test_insert_and_get() {
        let mut world = World::new();
        let entity = world.spawn();

        world.insert(entity, Position { x: 1.0, y: 2.0 });

        assert_eq!(world.get::<Position>(entity), Some(&Position { x: 1.0, y: 2.0 }));
        assert_eq!(world.get::<Velocity>(entity), None);
    }

    #[test]
    fn test_get_mut() {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert(entity, Position { x: 1.0, y: 2.0 });

        if let Some(pos) = world.get_mut::<Position>(entity) {
            pos.x = 10.0;
        }

        assert_eq!(world.get::<Position>(entity), Some(&Position { x: 10.0, y: 2.0 }));
    }

    #[test]
    fn test_multiple_components() {
        let mut world = World::new();
        let entity = world.spawn();

        world.insert(entity, Position { x: 1.0, y: 2.0 });
        world.insert(entity, Velocity { x: 3.0, y: 4.0 });
        world.insert(entity, Name("Player".to_string()));

        assert_eq!(world.get::<Position>(entity), Some(&Position { x: 1.0, y: 2.0 }));
        assert_eq!(world.get::<Velocity>(entity), Some(&Velocity { x: 3.0, y: 4.0 }));
        assert_eq!(world.get::<Name>(entity), Some(&Name("Player".to_string())));
    }

    #[test]
    fn test_despawn_removes_components() {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert(entity, Position { x: 1.0, y: 2.0 });

        world.despawn(entity);

        // despawn後は取得不可
        assert_eq!(world.get::<Position>(entity), None);
    }

    #[test]
    fn test_iter_entities() {
        let mut world = World::new();
        let e1 = world.spawn();
        let e2 = world.spawn();
        let e3 = world.spawn();

        world.despawn(e2);

        let entities: Vec<_> = world.iter_entities().collect();
        assert_eq!(entities.len(), 2);
        assert!(entities.contains(&e1));
        assert!(entities.contains(&e3));
    }

    #[test]
    fn test_entity_count() {
        let mut world = World::new();
        assert_eq!(world.entity_count(), 0);

        let e1 = world.spawn();
        let _e2 = world.spawn();
        assert_eq!(world.entity_count(), 2);

        world.despawn(e1);
        assert_eq!(world.entity_count(), 1);
    }

    #[test]
    fn test_remove_component() {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert(entity, Position { x: 1.0, y: 2.0 });

        let removed = world.remove::<Position>(entity);
        assert_eq!(removed, Some(Position { x: 1.0, y: 2.0 }));
        assert_eq!(world.get::<Position>(entity), None);
    }

    #[test]
    fn test_dead_entity_operations() {
        let mut world = World::new();
        let entity = world.spawn();
        world.despawn(entity);

        // 死んだEntityへの操作は無視される
        world.insert(entity, Position { x: 1.0, y: 2.0 });
        assert_eq!(world.get::<Position>(entity), None);
        assert_eq!(world.get_mut::<Position>(entity), None);
        assert_eq!(world.remove::<Position>(entity), None);
    }
}
