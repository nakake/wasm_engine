use std::any::TypeId;
use std::collections::HashMap;

use super::entity::EntityId;
use super::component::Component;
use super::storage::ComponentStorage;
use super::query::{QueryDescriptor, QueryResult, QueryResultRow, FilterExpr, FilterValue, SortDirection};
use crate::components::{Transform, Name};

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

    /// 指定したコンポーネントを持つEntityとコンポーネントをイテレート
    pub fn iter_with<T: Component>(&self) -> impl Iterator<Item = (EntityId, &T)> {
        self.iter_entities().filter_map(|entity| {
            self.get::<T>(entity).map(|component| (entity, component))
        })
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

    // ========================================================================
    // Query System
    // ========================================================================

    /// クエリを実行
    pub fn execute_query(&self, query: &QueryDescriptor) -> QueryResult {
        // 1. 全生存Entityを取得
        let mut candidates: Vec<EntityId> = self.iter_entities().collect();

        // 2. with_components でフィルタリング
        for component_name in &query.with_components {
            candidates.retain(|&entity| self.has_component(entity, component_name));
        }

        // 3. without_components で除外
        for component_name in &query.without_components {
            candidates.retain(|&entity| !self.has_component(entity, component_name));
        }

        // 4. filters で条件フィルタ
        for filter in &query.filters {
            candidates.retain(|&entity| self.evaluate_filter(entity, filter));
        }

        let total_count = candidates.len();

        // 5. order_by でソート（Task 05で詳細実装）
        if let Some(ref order) = query.order_by {
            candidates.sort_by(|&a, &b| {
                let val_a = self.extract_field(a, &order.field);
                let val_b = self.extract_field(b, &order.field);
                let cmp = Self::compare_json_values(&val_a, &val_b);
                match order.direction {
                    SortDirection::Asc => cmp,
                    SortDirection::Desc => cmp.reverse(),
                }
            });
        }

        // 6. limit で件数制限
        if let Some(limit) = query.limit {
            candidates.truncate(limit);
        }

        // 7. select でフィールド抽出して結果を構築
        let rows: Vec<QueryResultRow> = candidates
            .into_iter()
            .map(|entity| {
                let mut row = QueryResultRow::new(entity.to_u32());

                // selectが空の場合はidのみ返す
                if query.select.is_empty() {
                    row.set_field("id", serde_json::json!(entity.to_u32()));
                } else {
                    for field in &query.select {
                        if let Some(value) = self.extract_field(entity, field) {
                            row.set_field(field.clone(), value);
                        }
                    }
                }

                row
            })
            .collect();

        QueryResult { rows, total_count }
    }

    /// Entityから指定フィールドの値を取得
    fn extract_field(&self, entity: EntityId, field: &str) -> Option<serde_json::Value> {
        match field {
            "id" => Some(serde_json::json!(entity.to_u32())),
            "name" => self
                .get::<Name>(entity)
                .map(|n| serde_json::json!(n.as_str())),
            "position" => self.get::<Transform>(entity).map(|t| {
                serde_json::json!({
                    "x": t.position.x,
                    "y": t.position.y,
                    "z": t.position.z,
                })
            }),
            "position.x" => self
                .get::<Transform>(entity)
                .map(|t| serde_json::json!(t.position.x)),
            "position.y" => self
                .get::<Transform>(entity)
                .map(|t| serde_json::json!(t.position.y)),
            "position.z" => self
                .get::<Transform>(entity)
                .map(|t| serde_json::json!(t.position.z)),
            "rotation" => self.get::<Transform>(entity).map(|t| {
                serde_json::json!({
                    "x": t.rotation.x,
                    "y": t.rotation.y,
                    "z": t.rotation.z,
                    "w": t.rotation.w,
                })
            }),
            "scale" => self.get::<Transform>(entity).map(|t| {
                serde_json::json!({
                    "x": t.scale.x,
                    "y": t.scale.y,
                    "z": t.scale.z,
                })
            }),
            "scale.x" => self
                .get::<Transform>(entity)
                .map(|t| serde_json::json!(t.scale.x)),
            "scale.y" => self
                .get::<Transform>(entity)
                .map(|t| serde_json::json!(t.scale.y)),
            "scale.z" => self
                .get::<Transform>(entity)
                .map(|t| serde_json::json!(t.scale.z)),
            _ => None,
        }
    }

    /// コンポーネントの存在チェック
    fn has_component(&self, entity: EntityId, component_name: &str) -> bool {
        match component_name {
            "Transform" => self.get::<Transform>(entity).is_some(),
            "Name" => self.get::<Name>(entity).is_some(),
            // カスタムコンポーネントは動的登録が必要（Phase 4以降）
            _ => false,
        }
    }

    /// フィルター条件を評価
    fn evaluate_filter(&self, entity: EntityId, filter: &FilterExpr) -> bool {
        let field_value = match self.extract_field(entity, &filter.field) {
            Some(v) => FilterValue::from_json(v),
            None => return false, // フィールドなし = マッチしない
        };

        filter.op.compare_values(&field_value, &filter.value)
    }

    /// JSON値の比較（ソート用）
    fn compare_json_values(
        a: &Option<serde_json::Value>,
        b: &Option<serde_json::Value>,
    ) -> std::cmp::Ordering {
        use std::cmp::Ordering;

        match (a, b) {
            (Some(serde_json::Value::Number(a)), Some(serde_json::Value::Number(b))) => {
                let a_f = a.as_f64().unwrap_or(0.0);
                let b_f = b.as_f64().unwrap_or(0.0);
                a_f.partial_cmp(&b_f).unwrap_or(Ordering::Equal)
            }
            (Some(serde_json::Value::String(a)), Some(serde_json::Value::String(b))) => a.cmp(b),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            _ => Ordering::Equal,
        }
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

    // ========================================================================
    // execute_query tests
    // ========================================================================

    use crate::components::{Transform as RealTransform, Name as RealName};
    use crate::ecs::query::{QueryDescriptor, FilterExpr, FilterValue, OrderBy};
    use glam::Vec3;

    #[test]
    fn test_execute_query_basic() {
        let mut world = World::new();

        // 3つのEntityを作成
        let e1 = world.spawn();
        world.insert(e1, RealName::new("Entity1"));
        world.insert(e1, RealTransform::from_position(Vec3::new(1.0, 0.0, 0.0)));

        let e2 = world.spawn();
        world.insert(e2, RealName::new("Entity2"));
        world.insert(e2, RealTransform::from_position(Vec3::new(2.0, 0.0, 0.0)));

        let e3 = world.spawn();
        world.insert(e3, RealName::new("Entity3"));
        // Transformなし

        // Transformを持つEntityをクエリ
        let query = QueryDescriptor::new()
            .select(["id", "name"])
            .with(["Transform"]);

        let result = world.execute_query(&query);
        assert_eq!(result.len(), 2);
        assert_eq!(result.total_count, 2);
    }

    #[test]
    fn test_execute_query_without() {
        let mut world = World::new();

        let e1 = world.spawn();
        world.insert(e1, RealName::new("WithTransform"));
        world.insert(e1, RealTransform::identity());

        let e2 = world.spawn();
        world.insert(e2, RealName::new("WithoutTransform"));

        // Transformを持たないEntityをクエリ
        let query = QueryDescriptor::new()
            .select(["name"])
            .with(["Name"])
            .without(["Transform"]);

        let result = world.execute_query(&query);
        assert_eq!(result.len(), 1);
        assert_eq!(
            result.rows[0].get_field("name"),
            Some(&serde_json::json!("WithoutTransform"))
        );
    }

    #[test]
    fn test_execute_query_filter() {
        let mut world = World::new();

        let e1 = world.spawn();
        world.insert(e1, RealName::new("Left"));
        world.insert(e1, RealTransform::from_position(Vec3::new(-5.0, 0.0, 0.0)));

        let e2 = world.spawn();
        world.insert(e2, RealName::new("Right"));
        world.insert(e2, RealTransform::from_position(Vec3::new(5.0, 0.0, 0.0)));

        // position.x > 0 のEntityをクエリ
        let query = QueryDescriptor::new()
            .select(["name", "position.x"])
            .with(["Transform"])
            .filter(FilterExpr::gt("position.x", FilterValue::Number(0.0)));

        let result = world.execute_query(&query);
        assert_eq!(result.len(), 1);
        assert_eq!(
            result.rows[0].get_field("name"),
            Some(&serde_json::json!("Right"))
        );
    }

    #[test]
    fn test_execute_query_order_by() {
        let mut world = World::new();

        let e1 = world.spawn();
        world.insert(e1, RealName::new("C"));
        world.insert(e1, RealTransform::from_position(Vec3::new(3.0, 0.0, 0.0)));

        let e2 = world.spawn();
        world.insert(e2, RealName::new("A"));
        world.insert(e2, RealTransform::from_position(Vec3::new(1.0, 0.0, 0.0)));

        let e3 = world.spawn();
        world.insert(e3, RealName::new("B"));
        world.insert(e3, RealTransform::from_position(Vec3::new(2.0, 0.0, 0.0)));

        // position.xで昇順ソート
        let query = QueryDescriptor::new()
            .select(["name", "position.x"])
            .with(["Transform"])
            .order_by(OrderBy::asc("position.x"));

        let result = world.execute_query(&query);
        assert_eq!(result.len(), 3);
        assert_eq!(result.rows[0].get_field("name"), Some(&serde_json::json!("A")));
        assert_eq!(result.rows[1].get_field("name"), Some(&serde_json::json!("B")));
        assert_eq!(result.rows[2].get_field("name"), Some(&serde_json::json!("C")));
    }

    #[test]
    fn test_execute_query_limit() {
        let mut world = World::new();

        for i in 0..10 {
            let e = world.spawn();
            world.insert(e, RealName::new(format!("Entity{}", i)));
            world.insert(e, RealTransform::identity());
        }

        // 上位3件のみ取得
        let query = QueryDescriptor::new()
            .select(["name"])
            .with(["Transform"])
            .limit(3);

        let result = world.execute_query(&query);
        assert_eq!(result.len(), 3);
        assert_eq!(result.total_count, 10); // limit前の総数
    }
}
