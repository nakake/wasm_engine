use super::entity::EntityId;
use super::component::Component;

/// SparseSetベースのコンポーネントストレージ
/// - dense: 実際のデータ配列（連続メモリでキャッシュ効率が良い）
/// - sparse: EntityId.index -> denseのインデックスへのマッピング
/// - entities: denseと対応するEntityIdの配列（イテレーション用）
pub struct ComponentStorage<T: Component> {
    dense: Vec<T>,
    entities: Vec<EntityId>,
    sparse: Vec<Option<usize>>,
}

impl<T: Component> ComponentStorage<T> {
    /// 新しいストレージを作成
    pub fn new() -> Self {
        Self {
            dense: Vec::new(),
            entities: Vec::new(),
            sparse: Vec::new(),
        }
    }

    /// コンポーネントを挿入（既存の場合は上書き）
    pub fn insert(&mut self, entity: EntityId, component: T) {
        let index = entity.index() as usize;

        // sparse配列を必要に応じて拡張
        if index >= self.sparse.len() {
            self.sparse.resize(index + 1, None);
        }

        if let Some(dense_index) = self.sparse[index] {
            // 既存のコンポーネントを上書き
            self.dense[dense_index] = component;
        } else {
            // 新規追加
            let dense_index = self.dense.len();
            self.dense.push(component);
            self.entities.push(entity);
            self.sparse[index] = Some(dense_index);
        }
    }

    /// コンポーネントを取得（不変参照）
    pub fn get(&self, entity: EntityId) -> Option<&T> {
        let index = entity.index() as usize;
        self.sparse
            .get(index)
            .and_then(|opt| *opt)
            .map(|dense_index| &self.dense[dense_index])
    }

    /// コンポーネントを取得（可変参照）
    pub fn get_mut(&mut self, entity: EntityId) -> Option<&mut T> {
        let index = entity.index() as usize;
        self.sparse
            .get(index)
            .and_then(|opt| *opt)
            .map(|dense_index| &mut self.dense[dense_index])
    }

    /// コンポーネントを削除
    pub fn remove(&mut self, entity: EntityId) -> Option<T> {
        let index = entity.index() as usize;

        if index >= self.sparse.len() {
            return None;
        }

        let dense_index = self.sparse[index].take()?;

        // 最後の要素と交換して削除（O(1)削除）
        let last_index = self.dense.len() - 1;
        if dense_index != last_index {
            // 最後の要素のsparseを更新
            let last_entity = self.entities[last_index];
            self.sparse[last_entity.index() as usize] = Some(dense_index);

            // swap_removeで効率的に削除
            self.entities.swap_remove(dense_index);
            return Some(self.dense.swap_remove(dense_index));
        }

        // 最後の要素の場合は単純にpop
        self.entities.pop();
        self.dense.pop()
    }

    /// 全コンポーネントをイテレート
    pub fn iter(&self) -> impl Iterator<Item = (EntityId, &T)> {
        self.entities.iter().copied().zip(self.dense.iter())
    }

    /// 全コンポーネントを可変イテレート
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (EntityId, &mut T)> {
        self.entities.iter().copied().zip(self.dense.iter_mut())
    }

    /// ストレージ内のコンポーネント数
    pub fn len(&self) -> usize {
        self.dense.len()
    }

    /// ストレージが空かどうか
    pub fn is_empty(&self) -> bool {
        self.dense.is_empty()
    }

    /// 指定Entityがコンポーネントを持つか
    pub fn contains(&self, entity: EntityId) -> bool {
        let index = entity.index() as usize;
        self.sparse.get(index).is_some_and(|opt| opt.is_some())
    }
}

impl<T: Component> Default for ComponentStorage<T> {
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

    #[test]
    fn test_insert_and_get() {
        let mut storage = ComponentStorage::new();
        let entity = EntityId::new(0, 1);
        let pos = Position { x: 1.0, y: 2.0 };

        storage.insert(entity, pos.clone());

        assert_eq!(storage.get(entity), Some(&pos));
        assert_eq!(storage.len(), 1);
    }

    #[test]
    fn test_insert_overwrites() {
        let mut storage = ComponentStorage::new();
        let entity = EntityId::new(0, 1);

        storage.insert(entity, Position { x: 1.0, y: 2.0 });
        storage.insert(entity, Position { x: 3.0, y: 4.0 });

        assert_eq!(storage.get(entity), Some(&Position { x: 3.0, y: 4.0 }));
        assert_eq!(storage.len(), 1);
    }

    #[test]
    fn test_get_mut() {
        let mut storage = ComponentStorage::new();
        let entity = EntityId::new(0, 1);
        storage.insert(entity, Position { x: 1.0, y: 2.0 });

        if let Some(pos) = storage.get_mut(entity) {
            pos.x = 10.0;
        }

        assert_eq!(storage.get(entity), Some(&Position { x: 10.0, y: 2.0 }));
    }

    #[test]
    fn test_remove() {
        let mut storage = ComponentStorage::new();
        let e1 = EntityId::new(0, 1);
        let e2 = EntityId::new(1, 1);
        let e3 = EntityId::new(2, 1);

        storage.insert(e1, Position { x: 1.0, y: 1.0 });
        storage.insert(e2, Position { x: 2.0, y: 2.0 });
        storage.insert(e3, Position { x: 3.0, y: 3.0 });

        // 中間要素を削除
        let removed = storage.remove(e2);
        assert_eq!(removed, Some(Position { x: 2.0, y: 2.0 }));
        assert_eq!(storage.len(), 2);

        // 残りの要素が正しくアクセスできる
        assert_eq!(storage.get(e1), Some(&Position { x: 1.0, y: 1.0 }));
        assert_eq!(storage.get(e2), None);
        assert_eq!(storage.get(e3), Some(&Position { x: 3.0, y: 3.0 }));
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut storage: ComponentStorage<Position> = ComponentStorage::new();
        let entity = EntityId::new(0, 1);
        assert_eq!(storage.remove(entity), None);
    }

    #[test]
    fn test_iter() {
        let mut storage = ComponentStorage::new();
        let e1 = EntityId::new(0, 1);
        let e2 = EntityId::new(1, 1);

        storage.insert(e1, Position { x: 1.0, y: 1.0 });
        storage.insert(e2, Position { x: 2.0, y: 2.0 });

        let items: Vec<_> = storage.iter().collect();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_contains() {
        let mut storage = ComponentStorage::new();
        let e1 = EntityId::new(0, 1);
        let e2 = EntityId::new(1, 1);

        storage.insert(e1, Position { x: 1.0, y: 1.0 });

        assert!(storage.contains(e1));
        assert!(!storage.contains(e2));
    }

    #[test]
    fn test_sparse_index_gap() {
        let mut storage = ComponentStorage::new();
        // index 100に直接挿入（sparse配列が自動拡張される）
        let entity = EntityId::new(100, 1);
        storage.insert(entity, Position { x: 1.0, y: 1.0 });

        assert_eq!(storage.get(entity), Some(&Position { x: 1.0, y: 1.0 }));
        assert_eq!(storage.len(), 1);
    }
}
